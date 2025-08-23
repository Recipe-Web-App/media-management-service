use axum::{
    extract::Request,
    http::{header, HeaderValue},
    middleware::Next,
    response::Response,
};

/// Security headers configuration
#[derive(Debug, Clone)]
#[allow(clippy::struct_excessive_bools)]
pub struct SecurityConfig {
    /// Enable HSTS (HTTP Strict Transport Security)
    pub hsts_enabled: bool,
    /// HSTS max age in seconds (default: 1 year)
    pub hsts_max_age: u64,
    /// Include subdomains in HSTS
    pub hsts_include_subdomains: bool,
    /// HSTS preload directive
    pub hsts_preload: bool,
    /// Content Security Policy
    pub csp_policy: Option<String>,
    /// Frame options policy
    pub frame_options: FrameOptions,
    /// Content type options
    pub content_type_options: bool,
    /// XSS protection
    pub xss_protection: XssProtection,
    /// Referrer policy
    pub referrer_policy: ReferrerPolicy,
    /// Permissions policy
    pub permissions_policy: Option<String>,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            hsts_enabled: true,
            hsts_max_age: 31_536_000, // 1 year
            hsts_include_subdomains: true,
            hsts_preload: false,
            csp_policy: Some(
                "default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'; img-src 'self' data: https:; font-src 'self'; connect-src 'self'; media-src 'self'; object-src 'none'; child-src 'none'; worker-src 'none'; frame-ancestors 'none'; form-action 'self'; base-uri 'self'"
                    .to_string(),
            ),
            frame_options: FrameOptions::Deny,
            content_type_options: true,
            xss_protection: XssProtection::Block,
            referrer_policy: ReferrerPolicy::StrictOriginWhenCrossOrigin,
            permissions_policy: Some(
                "accelerometer=(), camera=(), geolocation=(), gyroscope=(), magnetometer=(), microphone=(), payment=(), usb=()"
                    .to_string(),
            ),
        }
    }
}

/// X-Frame-Options values
#[derive(Debug, Clone)]
pub enum FrameOptions {
    Deny,
    SameOrigin,
    AllowFrom(String),
}

impl FrameOptions {
    fn to_header_value(&self) -> HeaderValue {
        match self {
            FrameOptions::Deny => HeaderValue::from_static("DENY"),
            FrameOptions::SameOrigin => HeaderValue::from_static("SAMEORIGIN"),
            FrameOptions::AllowFrom(uri) => HeaderValue::from_str(&format!("ALLOW-FROM {uri}"))
                .unwrap_or_else(|_| HeaderValue::from_static("DENY")),
        }
    }
}

/// X-XSS-Protection values
#[derive(Debug, Clone)]
pub enum XssProtection {
    Disabled,
    Enabled,
    Block,
}

impl XssProtection {
    fn to_header_value(&self) -> HeaderValue {
        match self {
            XssProtection::Disabled => HeaderValue::from_static("0"),
            XssProtection::Enabled => HeaderValue::from_static("1"),
            XssProtection::Block => HeaderValue::from_static("1; mode=block"),
        }
    }
}

/// Referrer-Policy values
#[derive(Debug, Clone)]
pub enum ReferrerPolicy {
    NoReferrer,
    NoReferrerWhenDowngrade,
    Origin,
    OriginWhenCrossOrigin,
    SameOrigin,
    StrictOrigin,
    StrictOriginWhenCrossOrigin,
    UnsafeUrl,
}

impl ReferrerPolicy {
    fn to_header_value(&self) -> HeaderValue {
        let value = match self {
            ReferrerPolicy::NoReferrer => "no-referrer",
            ReferrerPolicy::NoReferrerWhenDowngrade => "no-referrer-when-downgrade",
            ReferrerPolicy::Origin => "origin",
            ReferrerPolicy::OriginWhenCrossOrigin => "origin-when-cross-origin",
            ReferrerPolicy::SameOrigin => "same-origin",
            ReferrerPolicy::StrictOrigin => "strict-origin",
            ReferrerPolicy::StrictOriginWhenCrossOrigin => "strict-origin-when-cross-origin",
            ReferrerPolicy::UnsafeUrl => "unsafe-url",
        };
        HeaderValue::from_static(value)
    }
}

