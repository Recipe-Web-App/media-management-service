use axum::{
    body::{to_bytes, Body, Bytes},
    extract::Request,
    http::{HeaderMap, Method, StatusCode, Uri, Version},
    middleware::Next,
    response::Response,
};
use serde_json::{json, Value};
use std::{
    net::IpAddr,
    time::{Duration, Instant},
};
use tracing::{info, warn};

/// Logging configuration for request/response middleware
#[derive(Debug, Clone)]
#[allow(clippy::struct_excessive_bools)]
pub struct LoggingConfig {
    /// Log request bodies (be careful with sensitive data)
    pub log_request_body: bool,
    /// Log response bodies (be careful with sensitive data)
    pub log_response_body: bool,
    /// Maximum body size to log (in bytes)
    pub max_body_size: usize,
    /// Log request headers
    pub log_request_headers: bool,
    /// Log response headers
    pub log_response_headers: bool,
    /// Headers to exclude from logging (for sensitive data)
    pub excluded_headers: Vec<String>,
    /// Log performance timing
    pub log_timing: bool,
    /// Minimum duration to log slow requests (in milliseconds)
    pub slow_request_threshold_ms: u64,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            log_request_body: false,  // Default to false for security
            log_response_body: false, // Default to false for security
            max_body_size: 1024,      // 1KB max by default
            log_request_headers: true,
            log_response_headers: false,
            excluded_headers: vec![
                "authorization".to_string(),
                "cookie".to_string(),
                "set-cookie".to_string(),
                "x-api-key".to_string(),
                "x-auth-token".to_string(),
            ],
            log_timing: true,
            slow_request_threshold_ms: 1000, // 1 second
        }
    }
}

/// Request/response logging information
#[derive(Debug)]
pub struct RequestInfo {
    pub method: Method,
    pub uri: Uri,
    pub version: Version,
    pub headers: HeaderMap,
    pub body: Option<Bytes>,
    pub client_ip: Option<IpAddr>,
    pub user_agent: Option<String>,
    pub request_id: Option<String>,
    pub start_time: Instant,
}

#[derive(Debug)]
pub struct ResponseInfo {
    pub status: StatusCode,
    pub headers: HeaderMap,
    pub body: Option<Bytes>,
    pub duration: Duration,
}

impl LoggingConfig {
    /// Create a development-friendly logging config
    pub fn development() -> Self {
        Self {
            log_request_body: true,
            log_response_body: true,
            max_body_size: 10_000, // 10KB for dev
            log_request_headers: true,
            log_response_headers: true,
            excluded_headers: vec![
                "authorization".to_string(),
                "cookie".to_string(),
                "set-cookie".to_string(),
            ],
            log_timing: true,
            slow_request_threshold_ms: 500, // 500ms threshold for dev
        }
    }

    /// Create a production logging config (more restrictive)
    pub fn production() -> Self {
        Self {
            log_request_body: false,
            log_response_body: false,
            max_body_size: 0,
            log_request_headers: false,
            log_response_headers: false,
            excluded_headers: vec![
                "authorization".to_string(),
                "cookie".to_string(),
                "set-cookie".to_string(),
                "x-api-key".to_string(),
                "x-auth-token".to_string(),
                "x-forwarded-for".to_string(),
                "x-real-ip".to_string(),
            ],
            log_timing: true,
            slow_request_threshold_ms: 2000, // 2 second threshold for prod
        }
    }

    /// Should we log this header?
    fn should_log_header(&self, header_name: &str) -> bool {
        !self
            .excluded_headers
            .iter()
            .any(|excluded| header_name.to_lowercase() == excluded.to_lowercase())
    }

    /// Filter headers for logging
    fn filter_headers(&self, headers: &HeaderMap) -> Value {
        let mut filtered = serde_json::Map::new();

        for (name, value) in headers {
            let name_str = name.as_str();
            if self.should_log_header(name_str) {
                if let Ok(value_str) = value.to_str() {
                    filtered.insert(name_str.to_string(), json!(value_str));
                } else {
                    filtered.insert(name_str.to_string(), json!("<binary>"));
                }
            }
        }

        Value::Object(filtered)
    }

    /// Should we log the body based on content type and size?
    fn should_log_body(&self, headers: &HeaderMap, body_size: usize) -> bool {
        if body_size > self.max_body_size {
            return false;
        }

        // Check content type
        if let Some(content_type) = headers.get("content-type") {
            if let Ok(content_type_str) = content_type.to_str() {
                let content_type_lower = content_type_str.to_lowercase();

                // Don't log binary content
                if content_type_lower.contains("image/")
                    || content_type_lower.contains("video/")
                    || content_type_lower.contains("audio/")
                    || content_type_lower.contains("application/octet-stream")
                {
                    return false;
                }
            }
        }

        true
    }

