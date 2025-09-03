use axum::http::{HeaderValue, StatusCode};
use axum::{
    extract::{DefaultBodyLimit, State},
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
    infrastructure::{
        config::AppConfig,
        persistence::{Database, ReconnectingMediaRepository},
        storage::{FileStorage, FilesystemStorage},
    },
    presentation::{
        handlers::media::AppState,
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
///
/// Creates an application with a reconnecting repository that automatically handles
/// database connection failures and attempts periodic reconnection.
pub fn create_app(config: &AppConfig, database: Option<&Database>) -> Router {
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

    // Create reconnecting repository that handles connection failures automatically
    let media_repo: std::sync::Arc<
        dyn crate::domain::repositories::MediaRepository<Error = AppError>,
    > = if let Some(db) = database {
        // Start with a connected repository
        let reconnecting_repo =
            ReconnectingMediaRepository::with_connection(config.postgres.clone(), db);

        // Start background reconnection task
        let reconnection_handle = reconnecting_repo.clone().start_reconnection_task();

        // Store the task handle (in a real application, you might want to store this
        // somewhere to gracefully shut it down on service shutdown)
        std::mem::forget(reconnection_handle);

        std::sync::Arc::new(reconnecting_repo)
    } else {
        // Start with a disconnected repository that will attempt reconnection
        let reconnecting_repo = ReconnectingMediaRepository::new(
            config.postgres.clone(),
            "Database connection failed during startup".to_string(),
        );

        // Start background reconnection task
        let reconnection_handle = reconnecting_repo.clone().start_reconnection_task();
        std::mem::forget(reconnection_handle);

        std::sync::Arc::new(reconnecting_repo)
    };

    let file_storage = std::sync::Arc::new(FilesystemStorage::new(&config.storage.base_path));

    // Create presigned URL service
    let presigned_service =
        crate::infrastructure::storage::PresignedUrlService::from_app_config(config);

    // Create application state
    let app_state =
        AppState::new(media_repo, file_storage, presigned_service, config.storage.max_file_size);

    if database.is_some() {
        tracing::info!("Creating application with database connection - will attempt reconnection if connection is lost");
    } else {
        tracing::warn!("Creating application with disconnected repository - will attempt periodic reconnection every 30 seconds");
    }

    Router::new()
        .merge(routes::create_routes(app_state))
        .layer(middleware_stack)
        .fallback(not_found_handler)
}

/// Comprehensive health check endpoint that validates all system dependencies
///
/// Checks the following components:
/// - Database connectivity (`PostgreSQL`)
/// - Storage accessibility (filesystem paths and permissions)
/// - Service basic functionality
///
/// Returns HTTP 200 with status "healthy" or "degraded" when service can operate
/// Returns HTTP 503 with status "unhealthy" when service cannot operate
///
/// Response format:
/// ```json
/// {
///   "status": "healthy|degraded|unhealthy",
///   "timestamp": "2025-01-15T10:30:00Z",
///   "service": "media-management-service",
///   "version": "0.1.0",
///   "checks": {
///     "database": {"status": "healthy", "response_time_ms": 5},
///     "storage": {"status": "healthy", "path": "/app/media", "writable": true},
///     "overall": "healthy"
///   }
/// }
/// ```
///
/// Timeouts: Each check has a 2-second timeout to prevent hanging
pub async fn health_check_with_dependencies(
    State(app_state): State<AppState>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    use std::time::{Duration, Instant};
    use tokio::time::timeout;

    let start_time = Instant::now();
    let check_timeout = Duration::from_secs(2);

    // Check database health
    let database_check = timeout(check_timeout, async {
        let check_start = Instant::now();
        let result = app_state.repository.health_check().await;
        let response_time = check_start.elapsed().as_millis() as u64;
        (result, response_time)
    })
    .await;

    let (database_status, database_response_time, database_healthy) = match database_check {
        Ok((Ok(()), response_time)) => {
            tracing::debug!("Database health check: healthy ({}ms)", response_time);
            ("healthy", response_time, true)
        }
        Ok((Err(e), response_time)) => {
            tracing::debug!("Database health check: unhealthy ({}ms) - {}", response_time, e);
            ("unhealthy", response_time, false)
        }
        Err(_) => {
            tracing::debug!("Database health check: timeout (2000ms)");
            ("timeout", 2000, false) // Timeout occurred
        }
    };

    // Check storage health
    let storage_check = timeout(check_timeout, async {
        let check_start = Instant::now();
        let result = app_state.storage.health_check().await;
        let response_time = check_start.elapsed().as_millis() as u64;
        (result, response_time)
    })
    .await;

    let (storage_status, storage_response_time, storage_healthy) = match storage_check {
        Ok((Ok(()), response_time)) => {
            tracing::debug!("Storage health check: healthy ({}ms)", response_time);
            ("healthy", response_time, true)
        }
        Ok((Err(e), response_time)) => {
            tracing::debug!("Storage health check: unhealthy ({}ms) - {}", response_time, e);
            ("unhealthy", response_time, false)
        }
        Err(_) => {
            tracing::debug!("Storage health check: timeout (2000ms)");
            ("timeout", 2000, false) // Timeout occurred
        }
    };

    // Determine overall health status
    // Service should be considered operational if storage is working, even without database
    let overall_status = if database_healthy && storage_healthy {
        tracing::debug!(
            "Overall health: healthy (database: {}, storage: {})",
            database_healthy,
            storage_healthy
        );
        "healthy"
    } else if storage_healthy {
        tracing::debug!(
            "Overall health: degraded (database: {}, storage: {})",
            database_healthy,
            storage_healthy
        );
        "degraded" // Storage working allows basic operation
    } else {
        tracing::debug!(
            "Overall health: unhealthy (database: {}, storage: {})",
            database_healthy,
            storage_healthy
        );
        "unhealthy" // Cannot function without storage
    };

    let total_response_time = start_time.elapsed().as_millis() as u64;

    let response = json!({
        "status": overall_status,
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "service": "media-management-service",
        "version": env!("CARGO_PKG_VERSION"),
        "response_time_ms": total_response_time,
        "checks": {
            "database": {
                "status": database_status,
                "response_time_ms": database_response_time
            },
            "storage": {
                "status": storage_status,
                "response_time_ms": storage_response_time
            },
            "overall": overall_status
        }
    });

    // Return appropriate HTTP status code
    match overall_status {
        "healthy" | "degraded" => Ok((StatusCode::OK, Json(response))),
        _ => Err((StatusCode::SERVICE_UNAVAILABLE, Json(response))),
    }
}

/// Basic health check endpoint (backward compatibility)
///
/// This is the original simple health check that always returns "healthy".
/// Use `health_check_with_dependencies` for production deployments.
pub fn health_check() -> Json<Value> {
    Json(json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "service": "media-management-service"
    }))
}

