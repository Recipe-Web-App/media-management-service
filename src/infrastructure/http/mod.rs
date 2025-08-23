use axum::http::HeaderValue;
use axum::{
    extract::DefaultBodyLimit,
    http::{header, Method},
    response::Json,
    Router,
};
use serde_json::{json, Value};
use std::time::Duration;
use tower::ServiceBuilder;
use tower_http::{
    compression::CompressionLayer,
    cors::CorsLayer,
    request_id::{MakeRequestId, RequestId, SetRequestIdLayer},
    timeout::TimeoutLayer,
    trace::TraceLayer,
};
use tracing::info;
use uuid::Uuid;

use crate::{
    infrastructure::{config::AppConfig, persistence::Database},
    presentation::{
        middleware::{error::global_error_handler, AppError},
        routes,
    },
};

/// Enhanced request ID maker that creates UUID v4 request IDs
#[derive(Clone, Debug)]
pub struct EnhancedRequestId;

impl MakeRequestId for EnhancedRequestId {
    fn make_request_id<B>(&mut self, _request: &axum::extract::Request<B>) -> Option<RequestId> {
        let request_id = Uuid::new_v4().to_string();
        let header_value = HeaderValue::try_from(request_id).ok()?;
        Some(RequestId::new(header_value))
    }
}

/// Create the main application router
pub fn create_app(config: &AppConfig, _database: Option<Database>) -> Router {
    let middleware_stack = ServiceBuilder::new()
        .layer(SetRequestIdLayer::new(
            header::HeaderName::from_static("x-request-id"),
            EnhancedRequestId,
        ))
        .layer(axum::middleware::from_fn(global_error_handler))
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
async fn not_found_handler() -> Result<Json<Value>, AppError> {
    Err(AppError::NotFound { resource: "The requested resource was not found".to_string() })
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
    info!("Middleware configuration:");
    info!("  - Request ID: enabled");
    info!("  - Global Error Handler: enabled");
    info!("  - Authentication: {}", config.middleware.auth.enabled);
    info!("  - Rate Limiting: {}", config.middleware.rate_limiting.enabled);
    info!("  - Security Headers: {}", config.middleware.security.enabled);
    info!("  - Request Validation: {}", config.middleware.validation.enabled);
    info!("  - Metrics Collection: {}", config.middleware.metrics.enabled);
    info!("  - Request Logging: {}", config.middleware.request_logging.enabled);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::config::{
        AuthConfig, DatabaseConfig, LoggingConfig, MetricsConfig, MiddlewareConfig,
        RateLimitTiersConfig, RateLimitingConfig, RequestLoggingConfig, RuntimeMode,
        SecurityConfig, SecurityFeatures, ServerConfig, StorageConfig, ValidationConfig,
    };
    use axum::{body::Body, http::Request};
    use tower::ServiceExt;

    #[allow(clippy::too_many_lines)]
    fn create_test_config() -> AppConfig {
        AppConfig {
            mode: RuntimeMode::Local,
            server: ServerConfig {
                host: "127.0.0.1".to_string(),
                port: 0,
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
            logging: LoggingConfig {
                level: "info".to_string(),
                filter: None,
                console_enabled: true,
                console_format: crate::infrastructure::config::LogFormat::Json,
                file_enabled: false,
                file_format: crate::infrastructure::config::LogFormat::Json,
                file_path: "/tmp/test/logs".to_string(),
                file_prefix: "test".to_string(),
                file_rotation: crate::infrastructure::config::RotationPolicy::Daily,
                file_retention_days: 1,
                file_max_size_mb: None,
                non_blocking: false,
                buffer_size: None,
            },
            middleware: MiddlewareConfig {
                auth: AuthConfig {
                    enabled: false,
                    jwt_secret: "test-secret".to_string(),
                    jwt_expiry_hours: 24,
                    require_auth_routes: vec![],
                    optional_auth_routes: vec![],
                },
                rate_limiting: RateLimitingConfig {
                    enabled: false,
                    default_requests_per_minute: 100,
                    default_burst_capacity: 10,
                    trust_forwarded_headers: false,
                    include_rate_limit_headers: true,
                    tiers: RateLimitTiersConfig {
                        health_requests_per_minute: 1000,
                        public_requests_per_minute: 60,
                        authenticated_requests_per_minute: 200,
                        upload_requests_per_minute: 10,
                        admin_requests_per_minute: 500,
                    },
                },
                security: SecurityConfig {
                    enabled: true,
                    features: SecurityFeatures {
                        hsts: false,
                        hsts_subdomains: true,
                        hsts_preload: false,
                        content_type_options: true,
                    },
                    hsts_max_age_seconds: 31_536_000,
                    csp_policy: Some("default-src 'self'".to_string()),
                    frame_options: "DENY".to_string(),
                    xss_protection: "1; mode=block".to_string(),
                    referrer_policy: "strict-origin-when-cross-origin".to_string(),
                    permissions_policy: Some("camera=()".to_string()),
                },
                metrics: MetricsConfig {
                    enabled: false,
                    endpoint_enabled: false,
                    endpoint_path: "/metrics".to_string(),
                    prometheus_port: 9090,
                    collect_request_metrics: true,
                    collect_timing_metrics: true,
                    collect_error_metrics: true,
                    collect_business_metrics: true,
                    normalize_routes: true,
                    collection_interval_seconds: 10,
                },
                validation: ValidationConfig {
                    enabled: true,
                    validate_content_type: true,
                    validate_body_size: true,
                    max_body_size_mb: 100,
                    validate_json_structure: true,
                    validate_file_uploads: true,
                    max_file_size_mb: 50,
                    allowed_file_types: vec!["image/jpeg".to_string()],
                    validate_headers: false,
                    validate_methods: false,
                },
                request_logging: RequestLoggingConfig {
                    enabled: false,
                    log_request_body: false,
                    log_response_body: false,
                    max_body_size_kb: 10,
                    log_request_headers: false,
                    log_response_headers: false,
                    excluded_headers: vec!["authorization".to_string()],
                    log_timing: true,
                    slow_request_threshold_ms: 500,
                },
            },
        }
    }

    #[tokio::test]
    async fn test_create_app_without_database() {
        let config = create_test_config();
        let app = create_app(&config, None);

        let request =
            Request::builder().uri("/api/v1/media-management/health").body(Body::empty()).unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);
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
    async fn test_not_found_handler() {
        let result = not_found_handler().await;
        assert!(result.is_err());

        let error = result.unwrap_err();
        assert!(matches!(error, AppError::NotFound { .. }));
    }

    #[test]
    fn test_enhanced_request_id_make_request_id() {
        let mut maker = EnhancedRequestId;
        let request = Request::builder().body(Body::empty()).unwrap();

        let request_id = maker.make_request_id(&request);
        assert!(request_id.is_some());

        // Verify it's a valid UUID format
        let request_id_value = request_id.unwrap();
        let header_value = request_id_value.header_value();
        let id_str = header_value.to_str().unwrap();
        let parsed_uuid = Uuid::parse_str(id_str);
        assert!(parsed_uuid.is_ok());
    }
}
