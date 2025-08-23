use axum::{
    extract::Request,
    http::{Method, StatusCode, Uri},
    middleware::Next,
    response::Response,
    routing::get,
    Router,
};
use metrics::{counter, describe_counter, describe_gauge, describe_histogram, gauge, histogram};
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};
use std::{
    collections::HashMap,
    net::IpAddr,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};
use tracing::debug;

/// Metrics configuration
#[derive(Debug, Clone)]
#[allow(clippy::struct_excessive_bools)]
pub struct MetricsConfig {
    /// Enable request/response metrics
    pub request_metrics: bool,
    /// Enable performance timing metrics
    pub timing_metrics: bool,
    /// Enable error metrics
    pub error_metrics: bool,
    /// Enable business metrics (custom metrics)
    pub business_metrics: bool,
    /// Group similar routes (e.g., /api/users/{id} becomes /api/users/:id)
    pub normalize_routes: bool,
    /// Metrics collection interval for gauges
    pub collection_interval: Duration,
    /// Custom labels to add to all metrics
    pub custom_labels: HashMap<String, String>,
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            request_metrics: true,
            timing_metrics: true,
            error_metrics: true,
            business_metrics: true,
            normalize_routes: true,
            collection_interval: Duration::from_secs(10),
            custom_labels: HashMap::new(),
        }
    }
}

/// Metrics collector for tracking application metrics
#[derive(Debug, Clone)]
pub struct MetricsCollector {
    config: MetricsConfig,
    active_requests: Arc<AtomicU64>,
    total_requests: Arc<AtomicU64>,
}

