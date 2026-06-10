mod llm;
mod pipeline;
mod routes;
mod state;

use anyhow::Result;
use axum::{middleware, Router};
use semantiq_config::Config;
use semantiq_monitoring::init_tracing;
use sqlx::postgres::PgPoolOptions;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing()?;
    let cfg = Config::from_env()?;

    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&cfg.database_url)
        .await?;

    sqlx::migrate!("../../migrations").run(&pool).await?;

    let app_state = state::AppState::new(cfg.clone(), pool)?;

    // background TTL pruning — runs once per day
    let vector_store = app_state.vector_store.clone();
    let prune_days = (cfg.cache_ttl_secs / 86_400).max(1) as i64;
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(24 * 60 * 60));
        loop {
            interval.tick().await;
            match vector_store.prune_older_than(prune_days).await {
                Ok(n) => tracing::info!(pruned = n, "pruned stale vector entries"),
                Err(e) => tracing::warn!(error = %e, "vector pruning failed"),
            }
        }
    });

    let app = Router::new()
        .merge(routes::router())
        .layer(middleware::from_fn_with_state(
            app_state.clone(),
            routes::auth_middleware,
        ))
        .with_state(app_state);

    let addr: SocketAddr = format!("0.0.0.0:{}", cfg.port).parse()?;
    tracing::info!("SemantiQ listening on {addr}");

    let listener = TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