/// Security headers middleware
pub fn security_headers_middleware(
    config: SecurityConfig,
) -> impl Fn(Request, Next) -> std::pin::Pin<Box<dyn std::future::Future<Output = Response> + Send>>
       + Clone {
    move |request: Request, next: Next| {
        let config = config.clone();
        Box::pin(async move {
            let mut response = next.run(request).await;
            let headers = response.headers_mut();

            // HSTS Header
            if config.hsts_enabled {
                let mut hsts_value = format!("max-age={}", config.hsts_max_age);
                if config.hsts_include_subdomains {
                    hsts_value.push_str("; includeSubDomains");
                }
                if config.hsts_preload {
                    hsts_value.push_str("; preload");
                }
                if let Ok(header_value) = HeaderValue::from_str(&hsts_value) {
                    headers.insert(header::STRICT_TRANSPORT_SECURITY, header_value);
                }
            }

            // Content Security Policy
            if let Some(csp) = &config.csp_policy {
                if let Ok(header_value) = HeaderValue::from_str(csp) {
                    headers.insert(header::CONTENT_SECURITY_POLICY, header_value);
                }
            }

            // X-Frame-Options
            headers.insert("x-frame-options", config.frame_options.to_header_value());

            // X-Content-Type-Options
            if config.content_type_options {
                headers.insert("x-content-type-options", HeaderValue::from_static("nosniff"));
            }

            // X-XSS-Protection
            headers.insert("x-xss-protection", config.xss_protection.to_header_value());

            // Referrer-Policy
            headers.insert("referrer-policy", config.referrer_policy.to_header_value());

            // Permissions-Policy
            if let Some(permissions) = &config.permissions_policy {
                if let Ok(header_value) = HeaderValue::from_str(permissions) {
                    headers.insert("permissions-policy", header_value);
                }
            }

            // Additional security headers
            headers.insert("x-permitted-cross-domain-policies", HeaderValue::from_static("none"));
            headers
                .insert("cross-origin-embedder-policy", HeaderValue::from_static("require-corp"));
            headers.insert("cross-origin-opener-policy", HeaderValue::from_static("same-origin"));
            headers.insert("cross-origin-resource-policy", HeaderValue::from_static("same-origin"));

            response
        })
    }
}

/// Create security headers layer for Tower
/// Simple security headers middleware
pub async fn add_security_headers(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;

    // Add security headers
    let headers = response.headers_mut();

    headers.insert("X-Frame-Options", HeaderValue::from_static("DENY"));
    headers.insert("X-Content-Type-Options", HeaderValue::from_static("nosniff"));
    headers.insert("X-XSS-Protection", HeaderValue::from_static("1; mode=block"));
    headers.insert(
        "Strict-Transport-Security",
        HeaderValue::from_static("max-age=31536000; includeSubDomains"),
    );
    headers.insert("Referrer-Policy", HeaderValue::from_static("strict-origin-when-cross-origin"));

    response
}

/// Security headers service implementation
#[derive(Clone)]
pub struct SecurityHeadersService<S> {
    service: S,
    config: SecurityConfig,
}

impl<S, ReqBody, ResBody> tower::Service<axum::http::Request<ReqBody>> for SecurityHeadersService<S>
where
    S: tower::Service<axum::http::Request<ReqBody>, Response = axum::http::Response<ResBody>>
        + Clone
        + Send
        + 'static,
    S::Future: Send + 'static,
    ReqBody: Send + 'static,
    ResBody: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>,
    >;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&mut self, request: axum::http::Request<ReqBody>) -> Self::Future {
        let mut service = self.service.clone();
        let config = self.config.clone();

        Box::pin(async move {
            let mut response = service.call(request).await?;
            let headers = response.headers_mut();

            // Apply all security headers
            apply_security_headers(headers, &config);

            Ok(response)
        })
    }
}

/// Apply security headers to response headers
fn apply_security_headers(headers: &mut axum::http::HeaderMap, config: &SecurityConfig) {
    // HSTS Header
    if config.hsts_enabled {
        let mut hsts_value = format!("max-age={}", config.hsts_max_age);
        if config.hsts_include_subdomains {
            hsts_value.push_str("; includeSubDomains");
        }
        if config.hsts_preload {
            hsts_value.push_str("; preload");
        }
        if let Ok(header_value) = HeaderValue::from_str(&hsts_value) {
            headers.insert(header::STRICT_TRANSPORT_SECURITY, header_value);
        }
    }

    // Content Security Policy
    if let Some(csp) = &config.csp_policy {
        if let Ok(header_value) = HeaderValue::from_str(csp) {
            headers.insert(header::CONTENT_SECURITY_POLICY, header_value);
        }
    }

    // X-Frame-Options
    headers.insert("x-frame-options", config.frame_options.to_header_value());

    // X-Content-Type-Options
    if config.content_type_options {
        headers.insert("x-content-type-options", HeaderValue::from_static("nosniff"));
    }

    // X-XSS-Protection
    headers.insert("x-xss-protection", config.xss_protection.to_header_value());

    // Referrer-Policy
    headers.insert("referrer-policy", config.referrer_policy.to_header_value());

    // Permissions-Policy
    if let Some(permissions) = &config.permissions_policy {
        if let Ok(header_value) = HeaderValue::from_str(permissions) {
            headers.insert("permissions-policy", header_value);
        }
    }

    // Additional security headers
    headers.insert("x-permitted-cross-domain-policies", HeaderValue::from_static("none"));
    headers.insert("cross-origin-embedder-policy", HeaderValue::from_static("require-corp"));
    headers.insert("cross-origin-opener-policy", HeaderValue::from_static("same-origin"));
    headers.insert("cross-origin-resource-policy", HeaderValue::from_static("same-origin"));
}

