pub mod metrics;

use anyhow::Result;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Initialize JSON structured logging. Call once at startup.
pub fn init_tracing() -> Result<()> {
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with(fmt::layer().json())
        .init();
    Ok(())
}
