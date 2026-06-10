use crate::{llm::LlmClient, state::AppState};
use hex::encode as hex_encode;
use semantiq_cache::KvStore;
use semantiq_embedding::preprocess;
use semantiq_monitoring::metrics::{emit, RequestMetrics};
use semantiq_types::{AdmissionDecision, CacheResult, Query};
use sha2::{Digest, Sha256};
use std::time::{Duration, Instant};
use tracing::instrument;

/// Run the full SemantiQ pipeline for a raw query string.
/// Returns the response text to send back to the caller.
#[instrument(skip(state, llm))]
pub async fn run(raw: &str, state: &AppState, llm: &LlmClient) -> String {
    let start = Instant::now();

    let normalized = preprocess::normalize(raw);
    let hash = {
        let mut h = Sha256::new();
        h.update(&normalized);
        hex_encode(h.finalize())
    };
    let query = Query {
        raw: raw.to_string(),
        normalized: normalized.clone(),
        hash: hash.clone(),
    };

    // --- Phase 1: embed ---
    let vec = match state.embedder.embed(&normalized).await {
        Ok(v) => v,
        Err(e) => {
            tracing::error!(error = %e, "embedding failed, falling back to LLM");
            return llm_fallback(llm, raw, &query, state, start).await;
        }
    };

    // --- Phase 2: vector search ---
    let cache_result = match state
        .vector_store
        .search(&vec, state.config.similarity_threshold)
        .await
    {
        Ok(Some(hit)) => {
            tracing::debug!(distance = hit.distance, "vector hit");
            // Phase 3: Redis lookup
            match state.kv_store.get(&hit.entry.redis_key).await {
                Ok(Some(cached)) => CacheResult::KvHit { response: cached },
                Ok(None) => CacheResult::VectorHitKvMiss { entry: hit.entry },
                Err(e) => {
                    tracing::warn!(error = %e, "Redis GET failed");
                    CacheResult::Miss
                }
            }
        }
        Ok(None) => CacheResult::Miss,
        Err(e) => {
            tracing::warn!(error = %e, "vector search failed");
            CacheResult::Miss
        }
    };

    match cache_result {
        CacheResult::KvHit { response } => {
            emit(&RequestMetrics {
                query_hash: hash,
                vector_hit: true,
                kv_hit: true,
                admitted: false,
                latency_ms: start.elapsed().as_millis() as u64,
            });
            response
        }

        CacheResult::VectorHitKvMiss { .. } | CacheResult::Miss => {
            // --- Phase 5: LLM call ---
            let llm_resp = match llm.complete(raw).await {
                Ok(r) => r,
                Err(e) => {
                    tracing::error!(error = %e, "LLM call failed");
                    return "upstream error".to_string();
                }
            };

            let response = llm_resp.content.clone();

            // --- Phase 4: async admission + write-back ---
            let state_clone = state.clone();
            let llm_resp_clone = llm_resp;
            let query_clone = query;
            let vec_clone = vec;
            let ttl = Duration::from_secs(state.config.cache_ttl_secs);

            tokio::spawn(async move {
                let decision = state_clone
                    .admission
                    .evaluate(&query_clone, &llm_resp_clone)
                    .await;

                if decision == AdmissionDecision::Accept {
                    let redis_key = format!("semantiq:resp:{}", query_clone.hash);

                    let write_ok = state_clone
                        .kv_store
                        .set(&redis_key, &llm_resp_clone.content, ttl)
                        .await
                        .is_ok();

                    if write_ok {
                        if let Err(e) = state_clone
                            .vector_store
                            .insert(&query_clone.hash, &redis_key, &vec_clone)
                            .await
                        {
                            tracing::warn!(error = %e, "pgvector insert failed");
                        } else {
                            tracing::info!(hash = %query_clone.hash, "cache entry admitted");
                        }
                    }
                }
            });

            emit(&RequestMetrics {
                query_hash: hash,
                vector_hit: matches!(cache_result, CacheResult::VectorHitKvMiss { .. }),
                kv_hit: false,
                admitted: false, // actual decision happens async
                latency_ms: start.elapsed().as_millis() as u64,
            });

            response
        }
    }
}

async fn llm_fallback(
    llm: &LlmClient,
    raw: &str,
    query: &Query,
    _state: &AppState,
    start: Instant,
) -> String {
    let resp = llm.complete(raw).await.unwrap_or_else(|_| {
        semantiq_types::LlmResponse {
            content: "upstream error".to_string(),
            status_ok: false,
            latency_ms: 0,
        }
    });

    emit(&RequestMetrics {
        query_hash: query.hash.clone(),
        vector_hit: false,
        kv_hit: false,
        admitted: false,
        latency_ms: start.elapsed().as_millis() as u64,
    });

    resp.content
}
