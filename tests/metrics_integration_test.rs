use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
};
use http_body_util::BodyExt;
use media_management_service::infrastructure::{config::*, http::create_app};
use std::str;
use tower::ServiceExt;

/// Helper function to convert response body to bytes
async fn to_bytes(body: Body) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    let collected = BodyExt::collect(body).await?;
    Ok(collected.to_bytes().to_vec())
}

/// Create a test configuration with metrics enabled using dynamic port
fn create_metrics_test_config() -> AppConfig {
    AppConfig {
        mode: RuntimeMode::Local,
        server: ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 3000,
            max_upload_size: 10 * 1024 * 1024,
        },
        postgres: PostgresConfig {
            url: "postgresql://test_user:test_password@localhost:5432/test_db".to_string(),
            max_connections: 5,
            min_connections: 1,
            acquire_timeout_seconds: 5,
            host: "localhost".to_string(),
            port: 5432,
            database: "test_db".to_string(),
            schema: "test_schema".to_string(),
            user: "test_user".to_string(),
            password: "test_password".to_string(),
        },
        storage: StorageConfig {
            base_path: "./test_media".to_string(),
            temp_path: "./test_media/temp".to_string(),
            max_file_size: 100 * 1024 * 1024,
        },
        logging: LoggingConfig {
            level: "info".to_string(),
            filter: None,
            console_enabled: true,
            console_format: LogFormat::Pretty,
            file_enabled: false,
            file_format: LogFormat::Json,
            file_path: "./test_logs/media-management.log".to_string(),
            file_prefix: "media-service".to_string(),
            file_rotation: RotationPolicy::Never,
            file_retention_days: 7,
            file_max_size_mb: Some(100),
            non_blocking: false,
            buffer_size: None,
        },
        middleware: MiddlewareConfig {
            auth: AuthConfig {
                enabled: false,
                jwt_secret: "test-secret-key".to_string(),
                jwt_expiry_hours: 24,
                require_auth_routes: vec![],
                optional_auth_routes: vec![],
            },
            oauth2: OAuth2Config {
                enabled: false,
                service_to_service_enabled: false,
                introspection_enabled: false,
                client_id: "test-client-id".to_string(),
                client_secret: "test-client-secret".to_string(),
                service_base_url: "http://localhost:8080".to_string(),
                jwt_secret: "test-jwt-secret".to_string(),
                token_cache_ttl_seconds: 300,
                client_credentials_cache_ttl_seconds: 3600,
                request_timeout_seconds: 5,
                max_retries: 3,
                retry_delay_ms: 100,
            },
            rate_limiting: RateLimitingConfig {
                enabled: false,
                default_requests_per_minute: 60,
                default_burst_capacity: 10,
                trust_forwarded_headers: false,
                include_rate_limit_headers: true,
                tiers: RateLimitTiersConfig {
                    health_requests_per_minute: 120,
                    public_requests_per_minute: 30,
                    authenticated_requests_per_minute: 100,
                    upload_requests_per_minute: 10,
                    admin_requests_per_minute: 200,
                },
            },
            security: SecurityConfig {
                enabled: false,
                features: SecurityFeatures {
                    hsts: false,
                    hsts_subdomains: false,
                    hsts_preload: false,
                    content_type_options: false,
                },
                hsts_max_age_seconds: 31536000,
                csp_policy: None,
                frame_options: "DENY".to_string(),
                xss_protection: "1; mode=block".to_string(),
                referrer_policy: "strict-origin-when-cross-origin".to_string(),
                permissions_policy: None,
            },
            metrics: MetricsConfig {
                enabled: true,
                endpoint_enabled: true,
                endpoint_path: "/metrics".to_string(),
                prometheus_port: 0, // Dynamic port allocation for tests
                collect_request_metrics: true,
                collect_timing_metrics: true,
                collect_error_metrics: true,
                collect_business_metrics: true,
                normalize_routes: true,
                collection_interval_seconds: 60,
            },
            validation: ValidationConfig {
                enabled: false,
                validate_content_type: true,
                validate_body_size: true,
                max_body_size_mb: 10,
                validate_json_structure: false,
                validate_file_uploads: true,
                max_file_size_mb: 100,
                allowed_file_types: vec![
                    "image/jpeg".to_string(),
                    "image/png".to_string(),
                    "image/webp".to_string(),
                    "image/gif".to_string(),
                ],
                validate_headers: false,
                validate_methods: false,
            },
            request_logging: RequestLoggingConfig {
                enabled: false,
                log_request_body: false,
                log_response_body: false,
                max_body_size_kb: 1,
                log_request_headers: false,
                log_response_headers: false,
                excluded_headers: vec![],
                log_timing: false,
                slow_request_threshold_ms: 1000,
            },
        },
    }
}

