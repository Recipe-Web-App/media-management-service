#![deny(clippy::all)]
#![deny(clippy::pedantic)]
#![deny(warnings)]

use media_management_service::infrastructure::{config::AppConfig, http::start_server};
use tracing::{error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load environment variables from .env file
    dotenvy::dotenv().ok();

    // Initialize logging
    init_tracing();

    // Load configuration
    let config = AppConfig::from_env().map_err(|e| {
        error!("Failed to load configuration: {}", e);
        e
    })?;

    info!("Starting Media Management Service");
    info!("Configuration loaded: server will bind to {}", config.server.socket_addr());

    // Start the HTTP server
    if let Err(e) = start_server(config).await {
        error!("Server error: {}", e);
        return Err(e);
    }

    Ok(())
}

/// Initialize structured logging
fn init_tracing() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "media_management_service=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer().json())
        .init();
}
