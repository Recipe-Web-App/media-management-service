//! Middleware modules for HTTP request processing
//!
//! This module contains all the middleware components for the media management service:
//! - Authentication & authorization
//! - Rate limiting
//! - Security headers
//! - Request validation
//! - Metrics collection
//! - Request/response logging
//! - Global error handling
//! - Request ID enhancement

pub mod auth;
pub mod error;
pub mod logging;
pub mod metrics;
pub mod rate_limit;
pub mod request_id;
pub mod security;
pub mod validation;

// Re-export commonly used types
pub use auth::{Claims, JwtService, UserContext};
pub use error::{AppError, ErrorResponse};
pub use logging::LoggingConfig as RequestLoggingConfig;
pub use metrics::{MetricsCollector, MetricsConfig as MiddlewareMetricsConfig};
pub use rate_limit::{RateLimitConfig, RateLimitTier, SimpleRateLimiter};
pub use request_id::EnhancedRequestId;
pub use security::{
    development_security_config, production_security_config,
    SecurityConfig as MiddlewareSecurityConfig,
};
pub use validation::{RequestValidator, ValidationConfig as MiddlewareValidationConfig};
