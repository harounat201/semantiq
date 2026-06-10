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