    /// Format body for logging
    fn format_body(&self, body: &[u8], headers: &HeaderMap) -> Value {
        if !self.should_log_body(headers, body.len()) {
            return json!({
                "size": body.len(),
                "content": "<not logged>"
            });
        }

        // Try to parse as UTF-8
        match std::str::from_utf8(body) {
            Ok(body_str) => {
                // Try to parse as JSON for pretty logging
                if let Ok(json_value) = serde_json::from_str::<Value>(body_str) {
                    json!({
                        "size": body.len(),
                        "content": json_value
                    })
                } else {
                    json!({
                        "size": body.len(),
                        "content": body_str
                    })
                }
            }
            Err(_) => json!({
                "size": body.len(),
                "content": "<binary data>"
            }),
        }
    }
}

/// Extract client IP from request
fn extract_client_ip(headers: &HeaderMap) -> Option<IpAddr> {
    // Try X-Forwarded-For first
    if let Some(forwarded) = headers.get("x-forwarded-for") {
        if let Ok(forwarded_str) = forwarded.to_str() {
            if let Some(first_ip) = forwarded_str.split(',').next() {
                if let Ok(ip) = first_ip.trim().parse::<IpAddr>() {
                    return Some(ip);
                }
            }
        }
    }

    // Try X-Real-IP
    if let Some(real_ip) = headers.get("x-real-ip") {
        if let Ok(ip_str) = real_ip.to_str() {
            if let Ok(ip) = ip_str.parse::<IpAddr>() {
                return Some(ip);
            }
        }
    }

    None
}

/// Request/response logging middleware
pub fn logging_middleware(
    config: LoggingConfig,
) -> impl Fn(Request, Next) -> std::pin::Pin<Box<dyn std::future::Future<Output = Response> + Send>>
       + Clone {
    move |request: Request, next: Next| {
        let config = config.clone();
        Box::pin(async move {
            let start_time = Instant::now();

            // Extract request information
            let method = request.method().clone();
            let uri = request.uri().clone();
            let version = request.version();
            let headers = request.headers().clone();
            let client_ip = extract_client_ip(&headers);
            let user_agent =
                headers.get("user-agent").and_then(|ua| ua.to_str().ok()).map(String::from);
            let request_id =
                headers.get("x-request-id").and_then(|id| id.to_str().ok()).map(String::from);

            // Extract request body if needed
            let (request, request_body) = if config.log_request_body {
                let (parts, body) = request.into_parts();
                if let Ok(bytes) = to_bytes(body, config.max_body_size).await {
                    let new_request = Request::from_parts(parts, Body::from(bytes.clone()));
                    (new_request, Some(bytes))
                } else {
                    let new_request = Request::from_parts(parts, Body::empty());
                    (new_request, None)
                }
            } else {
                (request, None)
            };

            // Log request
            let request_info = RequestInfo {
                method: method.clone(),
                uri: uri.clone(),
                version,
                headers: headers.clone(),
                body: request_body,
                client_ip,
                user_agent,
                request_id: request_id.clone(),
                start_time,
            };

            log_request(&config, &request_info);

            // Process request
            let response = next.run(request).await;

            // Extract response information
            let status = response.status();
            let response_headers = response.headers().clone();
            let duration = start_time.elapsed();

            // Extract response body if needed
            let (response, response_body) = if config.log_response_body {
                let (parts, body) = response.into_parts();
                if let Ok(bytes) = to_bytes(body, config.max_body_size).await {
                    let new_response = Response::from_parts(parts, Body::from(bytes.clone()));
                    (new_response, Some(bytes))
                } else {
                    let new_response = Response::from_parts(parts, Body::empty());
                    (new_response, None)
                }
            } else {
                (response, None)
            };

            // Log response
            let response_info =
                ResponseInfo { status, headers: response_headers, body: response_body, duration };

            log_response(&config, &request_info, &response_info);

            response
        })
    }
}

