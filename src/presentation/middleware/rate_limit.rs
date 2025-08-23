use axum::{
    extract::{ConnectInfo, Request},
    http::HeaderMap,
    middleware::Next,
    response::Response,
};
use std::{
    collections::HashMap,
    net::{IpAddr, SocketAddr},
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::RwLock;
// Note: tower_governor API has changed significantly in v0.8.0
// For now, we'll use our SimpleRateLimiter implementation
// TODO: Update to tower_governor 0.8.0 API when stable
use tracing::{debug, warn};

use super::error::AppError;

/// Rate limiting configuration
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Maximum number of requests per window
    pub max_requests: u32,
    /// Time window duration
    pub window_duration: Duration,
    /// Burst capacity (allows short bursts above the rate limit)
    pub burst_capacity: u32,
    /// Whether to trust X-Forwarded-For header for IP extraction
    pub trust_forwarded_headers: bool,
    /// Custom rate limit headers
    pub include_headers: bool,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            max_requests: 100,
            window_duration: Duration::from_secs(60),
            burst_capacity: 10,
            trust_forwarded_headers: false,
            include_headers: true,
        }
    }
}

/// Different rate limit tiers for different endpoints
#[derive(Debug, Clone)]
pub enum RateLimitTier {
    /// Health check endpoints - very high limits
    Health,
    /// Public endpoints - moderate limits
    Public,
    /// Authenticated endpoints - higher limits
    Authenticated,
    /// Upload endpoints - lower limits due to resource intensity
    Upload,
    /// Admin endpoints - high limits for admin users
    Admin,
}

impl RateLimitTier {
    pub fn to_config(&self) -> RateLimitConfig {
        match self {
            RateLimitTier::Health => RateLimitConfig {
                max_requests: 1000,
                window_duration: Duration::from_secs(60),
                burst_capacity: 100,
                trust_forwarded_headers: false,
                include_headers: true,
            },
            RateLimitTier::Public => RateLimitConfig {
                max_requests: 60,
                window_duration: Duration::from_secs(60),
                burst_capacity: 10,
                trust_forwarded_headers: false,
                include_headers: true,
            },
            RateLimitTier::Authenticated => RateLimitConfig {
                max_requests: 200,
                window_duration: Duration::from_secs(60),
                burst_capacity: 20,
                trust_forwarded_headers: false,
                include_headers: true,
            },
            RateLimitTier::Upload => RateLimitConfig {
                max_requests: 10,
                window_duration: Duration::from_secs(60),
                burst_capacity: 2,
                trust_forwarded_headers: false,
                include_headers: true,
            },
            RateLimitTier::Admin => RateLimitConfig {
                max_requests: 500,
                window_duration: Duration::from_secs(60),
                burst_capacity: 50,
                trust_forwarded_headers: false,
                include_headers: true,
            },
        }
    }
}

/// Simple in-memory rate limiter for basic use cases
#[derive(Debug, Clone)]
pub struct SimpleRateLimiter {
    config: RateLimitConfig,
    state: Arc<RwLock<HashMap<IpAddr, RequestHistory>>>,
}

#[derive(Debug, Clone)]
struct RequestHistory {
    requests: Vec<Instant>,
    last_cleanup: Instant,
}

impl SimpleRateLimiter {
    pub fn new(config: RateLimitConfig) -> Self {
        Self { config, state: Arc::new(RwLock::new(HashMap::new())) }
    }

    /// Check rate limit for an IP address
    ///
    /// # Panics
    ///
    /// Panics if the current time is before the window duration (should not occur in normal operation)
    pub async fn check_rate_limit(&self, ip: IpAddr) -> Result<RateLimitInfo, AppError> {
        let now = Instant::now();
        let mut state = self.state.write().await;

        let history = state
            .entry(ip)
            .or_insert_with(|| RequestHistory { requests: Vec::new(), last_cleanup: now });

        // Clean up old requests
        let cutoff = now.checked_sub(self.config.window_duration).unwrap();
        history.requests.retain(|&req_time| req_time > cutoff);

        // Check if we're over the limit
        let current_count = history.requests.len() as u32;
        if current_count >= self.config.max_requests {
            let oldest_request = history.requests.first().copied().unwrap_or(now);
            let reset_time = oldest_request + self.config.window_duration;
            let retry_after = reset_time.duration_since(now);

            return Err(AppError::RateLimit {
                message: format!(
                    "Rate limit exceeded. Try again in {} seconds",
                    retry_after.as_secs()
                ),
            });
        }

        // Add current request
        history.requests.push(now);

        // Periodic cleanup to prevent memory leaks
        let should_cleanup = now.duration_since(history.last_cleanup) > Duration::from_secs(300);
        if should_cleanup {
            history.last_cleanup = now;
            // Clean up after we're done with history reference
            state.retain(|_, hist| {
                hist.requests.retain(|&req_time| req_time > cutoff);
                !hist.requests.is_empty()
            });
        }

        Ok(RateLimitInfo {
            limit: self.config.max_requests,
            remaining: self.config.max_requests - current_count - 1,
            reset_time: now + self.config.window_duration,
            retry_after: None,
        })
    }
}

