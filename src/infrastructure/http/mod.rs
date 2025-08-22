use axum::{
    extract::DefaultBodyLimit,
    http::{header, Method, StatusCode},
    response::Json,
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
pub fn create_app(config: &AppConfig, _database: Option<Database>) -> Router {
    let middleware_stack = ServiceBuilder::new()
        .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
        .layer(TraceLayer::new_for_http())
        .layer(CompressionLayer::new())
        .layer(TimeoutLayer::new(Duration::from_secs(30)))
        .layer(create_cors_layer())
        .layer(DefaultBodyLimit::max(
            usize::try_from(config.server.max_upload_size).unwrap_or(100_000_000),
        ));

    Router::new().merge(routes::create_routes()).layer(middleware_stack).fallback(not_found_handler)
}

/// Health check endpoint for Kubernetes liveness probe
pub async fn health_check() -> Json<Value> {
    Json(json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "service": "media-management-service"
    }))
}

/// Readiness check without database (fallback)
pub async fn readiness_check_no_db() -> Json<Value> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::config::{
        DatabaseConfig, LoggingConfig, ServerConfig, StorageConfig,
    };
    use axum::{body::Body, http::Request};
    use tower::ServiceExt;

    fn create_test_config() -> AppConfig {
        AppConfig {
            server: ServerConfig {
                host: "127.0.0.1".to_string(),
                port: 0, // Use port 0 for testing to avoid conflicts
                max_upload_size: 1_000_000,
            },
            database: DatabaseConfig {
                url: "postgres://test:test@localhost:5432/test".to_string(),
                max_connections: 5,
                min_connections: 1,
                acquire_timeout_seconds: 10,
                host: "localhost".to_string(),
                port: 5432,
                database: "test".to_string(),
                schema: "public".to_string(),
                user: "test".to_string(),
                password: "test".to_string(),
            },
            storage: StorageConfig {
                base_path: "/tmp/test".to_string(),
                temp_path: "/tmp/test/temp".to_string(),
                max_file_size: 10_000_000,
            },
            logging: LoggingConfig { level: "info".to_string(), format: "json".to_string() },
        }
    }

    #[tokio::test]
    async fn test_create_app_without_database() {
        let config = create_test_config();
        let app = create_app(&config, None);

        // Test health check endpoint
        let request = Request::builder().uri("/health").body(Body::empty()).unwrap();

        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_health_check_endpoint() {
        let response = health_check().await;
        let json_value = response.0;

        assert!(json_value.get("status").is_some());
        assert!(json_value.get("timestamp").is_some());
        assert!(json_value.get("service").is_some());
        assert_eq!(json_value["status"], "healthy");
        assert_eq!(json_value["service"], "media-management-service");
    }

    #[tokio::test]
    async fn test_readiness_check_no_db() {
        let response = readiness_check_no_db().await;
        let json_value = response.0;

        assert_eq!(json_value["status"], "ready");
        assert!(json_value.get("timestamp").is_some());
        assert!(json_value.get("checks").is_some());

        let checks = &json_value["checks"];
        assert_eq!(checks["database"], "not_configured");
        assert_eq!(checks["storage"], "ok");
    }

    #[tokio::test]
    async fn test_not_found_handler() {
        let (status, json_response) = not_found_handler().await;

        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(json_response["error"], "Not Found");
        assert_eq!(json_response["message"], "The requested resource was not found");
    }

    #[tokio::test]
    async fn test_app_routes_structure() {
        let config = create_test_config();
        let app = create_app(&config, None);

        // Test that routes are properly mounted
        let health_request = Request::builder().uri("/health").body(Body::empty()).unwrap();

        let health_response = app.clone().oneshot(health_request).await.unwrap();
        assert_eq!(health_response.status(), StatusCode::OK);

        // Test readiness endpoint
        let ready_request = Request::builder().uri("/ready").body(Body::empty()).unwrap();

        let ready_response = app.clone().oneshot(ready_request).await.unwrap();
        assert_eq!(ready_response.status(), StatusCode::OK);

        // Test 404 for non-existent route
        let not_found_request =
            Request::builder().uri("/non-existent-route").body(Body::empty()).unwrap();

        let not_found_response = app.oneshot(not_found_request).await.unwrap();
        assert_eq!(not_found_response.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn test_create_cors_layer() {
        // Test that CORS layer can be created without panicking
        let cors_layer = create_cors_layer();

        // We can't easily inspect the CORS layer's internal configuration
        // but we can ensure it was created successfully
        drop(cors_layer); // This verifies the layer was created
    }

    #[tokio::test]
    async fn test_app_middleware_stack() {
        let config = create_test_config();
        let app = create_app(&config, None);

        // Test that middleware is properly applied by making a request
        // and checking that it completes successfully
        let request = Request::builder()
            .uri("/health")
            .header("Content-Type", "application/json")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        // Verify the response indicates middleware processed the request
        assert_eq!(response.status(), StatusCode::OK);

        // Check that compression and other middleware headers might be present
        // (actual header presence depends on response content and middleware config)
        let headers = response.headers();
        assert!(headers.get("content-type").is_some());
    }

    #[test]
    fn test_max_upload_size_conversion() {
        let config = create_test_config();

        // Test that max_upload_size conversion works
        let size = usize::try_from(config.server.max_upload_size).unwrap_or(100_000_000);
        assert_eq!(size, 1_000_000);

        // Test with a very large value that might overflow
        let large_config = AppConfig {
            server: ServerConfig {
                host: "127.0.0.1".to_string(),
                port: 8080,
                max_upload_size: u64::MAX,
            },
            database: config.database,
            storage: config.storage,
            logging: config.logging,
        };

        let large_size =
            usize::try_from(large_config.server.max_upload_size).unwrap_or(100_000_000);
        assert!(large_size > 0);
    }

    // Note: Testing start_server is complex because it:
    // 1. Binds to network ports
    // 2. Runs indefinitely
    // 3. Requires database connections
    //
    // Integration tests or mocked tests would be more appropriate for testing start_server
}
