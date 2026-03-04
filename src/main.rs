use media_management_service::config::{Config, RunMode};
use media_management_service::routes;
use tracing_subscriber::EnvFilter;

fn init_tracing(config: &Config) {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "media_management_service=info,tower_http=info".into());

    if config.run_mode == RunMode::Production {
        tracing_subscriber::fmt()
            .json()
            .with_env_filter(filter)
            .init();
    } else {
        tracing_subscriber::fmt()
            .pretty()
            .with_env_filter(filter)
            .init();
    }
}

#[tokio::main]
async fn main() {
    let config = Config::from_env();
    init_tracing(&config);

    let app = routes::router();

    let addr = format!("{}:{}", config.host, config.port);
    tracing::info!("listening on {addr}");

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("failed to bind listener");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .expect("server error");
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("failed to listen for ctrl-c");
    tracing::info!("shutdown signal received");
}
