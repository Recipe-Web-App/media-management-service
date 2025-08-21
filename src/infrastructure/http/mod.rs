use axum::{
    extract::DefaultBodyLimit,
    http::{header, Method, StatusCode},
    response::Json,
    routing::get,
    Router,
};
use serde_json::{json, Value};
use std::time::Duration;
use tower::ServiceBuilder;
use tower_http::{
    compression::CompressionLayer,
    cors::CorsLayer,
    request_id::{MakeRequestUuid, SetRequestIdLayer},
    timeout::TimeoutLayer,
    trace::TraceLayer,
};
use tracing::info;

use crate::infrastructure::{config::AppConfig, persistence::Database};
use crate::presentation::routes;

/// Create the main application router
pub fn create_app(config: &AppConfig, database: Option<Database>) -> Router {
    let middleware_stack = ServiceBuilder::new()
        .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
        .layer(TraceLayer::new_for_http())
        .layer(CompressionLayer::new())
        .layer(TimeoutLayer::new(Duration::from_secs(30)))
        .layer(create_cors_layer())
        .layer(DefaultBodyLimit::max(
            usize::try_from(config.server.max_upload_size).unwrap_or(100_000_000),
        ));

    let mut router = Router::new()
        .route("/health", get(health_check))
        .merge(routes::create_routes())
        .layer(middleware_stack)
        .fallback(not_found_handler);

    // Add readiness check with database if available
    if let Some(db) = database {
        router = router.route("/ready", get(move || readiness_check_with_db(db)));
    } else {
        router = router.route("/ready", get(readiness_check_no_db));
    }

    router
}

/// Health check endpoint for Kubernetes liveness probe
async fn health_check() -> Json<Value> {
    Json(json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "service": "media-management-service"
    }))
}

/// Readiness check with database health verification
async fn readiness_check_with_db(database: Database) -> Json<Value> {
    let mut checks = serde_json::Map::new();
    let mut overall_status = "ready";

    // Check database health
    match database.health_check().await {
        Ok(()) => {
            checks.insert("database".to_string(), json!("ok"));
        }
        Err(e) => {
            checks.insert("database".to_string(), json!(format!("error: {}", e)));
            overall_status = "not_ready";
        }
    }

    // Storage check (placeholder - will be implemented when storage is added)
    checks.insert("storage".to_string(), json!("ok"));

    Json(json!({
        "status": overall_status,
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "checks": checks
    }))
}

/// Readiness check without database (fallback)
async fn readiness_check_no_db() -> Json<Value> {
    Json(json!({
        "status": "ready",
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "checks": {
            "database": "not_configured",
            "storage": "ok"
        }
    }))
}

/// Handler for 404 not found
async fn not_found_handler() -> (StatusCode, Json<Value>) {
    (
        StatusCode::NOT_FOUND,
        Json(json!({
            "error": "Not Found",
            "message": "The requested resource was not found"
        })),
    )
}

/// Create CORS layer with appropriate settings
fn create_cors_layer() -> CorsLayer {
    CorsLayer::new()
        .allow_origin(tower_http::cors::Any) // TODO: Configure specific origins in production
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
        .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE, header::ACCEPT])
        .max_age(Duration::from_secs(3600))
}

/// Start the HTTP server
///
/// # Errors
/// Returns an error if the server fails to start
pub async fn start_server(config: AppConfig) -> Result<(), Box<dyn std::error::Error>> {
    // Try to initialize database connection
    let database = match Database::new(&config.database).await {
        Ok(db) => Some(db),
        Err(e) => {
            tracing::warn!("Failed to connect to database: {}", e);
            tracing::info!("Starting server without database connection");
            None
        }
    };

    let app = create_app(&config, database);
    let addr = config.server.socket_addr();

    info!("Starting server on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