impl MetricsCollector {
    pub fn new(config: MetricsConfig) -> Self {
        Self {
            config,
            active_requests: Arc::new(AtomicU64::new(0)),
            total_requests: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Initialize Prometheus metrics
    pub fn initialize_metrics(&self) {
        // HTTP Request metrics
        describe_counter!("http_requests_total", "Total number of HTTP requests processed");

        describe_histogram!(
            "http_request_duration_seconds",
            "Duration of HTTP requests in seconds"
        );

        describe_counter!("http_request_size_bytes", "Size of HTTP request bodies in bytes");

        describe_counter!("http_response_size_bytes", "Size of HTTP response bodies in bytes");

        // Active requests gauge
        describe_gauge!("http_requests_active", "Number of currently active HTTP requests");

        // Error metrics
        describe_counter!("http_errors_total", "Total number of HTTP errors by status code");

        // Performance metrics
        describe_histogram!(
            "http_request_processing_seconds",
            "Time spent processing requests (excluding I/O)"
        );

        // Business metrics
        describe_counter!("media_uploads_total", "Total number of media files uploaded");

        describe_counter!("media_downloads_total", "Total number of media files downloaded");

        describe_gauge!("media_storage_bytes", "Total bytes of media storage used");

        // Authentication metrics
        describe_counter!("auth_attempts_total", "Total authentication attempts");

        describe_counter!("auth_failures_total", "Total authentication failures");

        // Rate limiting metrics
        describe_counter!("rate_limit_exceeded_total", "Total number of rate limit violations");

        debug!("Metrics initialized");
    }

    /// Record HTTP request start
    pub fn record_request_start(&self, method: &Method, uri: &Uri) -> RequestMetrics {
        self.active_requests.fetch_add(1, Ordering::Relaxed);
        self.total_requests.fetch_add(1, Ordering::Relaxed);

        let route = if self.config.normalize_routes {
            normalize_route_path(uri.path())
        } else {
            uri.path().to_string()
        };

        if self.config.request_metrics {
            counter!("http_requests_total", "method" => method.to_string(), "route" => route.clone()).increment(1);
        }

        gauge!("http_requests_active").set(self.active_requests.load(Ordering::Relaxed) as f64);

        RequestMetrics { method: method.clone(), route, start_time: Instant::now() }
    }

    /// Record HTTP request completion
    pub fn record_request_complete(
        &self,
        request_metrics: &RequestMetrics,
        status: StatusCode,
        request_size: Option<usize>,
        response_size: Option<usize>,
    ) {
        let duration = request_metrics.start_time.elapsed();

        self.active_requests.fetch_sub(1, Ordering::Relaxed);

        // Duration histogram
        if self.config.timing_metrics {
            histogram!("http_request_duration_seconds", "method" => request_metrics.method.to_string(), "route" => request_metrics.route.clone(), "status" => status.as_u16().to_string()).record(duration.as_secs_f64());
        }

        // Request/response sizes
        if let Some(req_size) = request_size {
            counter!("http_request_size_bytes", "method" => request_metrics.method.to_string(), "route" => request_metrics.route.clone()).increment(req_size as u64);
        }

        if let Some(resp_size) = response_size {
            counter!("http_response_size_bytes", "method" => request_metrics.method.to_string(), "route" => request_metrics.route.clone(), "status" => status.as_u16().to_string()).increment(resp_size as u64);
        }

        // Error metrics
        if self.config.error_metrics && (status.is_client_error() || status.is_server_error()) {
            counter!("http_errors_total", "method" => request_metrics.method.to_string(), "route" => request_metrics.route.clone(), "status" => status.as_u16().to_string(), "status_class" => get_status_class(status).to_string()).increment(1);
        }

        // Update active requests gauge
        gauge!("http_requests_active").set(self.active_requests.load(Ordering::Relaxed) as f64);
    }

    /// Record authentication attempt
    pub fn record_auth_attempt(&self, success: bool, method: &str) {
        if self.config.business_metrics {
            counter!("auth_attempts_total", "method" => method.to_string(), "success" => success.to_string()).increment(1);

            if !success {
                counter!("auth_failures_total", "method" => method.to_string()).increment(1);
            }
        }
    }

    /// Record media upload
    pub fn record_media_upload(&self, file_size: u64, media_type: &str, success: bool) {
        if self.config.business_metrics {
            counter!("media_uploads_total", "media_type" => media_type.to_string(), "success" => success.to_string()).increment(1);

            if success {
                counter!("media_storage_bytes", "operation" => "upload").increment(file_size);
            }
        }
    }

    /// Record media download
    pub fn record_media_download(&self, file_size: u64, media_type: &str) {
        if self.config.business_metrics {
            counter!("media_downloads_total", "media_type" => media_type.to_string()).increment(1);

            counter!("media_storage_bytes", "operation" => "download").increment(file_size);
        }
    }

    /// Record rate limit violation
    pub fn record_rate_limit_exceeded(&self, client_ip: Option<IpAddr>) {
        if self.config.error_metrics {
            let ip_label = client_ip.map_or_else(|| "unknown".to_string(), |ip| ip.to_string());

            counter!("rate_limit_exceeded_total", "client_ip" => ip_label).increment(1);
        }
    }
}

/// Metrics data for a single request
#[derive(Debug, Clone)]
pub struct RequestMetrics {
    pub method: Method,
    pub route: String,
    pub start_time: Instant,
}

/// Initialize Prometheus metrics exporter
pub fn initialize_prometheus_exporter() -> Result<PrometheusHandle, Box<dyn std::error::Error>> {
    let recorder = PrometheusBuilder::new().build_recorder();
    let handle = recorder.handle();
    metrics::set_global_recorder(recorder)?;
    Ok(handle)
}

/// Create /metrics endpoint for Prometheus scraping
pub fn create_metrics_endpoint(handle: PrometheusHandle) -> Router {
    Router::new().route("/metrics", get(move || async move { handle.render() }))
}

/// Metrics middleware for collecting HTTP metrics
pub fn metrics_middleware(
    collector: MetricsCollector,
) -> impl Fn(Request, Next) -> std::pin::Pin<Box<dyn std::future::Future<Output = Response> + Send>>
       + Clone {
    move |request: Request, next: Next| {
        let collector = collector.clone();
        Box::pin(async move {
            // Extract request info
            let method = request.method().clone();
            let uri = request.uri().clone();

            // Get request body size if available
            let request_size = request
                .headers()
                .get("content-length")
                .and_then(|cl| cl.to_str().ok())
                .and_then(|cl| cl.parse::<usize>().ok());

            // Start request metrics
            let request_metrics = collector.record_request_start(&method, &uri);

            // Process request
            let response = next.run(request).await;

            // Extract response info
            let status = response.status();

            // Get response body size if available
            let response_size = response
                .headers()
                .get("content-length")
                .and_then(|cl| cl.to_str().ok())
                .and_then(|cl| cl.parse::<usize>().ok());

            // Record completion
            collector.record_request_complete(
                &request_metrics,
                status,
                request_size,
                response_size,
            );

            response
        })
    }
}

/// Normalize route paths for consistent metrics
fn normalize_route_path(path: &str) -> String {
    // Common patterns to normalize
    let patterns = [
        (r"/api/v\d+/", "/api/v*/"),
        (r"/media/[0-9a-fA-F-]{36}/?", "/media/:id"),
        (r"/media/\d+/?", "/media/:id"),
        (r"/users/[0-9a-fA-F-]{36}/?", "/users/:id"),
        (r"/users/\d+/?", "/users/:id"),
    ];

    let mut normalized = path.to_string();

    for (pattern, replacement) in &patterns {
        if let Ok(re) = regex::Regex::new(pattern) {
            normalized = re.replace_all(&normalized, *replacement).to_string();
        }
    }

    // Handle trailing slashes consistently
    if normalized != "/" && normalized.ends_with('/') {
        normalized.pop();
    }

    normalized
}

/// Get HTTP status class for metrics
fn get_status_class(status: StatusCode) -> &'static str {
    match status.as_u16() {
        200..=299 => "2xx",
        300..=399 => "3xx",
        400..=499 => "4xx",
        500..=599 => "5xx",
        _ => "unknown",
    }
}

/// Background task to collect system metrics
pub async fn collect_system_metrics(collector: MetricsCollector, interval: Duration) {
    let mut interval_timer = tokio::time::interval(interval);

    loop {
        interval_timer.tick().await;

        // Collect memory usage
        if let Ok(memory_info) = get_memory_info() {
            gauge!("process_memory_bytes").set(memory_info.used_bytes as f64);
            gauge!("process_memory_available_bytes").set(memory_info.available_bytes as f64);
        }

        // Collect CPU usage (simplified)
        let cpu_usage = get_cpu_usage();
        if cpu_usage >= 0.0 {
            gauge!("process_cpu_usage_percent").set(cpu_usage);
        }

        // Update total requests gauge
        gauge!("http_requests_processed_total")
            .set(collector.total_requests.load(Ordering::Relaxed) as f64);
    }
}

/// Simple memory information struct
#[derive(Debug)]
struct MemoryInfo {
    used_bytes: u64,
    available_bytes: u64,
}

/// Get basic memory information (simplified implementation)
fn get_memory_info() -> Result<MemoryInfo, Box<dyn std::error::Error>> {
    // This is a simplified implementation
    // In production, you might want to use a proper system metrics crate
    #[cfg(target_os = "linux")]
    {
        let meminfo = std::fs::read_to_string("/proc/meminfo")?;
        let mut total = 0;
        let mut available = 0;

        for line in meminfo.lines() {
            if line.starts_with("MemTotal:") {
                total = parse_meminfo_value(line)?;
            } else if line.starts_with("MemAvailable:") {
                available = parse_meminfo_value(line)?;
            }
        }

        Ok(MemoryInfo {
            used_bytes: (total - available) * 1024, // Convert KB to bytes
            available_bytes: available * 1024,
        })
    }

    #[cfg(not(target_os = "linux"))]
    {
        // Fallback for non-Linux systems
        Ok(MemoryInfo { used_bytes: 0, available_bytes: 0 })
    }
}

#[cfg(target_os = "linux")]
fn parse_meminfo_value(line: &str) -> Result<u64, Box<dyn std::error::Error>> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() >= 2 {
        Ok(parts[1].parse()?)
    } else {
        Err("Invalid meminfo line".into())
    }
}

