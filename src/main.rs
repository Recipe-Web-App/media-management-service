use media_management_service::auth::AuthMode;
use media_management_service::config::Config;
use media_management_service::db;
use media_management_service::routes;
use media_management_service::state::AppState;
use media_management_service::storage::Storage;
use media_management_service::telemetry;

#[tokio::main]
async fn main() {
    let config = Config::from_env();
    let otel_guard = telemetry::init(&config);

    let db_pool = db::connect(&config)
        .await
        .expect("failed to connect to database");
    tracing::info!("database connection pool established");

    let storage = Storage::new(&config.storage_base_path, &config.storage_temp_path)
        .await
        .expect("failed to initialise storage");
    tracing::info!("storage initialised");

    let auth_mode = AuthMode::from_config(&config.auth);
    tracing::info!("auth mode: {}", auth_mode.name());

    let state = AppState {
        db_pool,
        storage,
        config: config.clone(),
        auth_mode,
    };
    let app = routes::router(state);

    let addr = format!("{}:{}", config.host, config.port);
    tracing::info!("listening on {addr}");

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("failed to bind listener");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .expect("server error");

    tracing::info!("shutting down telemetry");
    otel_guard.shutdown();
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("failed to listen for ctrl-c");
    tracing::info!("shutdown signal received");
}