/// Log request information
fn log_request(config: &LoggingConfig, info: &RequestInfo) {
    let mut log_data = json!({
        "type": "request",
        "method": info.method.as_str(),
        "uri": info.uri.to_string(),
        "version": format!("{:?}", info.version),
        "timestamp": chrono::Utc::now().to_rfc3339(),
    });

    // Add request ID if available
    if let Some(request_id) = &info.request_id {
        log_data["request_id"] = json!(request_id);
    }

    // Add client IP if available
    if let Some(client_ip) = &info.client_ip {
        log_data["client_ip"] = json!(client_ip.to_string());
    }

    // Add user agent if available
    if let Some(user_agent) = &info.user_agent {
        log_data["user_agent"] = json!(user_agent);
    }

    // Add headers if configured
    if config.log_request_headers {
        log_data["headers"] = config.filter_headers(&info.headers);
    }

    // Add body if configured and available
    if config.log_request_body {
        if let Some(body) = &info.body {
            log_data["body"] = config.format_body(body, &info.headers);
        }
    }

    info!(target: "http_requests", "{}", log_data);
}

/// Log response information
fn log_response(config: &LoggingConfig, request_info: &RequestInfo, response_info: &ResponseInfo) {
    let mut log_data = json!({
        "type": "response",
        "method": request_info.method.as_str(),
        "uri": request_info.uri.to_string(),
        "status": response_info.status.as_u16(),
        "status_text": response_info.status.canonical_reason().unwrap_or("Unknown"),
        "duration_ms": response_info.duration.as_millis(),
        "timestamp": chrono::Utc::now().to_rfc3339(),
    });

    // Add request ID if available
    if let Some(request_id) = &request_info.request_id {
        log_data["request_id"] = json!(request_id);
    }

    // Add client IP if available
    if let Some(client_ip) = &request_info.client_ip {
        log_data["client_ip"] = json!(client_ip.to_string());
    }

    // Add response headers if configured
    if config.log_response_headers {
        log_data["headers"] = config.filter_headers(&response_info.headers);
    }

    // Add response body if configured and available
    if config.log_response_body {
        if let Some(body) = &response_info.body {
            log_data["body"] = config.format_body(body, &response_info.headers);
        }
    }

    // Determine log level based on status and timing
    let log_level = if response_info.status.is_server_error() {
        "error"
    } else if response_info.status.is_client_error()
        || response_info.duration.as_millis() > u128::from(config.slow_request_threshold_ms)
    {
        "warn"
    } else {
        "info"
    };

    match log_level {
        "error" => tracing::error!(target: "http_responses", "{}", log_data),
        "warn" => tracing::warn!(target: "http_responses", "{}", log_data),
        _ => tracing::info!(target: "http_responses", "{}", log_data),
    }

    // Log slow requests separately for monitoring
    if config.log_timing
        && response_info.duration.as_millis() > u128::from(config.slow_request_threshold_ms)
    {
        warn!(
            target: "slow_requests",
            method = request_info.method.as_str(),
            uri = request_info.uri.to_string(),
            status = response_info.status.as_u16(),
            duration_ms = response_info.duration.as_millis(),
            request_id = request_info.request_id.as_deref().unwrap_or("unknown"),
            "Slow request detected"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
        response::Json,
        routing::get,
        Router,
    };
    use serde_json::json;
    use std::net::Ipv4Addr;
    use tower::ServiceExt;

    async fn test_handler() -> Json<serde_json::Value> {
        Json(json!({"message": "test response"}))
    }

    #[allow(dead_code)]
    fn echo_handler(body: String) -> String {
        body
    }

    #[test]
    fn test_default_logging_config() {
        let config = LoggingConfig::default();
        assert!(!config.log_request_body);
        assert!(!config.log_response_body);
        assert_eq!(config.max_body_size, 1024);
        assert!(config.log_request_headers);
        assert!(!config.log_response_headers);
        assert!(config.excluded_headers.contains(&"authorization".to_string()));
    }

    #[test]
    fn test_development_logging_config() {
        let config = LoggingConfig::development();
        assert!(config.log_request_body);
        assert!(config.log_response_body);
        assert_eq!(config.max_body_size, 10_000);
        assert!(config.log_request_headers);
        assert!(config.log_response_headers);
        assert_eq!(config.slow_request_threshold_ms, 500);
    }

    #[test]
    fn test_production_logging_config() {
        let config = LoggingConfig::production();
        assert!(!config.log_request_body);
        assert!(!config.log_response_body);
        assert_eq!(config.max_body_size, 0);
        assert!(!config.log_request_headers);
        assert!(!config.log_response_headers);
        assert_eq!(config.slow_request_threshold_ms, 2000);
    }

    #[test]
    fn test_should_log_header() {
        let config = LoggingConfig::default();

        assert!(config.should_log_header("content-type"));
        assert!(config.should_log_header("accept"));
        assert!(!config.should_log_header("authorization"));
        assert!(!config.should_log_header("Authorization"));
        assert!(!config.should_log_header("cookie"));
        assert!(!config.should_log_header("x-api-key"));
    }

    #[test]
    fn test_filter_headers() {
        let config = LoggingConfig::default();
        let mut headers = HeaderMap::new();
        headers.insert("content-type", "application/json".parse().unwrap());
        headers.insert("authorization", "Bearer token123".parse().unwrap());
        headers.insert("x-request-id", "req-456".parse().unwrap());

        let filtered = config.filter_headers(&headers);

        assert!(filtered.get("content-type").is_some());
        assert!(filtered.get("x-request-id").is_some());
        assert!(filtered.get("authorization").is_none());
    }

    #[test]
    fn test_should_log_body_size_limit() {
        let config = LoggingConfig { max_body_size: 100, ..Default::default() };
        let headers = HeaderMap::new();

        assert!(config.should_log_body(&headers, 50));
        assert!(!config.should_log_body(&headers, 150));
    }

    #[test]
    fn test_should_log_body_content_type() {
        let config = LoggingConfig::default();

        let mut headers = HeaderMap::new();
        headers.insert("content-type", "application/json".parse().unwrap());
        assert!(config.should_log_body(&headers, 500));

        headers.insert("content-type", "image/jpeg".parse().unwrap());
        assert!(!config.should_log_body(&headers, 500));

        headers.insert("content-type", "video/mp4".parse().unwrap());
        assert!(!config.should_log_body(&headers, 500));
    }

    #[test]
    fn test_format_body_json() {
        let config = LoggingConfig::default();
        let body = r#"{"test": "value"}"#.as_bytes();
        let headers = HeaderMap::new();

        let formatted = config.format_body(body, &headers);
        assert_eq!(formatted["size"], body.len());
        assert_eq!(formatted["content"]["test"], "value");
    }

    #[test]
    fn test_format_body_plain_text() {
        let config = LoggingConfig::default();
        let body = "plain text content".as_bytes();
        let headers = HeaderMap::new();

        let formatted = config.format_body(body, &headers);
        assert_eq!(formatted["size"], body.len());
        assert_eq!(formatted["content"], "plain text content");
    }

    #[test]
    fn test_format_body_too_large() {
        let config = LoggingConfig { max_body_size: 10, ..Default::default() };
        let body = "this is a very long body content".as_bytes();
        let headers = HeaderMap::new();

        let formatted = config.format_body(body, &headers);
        assert_eq!(formatted["size"], body.len());
        assert_eq!(formatted["content"], "<not logged>");
    }

    #[test]
    fn test_extract_client_ip_forwarded() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", "203.0.113.1, 192.168.1.1".parse().unwrap());

        let ip = extract_client_ip(&headers);
        assert_eq!(ip, Some(IpAddr::V4(Ipv4Addr::new(203, 0, 113, 1))));
    }

    #[test]
    fn test_extract_client_ip_real_ip() {
        let mut headers = HeaderMap::new();
        headers.insert("x-real-ip", "203.0.113.2".parse().unwrap());

        let ip = extract_client_ip(&headers);
        assert_eq!(ip, Some(IpAddr::V4(Ipv4Addr::new(203, 0, 113, 2))));
    }

    #[test]
    fn test_extract_client_ip_none() {
        let headers = HeaderMap::new();
        let ip = extract_client_ip(&headers);
        assert_eq!(ip, None);
    }

    #[tokio::test]
    async fn test_logging_middleware_basic() {
        // This test mainly ensures the middleware doesn't panic
        // Actual logging output would need to be captured with a test subscriber
        let config = LoggingConfig::default();
        let app = Router::new()
            .route("/test", get(test_handler))
            .layer(axum::middleware::from_fn(logging_middleware(config)));

        let request = Request::builder()
            .uri("/test")
            .header("x-request-id", "test-123")
            .header("user-agent", "test-agent")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_request_info_creation() {
        let method = Method::GET;
        let uri = "/test".parse().unwrap();
        let mut headers = HeaderMap::new();
        headers.insert("user-agent", "test-agent".parse().unwrap());
        headers.insert("x-request-id", "req-789".parse().unwrap());

        let info = RequestInfo {
            method: method.clone(),
            uri,
            version: Version::HTTP_11,
            headers: headers.clone(),
            body: None,
            client_ip: Some(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1))),
            user_agent: Some("test-agent".to_string()),
            request_id: Some("req-789".to_string()),
            start_time: Instant::now(),
        };

        assert_eq!(info.method, method);
        assert_eq!(info.user_agent.as_deref(), Some("test-agent"));
        assert_eq!(info.request_id.as_deref(), Some("req-789"));
    }
}
