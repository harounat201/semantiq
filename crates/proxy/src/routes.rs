use crate::{llm::LlmClient, pipeline, state::AppState};
use axum::{
    body::Body,
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct QueryRequest {
    pub query: String,
}

#[derive(Serialize)]
pub struct QueryResponse {
    pub response: String,
}

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/query", post(query_handler))
        .route("/health", get(health_handler))
}

pub async fn auth_middleware(
    State(state): State<AppState>,
    headers: HeaderMap,
    request: Request<Body>,
    next: Next,
) -> Response {
    // /health is exempt from auth
    if request.uri().path() == "/health" {
        return next.run(request).await;
    }

    let provided = headers
        .get("x-api-key")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if provided != state.config.api_key {
        return (StatusCode::UNAUTHORIZED, "invalid api key").into_response();
    }

    next.run(request).await
}

async fn query_handler(
    State(state): State<AppState>,
    Json(body): Json<QueryRequest>,
) -> impl IntoResponse {
    let llm = LlmClient::new(&state.config.openai_api_key);
    let response = pipeline::run(&body.query, &state, &llm).await;
    Json(QueryResponse { response })
}

async fn health_handler(State(state): State<AppState>) -> impl IntoResponse {
    let pg_ok = sqlx::query("SELECT 1").execute(&state.pool).await.is_ok();

    if pg_ok {
        (StatusCode::OK, Json(HealthResponse { status: "ok" })).into_response()
    } else {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(HealthResponse { status: "degraded" }),
        )
            .into_response()
    }
}