/// Rate limit information
#[derive(Debug, Clone)]
pub struct RateLimitInfo {
    pub limit: u32,
    pub remaining: u32,
    pub reset_time: Instant,
    pub retry_after: Option<Duration>,
}

impl RateLimitInfo {
    pub fn add_headers(&self, headers: &mut HeaderMap) {
        use axum::http::HeaderValue;

        if let Ok(limit) = HeaderValue::from_str(&self.limit.to_string()) {
            headers.insert("x-ratelimit-limit", limit);
        }

        if let Ok(remaining) = HeaderValue::from_str(&self.remaining.to_string()) {
            headers.insert("x-ratelimit-remaining", remaining);
        }

        let reset_timestamp = self.reset_time.elapsed().as_secs();
        if let Ok(reset) = HeaderValue::from_str(&reset_timestamp.to_string()) {
            headers.insert("x-ratelimit-reset", reset);
        }

        if let Some(retry_after) = self.retry_after {
            if let Ok(retry) = HeaderValue::from_str(&retry_after.as_secs().to_string()) {
                headers.insert("retry-after", retry);
            }
        }
    }
}

/// Extract client IP address from request
fn extract_client_ip(request: &Request, trust_forwarded: bool) -> IpAddr {
    // Try to get IP from forwarded headers if trusted
    if trust_forwarded {
        // Check X-Forwarded-For header
        if let Some(forwarded) = request.headers().get("x-forwarded-for") {
            if let Ok(forwarded_str) = forwarded.to_str() {
                // Take the first IP (client IP)
                if let Some(first_ip) = forwarded_str.split(',').next() {
                    if let Ok(ip) = first_ip.trim().parse::<IpAddr>() {
                        return ip;
                    }
                }
            }
        }

        // Check X-Real-IP header
        if let Some(real_ip) = request.headers().get("x-real-ip") {
            if let Ok(ip_str) = real_ip.to_str() {
                if let Ok(ip) = ip_str.parse::<IpAddr>() {
                    return ip;
                }
            }
        }
    }

    // Fall back to connection info
    if let Some(ConnectInfo(socket_addr)) = request.extensions().get::<ConnectInfo<SocketAddr>>() {
        return socket_addr.ip();
    }

    // Default fallback
    IpAddr::from([127, 0, 0, 1])
}

/// Simple rate limiting middleware
pub fn simple_rate_limit_middleware(
    limiter: SimpleRateLimiter,
) -> impl Fn(
    Request,
    Next,
)
    -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Response, AppError>> + Send>>
       + Clone {
    move |request: Request, next: Next| {
        let limiter = limiter.clone();
        Box::pin(async move {
            let client_ip = extract_client_ip(&request, limiter.config.trust_forwarded_headers);

            debug!("Rate limit check for IP: {}", client_ip);

            match limiter.check_rate_limit(client_ip).await {
                Ok(info) => {
                    let mut response = next.run(request).await;

                    if limiter.config.include_headers {
                        info.add_headers(response.headers_mut());
                    }

                    Ok(response)
                }
                Err(rate_limit_error) => {
                    warn!("Rate limit exceeded for IP: {}", client_ip);
                    Err(rate_limit_error)
                }
            }
        })
    }
}