/// Create a minimal security configuration for development
pub fn development_security_config() -> SecurityConfig {
    SecurityConfig {
        hsts_enabled: false, // Don't enforce HTTPS in development
        csp_policy: Some(
            "default-src 'self' 'unsafe-inline' 'unsafe-eval'; img-src 'self' data: https:"
                .to_string(),
        ),
        frame_options: FrameOptions::SameOrigin,
        content_type_options: true,
        xss_protection: XssProtection::Block,
        referrer_policy: ReferrerPolicy::StrictOriginWhenCrossOrigin,
        permissions_policy: None, // Disable in development for easier testing
        ..Default::default()
    }
}

/// Create a strict security configuration for production
pub fn production_security_config() -> SecurityConfig {
    SecurityConfig {
        hsts_enabled: true,
        hsts_max_age: 63_072_000, // 2 years
        hsts_include_subdomains: true,
        hsts_preload: true,
        csp_policy: Some(
            "default-src 'none'; script-src 'self'; style-src 'self'; img-src 'self'; font-src 'self'; connect-src 'self'; media-src 'self'; frame-ancestors 'none'; form-action 'self'; base-uri 'self'"
                .to_string(),
        ),
        frame_options: FrameOptions::Deny,
        content_type_options: true,
        xss_protection: XssProtection::Block,
        referrer_policy: ReferrerPolicy::StrictOrigin,
        permissions_policy: Some(
            "accelerometer=(), camera=(), geolocation=(), gyroscope=(), magnetometer=(), microphone=(), payment=(), usb=(), interest-cohort=()"
                .to_string(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::Json;
    use serde_json::json;

    #[allow(dead_code)]
    fn test_handler() -> Json<serde_json::Value> {
        Json(json!({"message": "test"}))
    }

    #[test]
    fn test_frame_options_header_values() {
        assert_eq!(FrameOptions::Deny.to_header_value(), HeaderValue::from_static("DENY"));
        assert_eq!(
            FrameOptions::SameOrigin.to_header_value(),
            HeaderValue::from_static("SAMEORIGIN")
        );
        assert_eq!(
            FrameOptions::AllowFrom("https://example.com".to_string()).to_header_value(),
            HeaderValue::from_str("ALLOW-FROM https://example.com").unwrap()
        );
    }

    #[test]
    fn test_xss_protection_header_values() {
        assert_eq!(XssProtection::Disabled.to_header_value(), HeaderValue::from_static("0"));
        assert_eq!(XssProtection::Enabled.to_header_value(), HeaderValue::from_static("1"));
        assert_eq!(
            XssProtection::Block.to_header_value(),
            HeaderValue::from_static("1; mode=block")
        );
    }

    #[test]
    fn test_referrer_policy_header_values() {
        assert_eq!(
            ReferrerPolicy::NoReferrer.to_header_value(),
            HeaderValue::from_static("no-referrer")
        );
        assert_eq!(
            ReferrerPolicy::StrictOriginWhenCrossOrigin.to_header_value(),
            HeaderValue::from_static("strict-origin-when-cross-origin")
        );
        assert_eq!(ReferrerPolicy::Origin.to_header_value(), HeaderValue::from_static("origin"));
    }

    #[test]
    fn test_default_security_config() {
        let config = SecurityConfig::default();
        assert!(config.hsts_enabled);
        assert_eq!(config.hsts_max_age, 31_536_000);
        assert!(config.hsts_include_subdomains);
        assert!(!config.hsts_preload);
        assert!(config.csp_policy.is_some());
        assert!(config.content_type_options);
        assert!(matches!(config.xss_protection, XssProtection::Block));
    }

    #[test]
    fn test_development_security_config() {
        let config = development_security_config();
        assert!(!config.hsts_enabled); // Should be disabled in dev
        assert!(config.csp_policy.is_some());
        assert!(config.permissions_policy.is_none()); // Should be disabled in dev
    }

    #[test]
    fn test_production_security_config() {
        let config = production_security_config();
        assert!(config.hsts_enabled);
        assert_eq!(config.hsts_max_age, 63_072_000); // 2 years
        assert!(config.hsts_preload);
        assert!(config.csp_policy.is_some());
        assert!(config.permissions_policy.is_some());
    }

    #[tokio::test]
    async fn test_apply_security_headers() {
        let mut headers = axum::http::HeaderMap::new();
        let config = SecurityConfig::default();

        apply_security_headers(&mut headers, &config);

        // Check that basic security headers are present
        assert!(headers.get("x-frame-options").is_some());
        assert!(headers.get("x-content-type-options").is_some());
        assert!(headers.get("x-xss-protection").is_some());
        assert!(headers.get("referrer-policy").is_some());
        assert!(headers.get(header::STRICT_TRANSPORT_SECURITY).is_some());
        assert!(headers.get(header::CONTENT_SECURITY_POLICY).is_some());

        // Check additional security headers
        assert!(headers.get("x-permitted-cross-domain-policies").is_some());
        assert!(headers.get("cross-origin-embedder-policy").is_some());
        assert!(headers.get("cross-origin-opener-policy").is_some());
        assert!(headers.get("cross-origin-resource-policy").is_some());
    }

    #[tokio::test]
    async fn test_security_headers_values() {
        let mut headers = axum::http::HeaderMap::new();
        let config = SecurityConfig::default();

        apply_security_headers(&mut headers, &config);

        assert_eq!(headers.get("x-frame-options").unwrap(), "DENY");
        assert_eq!(headers.get("x-content-type-options").unwrap(), "nosniff");
        assert_eq!(headers.get("x-xss-protection").unwrap(), "1; mode=block");
        assert_eq!(headers.get("referrer-policy").unwrap(), "strict-origin-when-cross-origin");
        assert_eq!(headers.get("x-permitted-cross-domain-policies").unwrap(), "none");
    }

    #[tokio::test]
    async fn test_hsts_header_construction() {
        let mut headers = axum::http::HeaderMap::new();
        let config = SecurityConfig {
            hsts_enabled: true,
            hsts_max_age: 31_536_000,
            hsts_include_subdomains: true,
            hsts_preload: true,
            ..Default::default()
        };

        apply_security_headers(&mut headers, &config);

        let hsts_header = headers.get(header::STRICT_TRANSPORT_SECURITY).unwrap().to_str().unwrap();
        assert!(hsts_header.contains("max-age=31536000"));
        assert!(hsts_header.contains("includeSubDomains"));
        assert!(hsts_header.contains("preload"));
    }

    #[tokio::test]
    async fn test_hsts_disabled() {
        let mut headers = axum::http::HeaderMap::new();
        let config = SecurityConfig { hsts_enabled: false, ..Default::default() };

        apply_security_headers(&mut headers, &config);

        assert!(headers.get(header::STRICT_TRANSPORT_SECURITY).is_none());
    }

    #[tokio::test]
    async fn test_custom_csp_policy() {
        let mut headers = axum::http::HeaderMap::new();
        let custom_csp = "default-src 'self'; script-src 'none'".to_string();
        let config = SecurityConfig { csp_policy: Some(custom_csp.clone()), ..Default::default() };

        apply_security_headers(&mut headers, &config);

        let csp_header = headers.get(header::CONTENT_SECURITY_POLICY).unwrap().to_str().unwrap();
        assert_eq!(csp_header, custom_csp);
    }

    #[tokio::test]
    async fn test_no_csp_policy() {
        let mut headers = axum::http::HeaderMap::new();
        let config = SecurityConfig { csp_policy: None, ..Default::default() };

        apply_security_headers(&mut headers, &config);

        assert!(headers.get(header::CONTENT_SECURITY_POLICY).is_none());
    }

    #[tokio::test]
    async fn test_custom_permissions_policy() {
        let mut headers = axum::http::HeaderMap::new();
        let custom_permissions = "camera=(), microphone=()".to_string();
        let config = SecurityConfig {
            permissions_policy: Some(custom_permissions.clone()),
            ..Default::default()
        };

        apply_security_headers(&mut headers, &config);

        let permissions_header = headers.get("permissions-policy").unwrap().to_str().unwrap();
        assert_eq!(permissions_header, custom_permissions);
    }

    #[tokio::test]
    async fn test_frame_options_allow_from() {
        let mut headers = axum::http::HeaderMap::new();
        let config = SecurityConfig {
            frame_options: FrameOptions::AllowFrom("https://trusted.example.com".to_string()),
            ..Default::default()
        };

        apply_security_headers(&mut headers, &config);

        let frame_options = headers.get("x-frame-options").unwrap().to_str().unwrap();
        assert_eq!(frame_options, "ALLOW-FROM https://trusted.example.com");
    }
}
