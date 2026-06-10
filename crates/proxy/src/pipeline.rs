use crate::state::AppState;
use hex::encode as hex_encode;
use semantiq_embedding::preprocess;
use semantiq_monitoring::metrics::{emit, RequestMetrics};
use semantiq_types::{AdmissionDecision, CacheResult, EmbeddingVector, Query};
use sha2::{Digest, Sha256};
use std::time::{Duration, Instant};
use tracing::instrument;

#[instrument(skip(state))]
pub async fn run(raw: &str, state: &AppState) -> String {
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

    let vec = match state.embedder.embed(&normalized).await {
        Ok(v) => v,
        Err(e) => {
            tracing::error!(error = %e, "embedding failed, falling back to LLM");
            return llm_fallback(raw, &query, state, start).await;
        }
    };

    let cache_result = match state
        .vector_store
        .search(&vec, state.config.similarity_threshold)
        .await
    {
        Ok(Some(hit)) => {
            tracing::debug!(distance = hit.distance, "vector hit");
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

        CacheResult::VectorHitKvMiss { entry } => {
            // pgvector row exists but Redis key is gone — prune the stale entry
            let vs = state.vector_store.clone();
            let stale_id = entry.id;
            tokio::spawn(async move {
                if let Err(e) = vs.delete(stale_id).await {
                    tracing::warn!(error = %e, id = %stale_id, "failed to delete stale vector entry");
                }
            });
            call_llm_and_admit(raw, &query, &vec, state, start, true).await
        }

        CacheResult::Miss => call_llm_and_admit(raw, &query, &vec, state, start, false).await,
    }
}

async fn call_llm_and_admit(
    raw: &str,
    query: &Query,
    vec: &EmbeddingVector,
    state: &AppState,
    start: Instant,
    vector_hit: bool,
) -> String {
    let llm_resp = match state.llm.complete(raw).await {
        Ok(r) => r,
        Err(e) => {
            tracing::error!(error = %e, "LLM call failed");
            return "upstream error".to_string();
        }
    };

    let response = llm_resp.content.clone();
    let latency_ms = start.elapsed().as_millis() as u64;

    let state_clone = state.clone();
    let query_clone = query.clone();
    let vec_clone = vec.clone();
    let ttl = Duration::from_secs(state.config.cache_ttl_secs);

    tokio::spawn(async move {
        let decision = state_clone.admission.evaluate(&query_clone, &llm_resp).await;
        let admitted = decision == AdmissionDecision::Accept;

        emit(&RequestMetrics {
            query_hash: query_clone.hash.clone(),
            vector_hit,
            kv_hit: false,
            admitted,
            latency_ms,
        });

        if admitted {
            let redis_key = format!("semantiq:resp:{}", query_clone.hash);
            let write_ok = state_clone
                .kv_store
                .set(&redis_key, &llm_resp.content, ttl)
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

    response
}

async fn llm_fallback(raw: &str, query: &Query, state: &AppState, start: Instant) -> String {
    let resp = state.llm.complete(raw).await.unwrap_or_else(|_| semantiq_types::LlmResponse {
        content: "upstream error".to_string(),
        status_ok: false,
        latency_ms: 0,
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
