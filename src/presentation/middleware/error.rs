use axum::{
    extract::Request,
    http::{HeaderValue, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::{json, Value};
use std::collections::HashMap;
use thiserror::Error;
use tracing::{error, warn};
use uuid::Uuid;

/// Application error types that can be converted to HTTP responses
#[derive(Error, Debug)]
pub enum AppError {
    #[error("Authentication failed: {message}")]
    Authentication { message: String },

    #[error("Authorization failed: {message}")]
    Authorization { message: String },

    #[error("Validation failed: {errors:?}")]
    Validation { errors: HashMap<String, String> },

    #[error("Resource not found: {resource}")]
    NotFound { resource: String },

    #[error("Conflict: {message}")]
    Conflict { message: String },

    #[error("Rate limit exceeded: {message}")]
    RateLimit { message: String },

    #[error("Invalid request: {message}")]
    BadRequest { message: String },

    #[error("Request too large: {message}")]
    PayloadTooLarge { message: String },

    #[error("Unsupported media type: {content_type}")]
    UnsupportedMediaType { content_type: String },

    #[error("Database error: {message}")]
    Database { message: String },

    #[error("Storage error: {message}")]
    Storage { message: String },

    #[error("External service error: {service}: {message}")]
    ExternalService { service: String, message: String },

    #[error("Internal server error: {message}")]
    Internal { message: String },

    #[error("Service temporarily unavailable: {message}")]
    ServiceUnavailable { message: String },

    #[error("Request timeout: {message}")]
    Timeout { message: String },
}

impl AppError {
    /// Get the HTTP status code for this error
    pub fn status_code(&self) -> StatusCode {
        match self {
            AppError::Authentication { .. } => StatusCode::UNAUTHORIZED,
            AppError::Authorization { .. } => StatusCode::FORBIDDEN,
            AppError::Validation { .. } | AppError::BadRequest { .. } => StatusCode::BAD_REQUEST,
            AppError::NotFound { .. } => StatusCode::NOT_FOUND,
            AppError::Conflict { .. } => StatusCode::CONFLICT,
            AppError::RateLimit { .. } => StatusCode::TOO_MANY_REQUESTS,
            AppError::PayloadTooLarge { .. } => StatusCode::PAYLOAD_TOO_LARGE,
            AppError::UnsupportedMediaType { .. } => StatusCode::UNSUPPORTED_MEDIA_TYPE,
            AppError::Database { .. } | AppError::Storage { .. } | AppError::Internal { .. } => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
            AppError::ExternalService { .. } => StatusCode::BAD_GATEWAY,
            AppError::ServiceUnavailable { .. } => StatusCode::SERVICE_UNAVAILABLE,
            AppError::Timeout { .. } => StatusCode::REQUEST_TIMEOUT,
        }
    }

    /// Get the error type for logging and metrics
    pub fn error_type(&self) -> &'static str {
        match self {
            AppError::Authentication { .. } => "authentication",
            AppError::Authorization { .. } => "authorization",
            AppError::Validation { .. } => "validation",
            AppError::NotFound { .. } => "not_found",
            AppError::Conflict { .. } => "conflict",
            AppError::RateLimit { .. } => "rate_limit",
            AppError::BadRequest { .. } => "bad_request",
            AppError::PayloadTooLarge { .. } => "payload_too_large",
            AppError::UnsupportedMediaType { .. } => "unsupported_media_type",
            AppError::Database { .. } => "database",
            AppError::Storage { .. } => "storage",
            AppError::ExternalService { .. } => "external_service",
            AppError::Internal { .. } => "internal",
            AppError::ServiceUnavailable { .. } => "service_unavailable",
            AppError::Timeout { .. } => "timeout",
        }
    }

    /// Check if this error should be logged as an error (vs warning)
    pub fn should_log_as_error(&self) -> bool {
        matches!(
            self,
            AppError::Database { .. }
                | AppError::Storage { .. }
                | AppError::ExternalService { .. }
                | AppError::Internal { .. }
                | AppError::ServiceUnavailable { .. }
        )
    }

    /// Create error response with proper structure
    pub fn to_error_response(&self, request_id: Option<&str>) -> ErrorResponse {
        let error_id = Uuid::new_v4().to_string();

        ErrorResponse {
            error: ErrorDetail {
                id: error_id,
                error_type: self.error_type().to_string(),
                message: self.to_string(),
                details: self.get_details(),
                request_id: request_id.map(String::from),
                timestamp: chrono::Utc::now().to_rfc3339(),
            },
        }
    }

    /// Get additional error details
    fn get_details(&self) -> Option<Value> {
        match self {
            AppError::Validation { errors } => Some(json!({ "validation_errors": errors })),
            AppError::NotFound { resource } => Some(json!({ "resource": resource })),
            AppError::UnsupportedMediaType { content_type } => {
                Some(json!({ "content_type": content_type }))
            }
            AppError::ExternalService { service, .. } => Some(json!({ "service": service })),
            _ => None,
        }
    }
}

/// Structured error response
#[derive(serde::Serialize, Debug)]
pub struct ErrorResponse {
    pub error: ErrorDetail,
}

#[derive(serde::Serialize, Debug)]
pub struct ErrorDetail {
    pub id: String,
    #[serde(rename = "type")]
    pub error_type: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    pub timestamp: String,
}

impl IntoResponse for ErrorResponse {
    fn into_response(self) -> Response {
        Json(self).into_response()
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        let error_response = self.to_error_response(None);

        // Log the error appropriately
        if self.should_log_as_error() {
            error!(
                error_type = self.error_type(),
                error_id = error_response.error.id,
                "Application error: {}",
                self
            );
        } else {
            warn!(
                error_type = self.error_type(),
                error_id = error_response.error.id,
                "Application warning: {}",
                self
            );
        }

        (status, Json(error_response)).into_response()
    }
}

/// Global error handling middleware
pub async fn global_error_handler(request: Request, next: Next) -> Response {
    let request_id = extract_request_id(&request);

    // Simply run the next handler and enhance the response if needed
    let response = next.run(request).await;

    // Check if the response is an error status and enhance it
    enhance_error_response(response, request_id.as_deref())
}

/// Extract request ID from request headers
fn extract_request_id(request: &Request) -> Option<String> {
    request.headers().get("x-request-id").and_then(|v| v.to_str().ok()).map(String::from)
}

/// Enhance error responses with consistent structure
fn enhance_error_response(response: Response, request_id: Option<&str>) -> Response {
    let status = response.status();

    // Only enhance error responses (4xx, 5xx)
    if !status.is_client_error() && !status.is_server_error() {
        return response;
    }

    // If response already has proper error structure, just add request ID
    let mut enhanced_response = response;

    if let Some(req_id) = request_id {
        if let Ok(header_value) = req_id.parse::<HeaderValue>() {
            enhanced_response.headers_mut().insert("x-request-id", header_value.clone());
            enhanced_response.headers_mut().insert("x-correlation-id", header_value);
        }
    }

    enhanced_response
}

/// Convert common errors to `AppError`
impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        AppError::Database { message: err.to_string() }
    }
}

impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        AppError::Storage { message: err.to_string() }
    }
}

impl From<serde_json::Error> for AppError {
    fn from(err: serde_json::Error) -> Self {
        AppError::BadRequest { message: format!("Invalid JSON: {err}") }
    }
}

impl From<crate::infrastructure::storage::presigned_urls::PresignedUrlError> for AppError {
    fn from(err: crate::infrastructure::storage::presigned_urls::PresignedUrlError) -> Self {
        use crate::infrastructure::storage::presigned_urls::PresignedUrlError;

        match err {
            PresignedUrlError::FileTooLarge { size, max_size } => AppError::PayloadTooLarge {
                message: format!(
                    "File size {size} bytes exceeds maximum allowed size of {max_size} bytes"
                ),
            },
            PresignedUrlError::Expired { expired_at } => {
                AppError::BadRequest { message: format!("Upload URL has expired at {expired_at}") }
            }
            PresignedUrlError::InvalidSignature => {
                AppError::Authentication { message: "Invalid upload signature".to_string() }
            }
            PresignedUrlError::InvalidExpiration => {
                AppError::BadRequest { message: "Invalid expiration time".to_string() }
            }
            PresignedUrlError::SigningError => {
                AppError::Internal { message: "Failed to sign payload".to_string() }
            }
        }
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
    use tower::ServiceExt;

    async fn test_success_handler() -> Json<serde_json::Value> {
        Json(json!({"status": "ok"}))
    }

    async fn test_app_error_handler() -> Result<Json<serde_json::Value>, AppError> {
        Err(AppError::NotFound { resource: "test resource".to_string() })
    }

    #[allow(dead_code)]
    fn test_panic_handler() -> Json<serde_json::Value> {
        panic!("Test panic");
    }

    #[test]
    fn test_app_error_status_codes() {
        assert_eq!(
            AppError::Authentication { message: "test".to_string() }.status_code(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            AppError::Authorization { message: "test".to_string() }.status_code(),
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            AppError::Validation { errors: HashMap::new() }.status_code(),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            AppError::NotFound { resource: "test".to_string() }.status_code(),
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            AppError::RateLimit { message: "test".to_string() }.status_code(),
            StatusCode::TOO_MANY_REQUESTS
        );
        assert_eq!(
            AppError::Internal { message: "test".to_string() }.status_code(),
            StatusCode::INTERNAL_SERVER_ERROR
        );
    }

    #[test]
    fn test_app_error_types() {
        assert_eq!(
            AppError::Authentication { message: "test".to_string() }.error_type(),
            "authentication"
        );
        assert_eq!(AppError::Validation { errors: HashMap::new() }.error_type(), "validation");
        assert_eq!(AppError::Database { message: "test".to_string() }.error_type(), "database");
    }

    #[test]
    fn test_should_log_as_error() {
        assert!(AppError::Database { message: "test".to_string() }.should_log_as_error());
        assert!(AppError::Internal { message: "test".to_string() }.should_log_as_error());
        assert!(!AppError::NotFound { resource: "test".to_string() }.should_log_as_error());
        assert!(!AppError::BadRequest { message: "test".to_string() }.should_log_as_error());
    }

    #[test]
    fn test_error_response_structure() {
        let error = AppError::NotFound { resource: "user".to_string() };
        let response = error.to_error_response(Some("test-request-id"));

        assert_eq!(response.error.error_type, "not_found");
        assert!(response.error.message.contains("not found"));
        assert_eq!(response.error.request_id, Some("test-request-id".to_string()));
        assert!(response.error.details.is_some());

        if let Some(details) = response.error.details {
            assert_eq!(details["resource"], "user");
        }
    }

    #[test]
    fn test_validation_error_details() {
        let mut errors = HashMap::new();
        errors.insert("email".to_string(), "Invalid email format".to_string());
        errors.insert("age".to_string(), "Must be between 18 and 100".to_string());

        let error = AppError::Validation { errors };
        let response = error.to_error_response(None);

        assert!(response.error.details.is_some());
        if let Some(details) = response.error.details {
            assert!(details["validation_errors"]["email"]
                .as_str()
                .unwrap()
                .contains("Invalid email"));
            assert!(details["validation_errors"]["age"]
                .as_str()
                .unwrap()
                .contains("Must be between"));
        }
    }

    #[tokio::test]
    async fn test_global_error_handler_success() {
        let app = Router::new()
            .route("/success", get(test_success_handler))
            .layer(axum::middleware::from_fn(global_error_handler));

        let request = Request::builder()
            .uri("/success")
            .header("x-request-id", "test-id-123")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_global_error_handler_app_error() {
        let app = Router::new()
            .route("/error", get(test_app_error_handler))
            .layer(axum::middleware::from_fn(global_error_handler));

        let request = Request::builder()
            .uri("/error")
            .header("x-request-id", "test-id-456")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert!(response.headers().get("x-request-id").is_some());
    }

    #[test]
    fn test_error_conversions() {
        let sql_error = sqlx::Error::RowNotFound;
        let app_error: AppError = sql_error.into();
        assert!(matches!(app_error, AppError::Database { .. }));

        let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let app_error: AppError = io_error.into();
        assert!(matches!(app_error, AppError::Storage { .. }));
    }

    #[tokio::test]
    async fn test_extract_request_id() {
        let request = Request::builder()
            .header("x-request-id", "test-request-123")
            .body(Body::empty())
            .unwrap();

        let request_id = extract_request_id(&request);
        assert_eq!(request_id, Some("test-request-123".to_string()));
    }

    #[tokio::test]
    async fn test_extract_request_id_missing() {
        let request = Request::builder().body(Body::empty()).unwrap();

        let request_id = extract_request_id(&request);
        assert_eq!(request_id, None);
    }
}