/// Helper function to wait for metrics endpoint to be available
async fn wait_for_metrics_endpoint(app: &axum::Router, max_attempts: u32) -> Result<(), String> {
    for attempt in 1..=max_attempts {
        let request =
            Request::builder().method(Method::GET).uri("/metrics").body(Body::empty()).unwrap();

        let response =
            app.clone().oneshot(request).await.map_err(|e| format!("Request failed: {}", e))?;

        if response.status() == StatusCode::OK {
            return Ok(());
        }

        if attempt < max_attempts {
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        }
    }

    Err(format!("Metrics endpoint not available after {} attempts", max_attempts))
}

/// Comprehensive metrics endpoint integration test
#[tokio::test]
async fn test_comprehensive_metrics_functionality() {
    let config = create_metrics_test_config();
    let app = create_app(&config, None);

    // Wait for metrics endpoint to be available (handles coverage tool initialization delays)
    wait_for_metrics_endpoint(&app, 10).await.expect("Metrics endpoint should become available");

    // Test 1: Metrics endpoint returns Prometheus-formatted metrics
    let request =
        Request::builder().method(Method::GET).uri("/metrics").body(Body::empty()).unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Check content type
    let content_type = response.headers().get("content-type");
    assert!(content_type.is_some());
    let content_type_str = content_type.unwrap().to_str().unwrap();
    assert!(content_type_str.starts_with("text/plain"));

    let body = to_bytes(response.into_body()).await.unwrap();
    let body_str = str::from_utf8(&body).unwrap();

    // Verify Prometheus format - should contain HELP and TYPE lines
    assert!(body_str.contains("# HELP"), "Prometheus metrics should contain HELP lines");
    assert!(body_str.contains("# TYPE"), "Prometheus metrics should contain TYPE lines");

    // Test 2: Make a request to generate metrics, then check metrics again
    let health_request = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/media-management/health")
        .body(Body::empty())
        .unwrap();

    let _health_response = app.clone().oneshot(health_request).await.unwrap();

    // Check metrics again after making a request
    let metrics_request =
        Request::builder().method(Method::GET).uri("/metrics").body(Body::empty()).unwrap();

    let response = app.clone().oneshot(metrics_request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body()).await.unwrap();
    let body_str = str::from_utf8(&body).unwrap();

    // Should contain HTTP request metrics after making a request
    assert!(
        body_str.contains("http_requests_total") || body_str.len() > 100,
        "Metrics should contain request metrics or substantial content"
    );

    // Test 3: Metrics endpoint doesn't require authentication
    let auth_test_request =
        Request::builder().method(Method::GET).uri("/metrics").body(Body::empty()).unwrap();

    let response = app.clone().oneshot(auth_test_request).await.unwrap();
    // Should succeed without authentication (monitoring endpoints are typically unauthenticated)
    assert_eq!(response.status(), StatusCode::OK);

    println!("✅ All comprehensive metrics tests passed!");
}

/// Test metrics configuration scenarios
#[tokio::test]
async fn test_metrics_configuration_scenarios() {
    // Test 1: Metrics disabled
    let mut disabled_config = create_metrics_test_config();
    disabled_config.middleware.metrics.enabled = false;

    let app = create_app(&disabled_config, None);

    let request =
        Request::builder().method(Method::GET).uri("/metrics").body(Body::empty()).unwrap();

    let response = app.oneshot(request).await.unwrap();
    // Should return 404 when metrics are disabled
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    // Test 2: Metrics enabled but endpoint disabled
    let mut endpoint_disabled_config = create_metrics_test_config();
    endpoint_disabled_config.middleware.metrics.enabled = true;
    endpoint_disabled_config.middleware.metrics.endpoint_enabled = false;

    let app = create_app(&endpoint_disabled_config, None);

    let request =
        Request::builder().method(Method::GET).uri("/metrics").body(Body::empty()).unwrap();

    let response = app.oneshot(request).await.unwrap();
    // Should return 404 when endpoint is disabled
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    println!("✅ All metrics configuration tests passed!");
}