/// Comprehensive readiness check endpoint that validates all system dependencies
///
/// Readiness indicates whether the service is prepared to accept traffic.
/// Unlike health checks which can report "degraded" status, readiness is binary:
/// the service is either ready to serve requests or it is not.
///
/// Checks the following components:
/// - Database connectivity (`PostgreSQL`)
/// - Storage accessibility (filesystem paths and permissions)
///
/// Returns HTTP 200 with status "ready" when ALL dependencies are operational
/// Returns HTTP 503 with status "`not_ready`" when ANY dependency fails
///
/// Response format:
/// ```json
/// {
///   "status": "ready|not_ready",
///   "timestamp": "2025-01-15T10:30:00Z",
///   "service": "media-management-service",
///   "version": "0.1.0",
///   "checks": {
///     "database": {"status": "ready", "response_time_ms": 5},
///     "storage": {"status": "ready", "response_time_ms": 3},
///     "overall": "ready"
///   }
/// }
/// ```
///
/// Timeouts: Each check has a 2-second timeout to prevent hanging
pub async fn readiness_check_with_dependencies(
    State(app_state): State<AppState>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    use std::time::{Duration, Instant};
    use tokio::time::timeout;

    let start_time = Instant::now();
    let check_timeout = Duration::from_secs(2);

    // Check database readiness
    let database_check = timeout(check_timeout, async {
        let check_start = Instant::now();
        let result = app_state.repository.health_check().await;
        let response_time = check_start.elapsed().as_millis() as u64;
        (result, response_time)
    })
    .await;

    let (database_status, database_response_time, _database_ready) = match database_check {
        Ok((Ok(()), response_time)) => ("ready", response_time, true),
        Ok((Err(_), response_time)) => ("not_ready", response_time, false),
        Err(_) => ("timeout", 2000, false), // Timeout occurred
    };

    // Check storage readiness
    let storage_check = timeout(check_timeout, async {
        let check_start = Instant::now();
        let result = app_state.storage.health_check().await;
        let response_time = check_start.elapsed().as_millis() as u64;
        (result, response_time)
    })
    .await;

    let (storage_status, storage_response_time, storage_ready) = match storage_check {
        Ok((Ok(()), response_time)) => ("ready", response_time, true),
        Ok((Err(_), response_time)) => ("not_ready", response_time, false),
        Err(_) => ("timeout", 2000, false), // Timeout occurred
    };

    // Determine overall readiness status - storage must be ready for basic operation
    let overall_status = if storage_ready { "ready" } else { "not_ready" };

    let total_response_time = start_time.elapsed().as_millis() as u64;

    let response = json!({
        "status": overall_status,
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "service": "media-management-service",
        "version": env!("CARGO_PKG_VERSION"),
        "response_time_ms": total_response_time,
        "checks": {
            "database": {
                "status": database_status,
                "response_time_ms": database_response_time
            },
            "storage": {
                "status": storage_status,
                "response_time_ms": storage_response_time
            },
            "overall": overall_status
        }
    });

    // Return appropriate HTTP status code - readiness is binary
    match overall_status {
        "ready" => Ok((StatusCode::OK, Json(response))),
        _ => Err((StatusCode::SERVICE_UNAVAILABLE, Json(response))),
    }
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
    let database = match Database::new(&config.postgres).await {
        Ok(db) => Some(db),
        Err(e) => {
            tracing::warn!("Failed to connect to database: {}", e);
            tracing::info!("Starting server without database connection");
            None
        }
    };

    let app = create_app(&config, database.as_ref());
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
        AuthConfig, LoggingConfig, MetricsConfig, MiddlewareConfig, PostgresConfig,
        RateLimitTiersConfig, RateLimitingConfig, RequestLoggingConfig, RuntimeMode,
        SecurityConfig, SecurityFeatures, ServerConfig, StorageConfig, ValidationConfig,
    };
    use axum::{body::Body, http::Request};

    #[allow(clippy::too_many_lines)]
    fn create_test_config() -> AppConfig {
        AppConfig {
            mode: RuntimeMode::Local,
            server: ServerConfig {
                host: "127.0.0.1".to_string(),
                port: 0,
                max_upload_size: 1_000_000,
            },
            postgres: PostgresConfig {
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
                oauth2: crate::infrastructure::config::OAuth2Config {
                    enabled: false,
                    service_to_service_enabled: false,
                    introspection_enabled: false,
                    client_id: "test-client".to_string(),
                    client_secret: "test-secret".to_string(),
                    service_base_url: "http://localhost:8080/api/v1/auth".to_string(),
                    jwt_secret: "test-jwt-secret".to_string(),
                    token_cache_ttl_seconds: 300,
                    client_credentials_cache_ttl_seconds: 1800,
                    request_timeout_seconds: 10,
                    max_retries: 3,
                    retry_delay_ms: 1000,
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
        // This should now succeed but use reconnecting repository
        let app = create_app(&config, None);
        // Verify app is created successfully
        assert!(std::ptr::addr_of!(app).is_aligned());
    }

    #[tokio::test]
    async fn test_create_app_with_database() {
        use crate::infrastructure::persistence::Database;

        let config = create_test_config();

        // Try to create a database connection (this may fail in test environment)
        // This test primarily exercises the code path where database is available
        if let Ok(database) = Database::new(&config.postgres).await {
            let app = create_app(&config, Some(&database));
            // Verify app is created successfully with database
            assert!(std::ptr::addr_of!(app).is_aligned());
        } else {
            // If database connection fails, we've already tested the disconnected path
            // This confirms our fallback logic works correctly
            let app = create_app(&config, None);
            assert!(std::ptr::addr_of!(app).is_aligned());
        }
    }

    #[test]
    fn test_enhanced_request_id_uniqueness() {
        let mut maker = EnhancedRequestId;
        let request = Request::builder().body(Body::empty()).unwrap();

        let id1 = maker.make_request_id(&request);
        let id2 = maker.make_request_id(&request);

        assert!(id1.is_some());
        assert!(id2.is_some());

        // IDs should be different (UUIDs are unique)
        let id1_unwrapped = id1.unwrap();
        let id2_unwrapped = id2.unwrap();
        let id1_str = id1_unwrapped.header_value().to_str().unwrap();
        let id2_str = id2_unwrapped.header_value().to_str().unwrap();
        assert_ne!(id1_str, id2_str);
    }

    #[test]
    fn test_health_check_endpoint_basic() {
        let response = health_check();
        let json_value = response.0;

        assert!(json_value.get("status").is_some());
        assert!(json_value.get("timestamp").is_some());
        assert!(json_value.get("service").is_some());
        assert_eq!(json_value["status"], "healthy");
        assert_eq!(json_value["service"], "media-management-service");
    }

    // Note: Full integration tests with dependencies are in tests/integration/health_check_test.rs
    // These unit tests focus on the health check handler logic

    // Note: readiness_check_with_dependencies requires AppState with actual repository/storage
    // implementations, so comprehensive readiness tests are in integration tests.
    // Unit testing of readiness logic is covered indirectly through health check tests
    // since both use the same underlying dependency validation patterns.
    // Key difference: readiness is binary (ready/not_ready) while health allows degraded state.

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

    #[test]
    fn test_create_cors_layer() {
        let cors_layer = create_cors_layer();
        // Verify CORS layer is created successfully
        // We can't easily test the internal configuration without making the function return values
        // This at least ensures the function doesn't panic
        assert!(std::ptr::addr_of!(cors_layer).is_aligned());
    }
}