/// Get basic CPU usage (simplified implementation)
fn get_cpu_usage() -> f64 {
    // This is a very simplified implementation
    // In production, you'd want to use proper system monitoring
    0.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
        response::Json,
        routing::get,
    };
    use serde_json::json;
    use tower::ServiceExt;

    async fn test_handler() -> Json<serde_json::Value> {
        Json(json!({"message": "test"}))
    }

    #[test]
    fn test_default_metrics_config() {
        let config = MetricsConfig::default();
        assert!(config.request_metrics);
        assert!(config.timing_metrics);
        assert!(config.error_metrics);
        assert!(config.business_metrics);
        assert!(config.normalize_routes);
        assert_eq!(config.collection_interval, Duration::from_secs(10));
    }

    #[test]
    fn test_metrics_collector_creation() {
        let config = MetricsConfig::default();
        let collector = MetricsCollector::new(config);

        assert_eq!(collector.active_requests.load(Ordering::Relaxed), 0);
        assert_eq!(collector.total_requests.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn test_normalize_route_path() {
        assert_eq!(normalize_route_path("/api/v1/users"), "/api/v*/users");
        assert_eq!(normalize_route_path("/api/v2/media"), "/api/v*/media");
        assert_eq!(
            normalize_route_path("/media/123e4567-e89b-12d3-a456-426614174000"),
            "/media/:id"
        );
        assert_eq!(normalize_route_path("/users/123/"), "/users/:id");
        assert_eq!(normalize_route_path("/static/file.css"), "/static/file.css");
    }

    #[test]
    fn test_get_status_class() {
        assert_eq!(get_status_class(StatusCode::OK), "2xx");
        assert_eq!(get_status_class(StatusCode::CREATED), "2xx");
        assert_eq!(get_status_class(StatusCode::MOVED_PERMANENTLY), "3xx");
        assert_eq!(get_status_class(StatusCode::NOT_FOUND), "4xx");
        assert_eq!(get_status_class(StatusCode::BAD_REQUEST), "4xx");
        assert_eq!(get_status_class(StatusCode::INTERNAL_SERVER_ERROR), "5xx");
        assert_eq!(get_status_class(StatusCode::BAD_GATEWAY), "5xx");
    }

    #[tokio::test]
    async fn test_metrics_collector_request_lifecycle() {
        let config = MetricsConfig::default();
        let collector = MetricsCollector::new(config);

        // Record request start
        let method = Method::GET;
        let uri = "/test".parse().unwrap();
        let request_metrics = collector.record_request_start(&method, &uri);

        assert_eq!(collector.active_requests.load(Ordering::Relaxed), 1);
        assert_eq!(collector.total_requests.load(Ordering::Relaxed), 1);
        assert_eq!(request_metrics.method, method);
        assert_eq!(request_metrics.route, "/test");

        // Record request completion
        collector.record_request_complete(&request_metrics, StatusCode::OK, Some(100), Some(200));

        assert_eq!(collector.active_requests.load(Ordering::Relaxed), 0);
        assert_eq!(collector.total_requests.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn test_metrics_collector_auth_attempts() {
        let config = MetricsConfig::default();
        let collector = MetricsCollector::new(config);

        collector.record_auth_attempt(true, "jwt");
        collector.record_auth_attempt(false, "basic");

        // Test passes if no panics occur
        // Actual metric values would need to be verified with a test metrics recorder
    }

    #[test]
    fn test_metrics_collector_media_operations() {
        let config = MetricsConfig::default();
        let collector = MetricsCollector::new(config);

        collector.record_media_upload(1024, "image/jpeg", true);
        collector.record_media_download(2048, "video/mp4");
        collector.record_rate_limit_exceeded(Some(IpAddr::from([127, 0, 0, 1])));

        // Test passes if no panics occur
    }

    #[tokio::test]
    async fn test_metrics_middleware_integration() {
        let config = MetricsConfig::default();
        let collector = MetricsCollector::new(config);

        let app = Router::new()
            .route("/test", get(test_handler))
            .layer(axum::middleware::from_fn(metrics_middleware(collector)));

        let request = Request::builder().uri("/test").body(Body::empty()).unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[test]
    fn test_memory_info_structure() {
        let info = MemoryInfo {
            used_bytes: 1024 * 1024,     // 1MB
            available_bytes: 512 * 1024, // 512KB
        };

        assert_eq!(info.used_bytes, 1_048_576);
        assert_eq!(info.available_bytes, 524_288);
    }
}