// Tower Governor integration has been simplified for now
// The SimpleRateLimiter above provides the core functionality
// TODO: Add tower_governor integration when API stabilizes

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::Request, response::Json};
    use std::net::Ipv4Addr;
    use tokio::time::{sleep, Duration};

    #[allow(dead_code)]
    fn test_handler() -> Json<serde_json::Value> {
        Json(serde_json::json!({"message": "success"}))
    }

    #[test]
    fn test_rate_limit_tier_configs() {
        let health_config = RateLimitTier::Health.to_config();
        assert_eq!(health_config.max_requests, 1000);

        let public_config = RateLimitTier::Public.to_config();
        assert_eq!(public_config.max_requests, 60);

        let upload_config = RateLimitTier::Upload.to_config();
        assert_eq!(upload_config.max_requests, 10);
    }

    #[tokio::test]
    async fn test_simple_rate_limiter() {
        let config = RateLimitConfig {
            max_requests: 3,
            window_duration: Duration::from_secs(60),
            burst_capacity: 1,
            trust_forwarded_headers: false,
            include_headers: true,
        };

        let limiter = SimpleRateLimiter::new(config);
        let test_ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));

        // First 3 requests should succeed
        for i in 0..3 {
            let result = limiter.check_rate_limit(test_ip).await;
            assert!(result.is_ok(), "Request {} should succeed", i + 1);

            if let Ok(info) = result {
                assert_eq!(info.limit, 3);
                assert_eq!(info.remaining, 3 - i - 1);
            }
        }

        // 4th request should fail
        let result = limiter.check_rate_limit(test_ip).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AppError::RateLimit { .. }));
    }

    #[tokio::test]
    async fn test_rate_limit_window_reset() {
        let config = RateLimitConfig {
            max_requests: 2,
            window_duration: Duration::from_millis(100), // Very short window for testing
            burst_capacity: 1,
            trust_forwarded_headers: false,
            include_headers: true,
        };

        let limiter = SimpleRateLimiter::new(config);
        let test_ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 2));

        // Use up the limit
        assert!(limiter.check_rate_limit(test_ip).await.is_ok());
        assert!(limiter.check_rate_limit(test_ip).await.is_ok());
        assert!(limiter.check_rate_limit(test_ip).await.is_err());

        // Wait for window to reset
        sleep(Duration::from_millis(150)).await;

        // Should be able to make requests again
        assert!(limiter.check_rate_limit(test_ip).await.is_ok());
    }

    #[test]
    fn test_extract_client_ip_from_connection() {
        let socket_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(203, 0, 113, 1)), 8080);
        let mut request = Request::builder().body(Body::empty()).unwrap();
        request.extensions_mut().insert(ConnectInfo(socket_addr));

        let ip = extract_client_ip(&request, false);
        assert_eq!(ip, IpAddr::V4(Ipv4Addr::new(203, 0, 113, 1)));
    }

    #[test]
    fn test_extract_client_ip_from_forwarded_header() {
        let mut request = Request::builder()
            .header("x-forwarded-for", "203.0.113.1, 192.168.1.1")
            .body(Body::empty())
            .unwrap();

        // Add a fallback connection info
        let socket_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)), 8080);
        request.extensions_mut().insert(ConnectInfo(socket_addr));

        // With trust_forwarded = true, should use forwarded header
        let ip = extract_client_ip(&request, true);
        assert_eq!(ip, IpAddr::V4(Ipv4Addr::new(203, 0, 113, 1)));

        // With trust_forwarded = false, should use connection info
        let ip = extract_client_ip(&request, false);
        assert_eq!(ip, IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)));
    }

    #[test]
    fn test_extract_client_ip_from_real_ip_header() {
        let mut request =
            Request::builder().header("x-real-ip", "203.0.113.2").body(Body::empty()).unwrap();

        // Add a fallback connection info
        let socket_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)), 8080);
        request.extensions_mut().insert(ConnectInfo(socket_addr));

        let ip = extract_client_ip(&request, true);
        assert_eq!(ip, IpAddr::V4(Ipv4Addr::new(203, 0, 113, 2)));
    }

    #[test]
    fn test_rate_limit_info_headers() {
        let info = RateLimitInfo {
            limit: 100,
            remaining: 85,
            reset_time: Instant::now() + Duration::from_secs(60),
            retry_after: Some(Duration::from_secs(30)),
        };

        let mut headers = HeaderMap::new();
        info.add_headers(&mut headers);

        assert_eq!(headers.get("x-ratelimit-limit").unwrap(), "100");
        assert_eq!(headers.get("x-ratelimit-remaining").unwrap(), "85");
        assert!(headers.get("x-ratelimit-reset").is_some());
        assert_eq!(headers.get("retry-after").unwrap(), "30");
    }

    #[test]
    fn test_default_rate_limit_config() {
        let config = RateLimitConfig::default();
        assert_eq!(config.max_requests, 100);
        assert_eq!(config.window_duration, Duration::from_secs(60));
        assert_eq!(config.burst_capacity, 10);
        assert!(!config.trust_forwarded_headers);
        assert!(config.include_headers);
    }

    #[tokio::test]
    async fn test_rate_limit_different_ips() {
        let config = RateLimitConfig {
            max_requests: 2,
            window_duration: Duration::from_secs(60),
            burst_capacity: 1,
            trust_forwarded_headers: false,
            include_headers: true,
        };

        let limiter = SimpleRateLimiter::new(config);
        let ip1 = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));
        let ip2 = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 2));

        // Each IP should have its own limit
        assert!(limiter.check_rate_limit(ip1).await.is_ok());
        assert!(limiter.check_rate_limit(ip1).await.is_ok());
        assert!(limiter.check_rate_limit(ip1).await.is_err()); // IP1 exhausted

        // IP2 should still work
        assert!(limiter.check_rate_limit(ip2).await.is_ok());
        assert!(limiter.check_rate_limit(ip2).await.is_ok());
        assert!(limiter.check_rate_limit(ip2).await.is_err()); // IP2 exhausted
    }

    // Tower Governor tests removed for now - using SimpleRateLimiter instead
    // TODO: Add tower_governor tests when API is updated
}
