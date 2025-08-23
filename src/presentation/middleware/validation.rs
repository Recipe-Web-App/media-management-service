use axum::{
    body::{to_bytes, Body, Bytes},
    extract::Request,
    http::{header, HeaderMap, Method},
    middleware::Next,
    response::Response,
};
use serde_json::Value;
use std::collections::HashMap;
use tracing::debug;

use super::error::AppError;

/// Request validation configuration
#[derive(Debug, Clone)]
#[allow(clippy::struct_excessive_bools)]
pub struct ValidationConfig {
    /// Validate Content-Type headers
    pub validate_content_type: bool,
    /// Allowed content types for different methods
    pub allowed_content_types: HashMap<String, Vec<String>>,
    /// Validate request body size
    pub validate_body_size: bool,
    /// Maximum request body size (in bytes)
    pub max_body_size: usize,
    /// Validate JSON structure for JSON requests
    pub validate_json_structure: bool,
    /// Validate file uploads
    pub validate_file_uploads: bool,
    /// Allowed file types for uploads
    pub allowed_file_types: Vec<String>,
    /// Maximum file size for uploads (in bytes)
    pub max_file_size: usize,
    /// Validate request headers
    pub validate_headers: bool,
    /// Required headers for specific routes
    pub required_headers: HashMap<String, Vec<String>>,
    /// Validate request method for routes
    pub validate_methods: bool,
    /// Allowed methods for routes
    pub allowed_methods: HashMap<String, Vec<String>>,
}

impl Default for ValidationConfig {
    fn default() -> Self {
        let mut allowed_content_types = HashMap::new();
        allowed_content_types.insert(
            "POST".to_string(),
            vec![
                "application/json".to_string(),
                "multipart/form-data".to_string(),
                "application/x-www-form-urlencoded".to_string(),
            ],
        );
        allowed_content_types.insert(
            "PUT".to_string(),
            vec!["application/json".to_string(), "multipart/form-data".to_string()],
        );
        allowed_content_types.insert("PATCH".to_string(), vec!["application/json".to_string()]);

        let mut allowed_methods = HashMap::new();
        allowed_methods.insert(
            "/api/v1/media-management/media".to_string(),
            vec!["GET".to_string(), "POST".to_string()],
        );
        allowed_methods.insert(
            "/api/v1/media-management/media/*".to_string(),
            vec!["GET".to_string(), "PUT".to_string(), "DELETE".to_string()],
        );

        Self {
            validate_content_type: true,
            allowed_content_types,
            validate_body_size: true,
            max_body_size: 100 * 1024 * 1024, // 100MB
            validate_json_structure: true,
            validate_file_uploads: true,
            allowed_file_types: vec![
                "image/jpeg".to_string(),
                "image/png".to_string(),
                "image/webp".to_string(),
                "image/avif".to_string(),
                "video/mp4".to_string(),
                "video/webm".to_string(),
            ],
            max_file_size: 50 * 1024 * 1024, // 50MB
            validate_headers: true,
            required_headers: HashMap::new(),
            validate_methods: true,
            allowed_methods,
        }
    }
}

impl ValidationConfig {
    /// Create a lenient configuration for development
    pub fn lenient() -> Self {
        Self {
            validate_content_type: false,
            allowed_content_types: HashMap::new(),
            validate_body_size: true,
            max_body_size: 500 * 1024 * 1024, // 500MB for dev
            validate_json_structure: false,
            validate_file_uploads: false,
            allowed_file_types: vec![],       // Allow all in dev
            max_file_size: 100 * 1024 * 1024, // 100MB for dev
            validate_headers: false,
            required_headers: HashMap::new(),
            validate_methods: false,
            allowed_methods: HashMap::new(),
        }
    }

    /// Create a strict configuration for production
    pub fn strict() -> Self {
        let mut required_headers = HashMap::new();
        required_headers
            .insert("/api/v1/media-management/media".to_string(), vec!["content-type".to_string()]);

        Self {
            max_body_size: 10 * 1024 * 1024, // 10MB strict limit
            max_file_size: 20 * 1024 * 1024, // 20MB strict limit
            required_headers,
            ..Self::default()
        }
    }
}

/// Validation result
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub valid: bool,
    pub errors: Vec<ValidationError>,
}

impl ValidationResult {
    pub fn valid() -> Self {
        Self { valid: true, errors: vec![] }
    }

    pub fn invalid(errors: Vec<ValidationError>) -> Self {
        Self { valid: false, errors }
    }

    pub fn add_error(&mut self, error: ValidationError) {
        self.valid = false;
        self.errors.push(error);
    }
}

/// Validation error types
#[derive(Debug, Clone)]
pub enum ValidationError {
    InvalidContentType { received: String, allowed: Vec<String> },
    BodyTooLarge { size: usize, max_size: usize },
    InvalidJson { message: String },
    UnsupportedFileType { content_type: String, allowed: Vec<String> },
    FileTooLarge { size: usize, max_size: usize },
    MissingRequiredHeader { header: String },
    MethodNotAllowed { method: String, allowed: Vec<String> },
    InvalidHeaderValue { header: String, value: String, reason: String },
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidationError::InvalidContentType { received, allowed } => {
                write!(f, "Invalid content type '{}', allowed: {}", received, allowed.join(", "))
            }
            ValidationError::BodyTooLarge { size, max_size } => {
                write!(f, "Request body too large: {size} bytes (max: {max_size} bytes)")
            }
            ValidationError::InvalidJson { message } => {
                write!(f, "Invalid JSON: {message}")
            }
            ValidationError::UnsupportedFileType { content_type, allowed } => {
                write!(
                    f,
                    "Unsupported file type '{}', allowed: {}",
                    content_type,
                    allowed.join(", ")
                )
            }
            ValidationError::FileTooLarge { size, max_size } => {
                write!(f, "File too large: {size} bytes (max: {max_size} bytes)")
            }
            ValidationError::MissingRequiredHeader { header } => {
                write!(f, "Missing required header: {header}")
            }
            ValidationError::MethodNotAllowed { method, allowed } => {
                write!(f, "Method '{}' not allowed, allowed: {}", method, allowed.join(", "))
            }
            ValidationError::InvalidHeaderValue { header, value, reason } => {
                write!(f, "Invalid header '{header}' value '{value}': {reason}")
            }
        }
    }
}

/// Request validator
#[derive(Debug, Clone)]
pub struct RequestValidator {
    config: ValidationConfig,
}

impl RequestValidator {
    pub fn new(config: ValidationConfig) -> Self {
        Self { config }
    }

    /// Validate a request
    pub fn validate_request(
        &self,
        method: &Method,
        path: &str,
        headers: &HeaderMap,
        body: Option<&Bytes>,
    ) -> ValidationResult {
        let mut result = ValidationResult::valid();

        // Validate HTTP method
        if self.config.validate_methods {
            if let Some(validation_error) = self.validate_method(method, path) {
                result.add_error(validation_error);
            }
        }

        // Validate Content-Type
        if self.config.validate_content_type {
            if let Some(validation_error) = self.validate_content_type(method, headers) {
                result.add_error(validation_error);
            }
        }

        // Validate required headers
        if self.config.validate_headers {
            if let Some(validation_error) = self.validate_required_headers(path, headers) {
                result.add_error(validation_error);
            }
        }

        // Validate body size
        if self.config.validate_body_size {
            if let Some(body_bytes) = body {
                if let Some(validation_error) = self.validate_body_size(body_bytes) {
                    result.add_error(validation_error);
                }
            }
        }

        // Validate JSON structure
        if self.config.validate_json_structure {
            if let Some(body_bytes) = body {
                if let Some(validation_error) = Self::validate_json_structure(headers, body_bytes) {
                    result.add_error(validation_error);
                }
            }
        }

        // Validate file uploads
        if self.config.validate_file_uploads {
            if let Some(body_bytes) = body {
                if let Some(validation_error) = self.validate_file_upload(headers, body_bytes) {
                    result.add_error(validation_error);
                }
            }
        }

        result
    }

    /// Validate HTTP method for route
    fn validate_method(&self, method: &Method, path: &str) -> Option<ValidationError> {
        // Try exact match first
        if let Some(allowed_methods) = self.config.allowed_methods.get(path) {
            if !allowed_methods.contains(&method.to_string()) {
                return Some(ValidationError::MethodNotAllowed {
                    method: method.to_string(),
                    allowed: allowed_methods.clone(),
                });
            }
        } else {
            // Try pattern matching
            for (pattern, allowed_methods) in &self.config.allowed_methods {
                if Self::matches_pattern(path, pattern) {
                    if !allowed_methods.contains(&method.to_string()) {
                        return Some(ValidationError::MethodNotAllowed {
                            method: method.to_string(),
                            allowed: allowed_methods.clone(),
                        });
                    }
                    break;
                }
            }
        }

        None
    }

    /// Validate Content-Type header
    fn validate_content_type(
        &self,
        method: &Method,
        headers: &HeaderMap,
    ) -> Option<ValidationError> {
        // Only validate for methods that typically have request bodies
        if !matches!(method, &Method::POST | &Method::PUT | &Method::PATCH) {
            return None;
        }

        let content_type =
            headers.get(header::CONTENT_TYPE).and_then(|ct| ct.to_str().ok()).unwrap_or("");

        if content_type.is_empty() {
            return Some(ValidationError::InvalidContentType {
                received: "none".to_string(),
                allowed: self
                    .config
                    .allowed_content_types
                    .get(&method.to_string())
                    .cloned()
                    .unwrap_or_default(),
            });
        }

        // Extract main content type (ignore parameters like charset)
        let main_content_type = content_type.split(';').next().unwrap_or(content_type).trim();

        if let Some(allowed_types) = self.config.allowed_content_types.get(&method.to_string()) {
            let is_allowed = allowed_types.iter().any(|allowed| {
                if allowed == "multipart/form-data" {
                    main_content_type.starts_with("multipart/form-data")
                } else {
                    main_content_type == allowed
                }
            });

            if !is_allowed {
                return Some(ValidationError::InvalidContentType {
                    received: main_content_type.to_string(),
                    allowed: allowed_types.clone(),
                });
            }
        }

        None
    }

    /// Validate required headers
    fn validate_required_headers(
        &self,
        path: &str,
        headers: &HeaderMap,
    ) -> Option<ValidationError> {
        if let Some(required_headers) = self.config.required_headers.get(path) {
            for required_header in required_headers {
                if headers.get(required_header).is_none() {
                    return Some(ValidationError::MissingRequiredHeader {
                        header: required_header.clone(),
                    });
                }
            }
        }

        None
    }

    /// Validate request body size
    fn validate_body_size(&self, body: &Bytes) -> Option<ValidationError> {
        if body.len() > self.config.max_body_size {
            return Some(ValidationError::BodyTooLarge {
                size: body.len(),
                max_size: self.config.max_body_size,
            });
        }

        None
    }

    /// Validate JSON structure
    fn validate_json_structure(headers: &HeaderMap, body: &Bytes) -> Option<ValidationError> {
        let content_type =
            headers.get(header::CONTENT_TYPE).and_then(|ct| ct.to_str().ok()).unwrap_or("");

        if content_type.contains("application/json") {
            if let Err(e) = serde_json::from_slice::<Value>(body) {
                return Some(ValidationError::InvalidJson { message: e.to_string() });
            }
        }

        None
    }

    /// Validate file upload
    fn validate_file_upload(&self, headers: &HeaderMap, body: &Bytes) -> Option<ValidationError> {
        let content_type =
            headers.get(header::CONTENT_TYPE).and_then(|ct| ct.to_str().ok()).unwrap_or("");

        // Check if this is a file upload
        if content_type.starts_with("multipart/form-data")
            || Self::is_file_content_type(content_type)
        {
            // Check file size
            if body.len() > self.config.max_file_size {
                return Some(ValidationError::FileTooLarge {
                    size: body.len(),
                    max_size: self.config.max_file_size,
                });
            }

            // Check file type if it's a direct file upload
            if Self::is_file_content_type(content_type) {
                let main_content_type =
                    content_type.split(';').next().unwrap_or(content_type).trim();

                if !self.config.allowed_file_types.is_empty()
                    && !self.config.allowed_file_types.contains(&main_content_type.to_string())
                {
                    return Some(ValidationError::UnsupportedFileType {
                        content_type: main_content_type.to_string(),
                        allowed: self.config.allowed_file_types.clone(),
                    });
                }
            }
        }

        None
    }

    /// Check if content type represents a file
    fn is_file_content_type(content_type: &str) -> bool {
        content_type.starts_with("image/")
            || content_type.starts_with("video/")
            || content_type.starts_with("audio/")
            || content_type.starts_with("application/pdf")
            || content_type == "application/octet-stream"
    }

    /// Simple pattern matching for routes
    fn matches_pattern(path: &str, pattern: &str) -> bool {
        if let Some(prefix) = pattern.strip_suffix('*') {
            path.starts_with(prefix)
        } else {
            path == pattern
        }
    }
}

/// Request validation middleware
///
/// # Panics
///
/// Panics if primary error unwrap fails (should not occur in normal operation)
pub fn validation_middleware(
    validator: RequestValidator,
) -> impl Fn(
    Request,
    Next,
)
    -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Response, AppError>> + Send>>
       + Clone {
    move |request: Request, next: Next| {
        let validator = validator.clone();
        Box::pin(async move {
            let method = request.method().clone();
            let path = request.uri().path().to_string();
            let headers = request.headers().clone();

            // Extract body for validation if needed
            let (request, body_bytes) = if validator.config.validate_body_size
                || validator.config.validate_json_structure
                || validator.config.validate_file_uploads
            {
                let (parts, body) = request.into_parts();
                match to_bytes(body, validator.config.max_body_size + 1).await {
                    Ok(bytes) => {
                        let new_request = Request::from_parts(parts, Body::from(bytes.clone()));
                        (new_request, Some(bytes))
                    }
                    Err(_) => {
                        return Err(AppError::BadRequest {
                            message: "Failed to read request body".to_string(),
                        });
                    }
                }
            } else {
                (request, None)
            };

            // Validate request
            let validation_result =
                validator.validate_request(&method, &path, &headers, body_bytes.as_ref());

            if !validation_result.valid {
                debug!("Request validation failed: {:?}", validation_result.errors);

                // Convert validation errors to appropriate HTTP errors
                let primary_error = validation_result.errors.first().cloned();

                return match primary_error {
                    Some(ValidationError::InvalidContentType { .. }) => {
                        Err(AppError::UnsupportedMediaType {
                            content_type: headers
                                .get(header::CONTENT_TYPE)
                                .and_then(|ct| ct.to_str().ok())
                                .unwrap_or("unknown")
                                .to_string(),
                        })
                    }
                    Some(
                        ValidationError::BodyTooLarge { .. } | ValidationError::FileTooLarge { .. },
                    ) => Err(AppError::PayloadTooLarge {
                        message: primary_error.unwrap().to_string(),
                    }),
                    Some(ValidationError::MethodNotAllowed { .. }) => {
                        Err(AppError::BadRequest { message: primary_error.unwrap().to_string() })
                    }
                    _ => {
                        let error_messages: Vec<String> = validation_result
                            .errors
                            .iter()
                            .map(std::string::ToString::to_string)
                            .collect();

                        Err(AppError::BadRequest {
                            message: format!("Validation failed: {}", error_messages.join("; ")),
                        })
                    }
                };
            }

            // Continue with validated request
            Ok(next.run(request).await)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::Json;
    use serde_json::json;

    #[allow(dead_code)]
    fn test_handler() -> Json<serde_json::Value> {
        Json(json!({"message": "success"}))
    }

    #[test]
    fn test_default_validation_config() {
        let config = ValidationConfig::default();
        assert!(config.validate_content_type);
        assert!(config.validate_body_size);
        assert_eq!(config.max_body_size, 100 * 1024 * 1024);
        assert!(config.validate_json_structure);
        assert!(config.validate_file_uploads);
    }

    #[test]
    fn test_lenient_validation_config() {
        let config = ValidationConfig::lenient();
        assert!(!config.validate_content_type);
        assert!(!config.validate_json_structure);
        assert!(!config.validate_file_uploads);
        assert!(!config.validate_headers);
        assert_eq!(config.max_body_size, 500 * 1024 * 1024);
    }

    #[test]
    fn test_strict_validation_config() {
        let config = ValidationConfig::strict();
        assert_eq!(config.max_body_size, 10 * 1024 * 1024);
        assert_eq!(config.max_file_size, 20 * 1024 * 1024);
    }

    #[tokio::test]
    async fn test_request_validator_valid_json() {
        let config = ValidationConfig::default();
        let validator = RequestValidator::new(config);

        let mut headers = HeaderMap::new();
        headers
            .insert(header::CONTENT_TYPE, axum::http::HeaderValue::from_static("application/json"));

        let body = Bytes::from(r#"{"test": "value"}"#);

        let result = validator.validate_request(&Method::POST, "/test", &headers, Some(&body));

        assert!(result.valid);
        assert!(result.errors.is_empty());
    }

    #[tokio::test]
    async fn test_request_validator_invalid_json() {
        let config = ValidationConfig::default();
        let validator = RequestValidator::new(config);

        let mut headers = HeaderMap::new();
        headers
            .insert(header::CONTENT_TYPE, axum::http::HeaderValue::from_static("application/json"));

        let body = Bytes::from(r#"{"invalid": json}"#);

        let result = validator.validate_request(&Method::POST, "/test", &headers, Some(&body));

        assert!(!result.valid);
        assert!(matches!(result.errors[0], ValidationError::InvalidJson { .. }));
    }

    #[tokio::test]
    async fn test_request_validator_body_too_large() {
        let config = ValidationConfig { max_body_size: 10, ..Default::default() };
        let validator = RequestValidator::new(config);

        let mut headers = HeaderMap::new();
        headers
            .insert(header::CONTENT_TYPE, axum::http::HeaderValue::from_static("application/json"));
        let body = Bytes::from("this is a very long body");

        let result = validator.validate_request(&Method::POST, "/test", &headers, Some(&body));

        assert!(!result.valid);
        assert!(matches!(result.errors[0], ValidationError::BodyTooLarge { .. }));
    }

    #[tokio::test]
    async fn test_request_validator_invalid_content_type() {
        let config = ValidationConfig::default();
        let validator = RequestValidator::new(config);

        let mut headers = HeaderMap::new();
        headers.insert(header::CONTENT_TYPE, axum::http::HeaderValue::from_static("text/plain"));

        let result = validator.validate_request(&Method::POST, "/test", &headers, None);

        assert!(!result.valid);
        assert!(matches!(result.errors[0], ValidationError::InvalidContentType { .. }));
    }

    #[tokio::test]
    async fn test_request_validator_method_not_allowed() {
        let mut config = ValidationConfig::default();
        config
            .allowed_methods
            .insert("/test".to_string(), vec!["GET".to_string(), "POST".to_string()]);

        let validator = RequestValidator::new(config);
        let headers = HeaderMap::new();

        let result = validator.validate_request(&Method::DELETE, "/test", &headers, None);

        assert!(!result.valid);
        assert!(matches!(result.errors[0], ValidationError::MethodNotAllowed { .. }));
    }

    #[test]
    fn test_validation_error_display() {
        let error = ValidationError::InvalidContentType {
            received: "text/plain".to_string(),
            allowed: vec!["application/json".to_string()],
        };

        let display = error.to_string();
        assert!(display.contains("Invalid content type"));
        assert!(display.contains("text/plain"));
        assert!(display.contains("application/json"));
    }

    #[test]
    fn test_matches_pattern() {
        let _validator = RequestValidator::new(ValidationConfig::default());

        assert!(RequestValidator::matches_pattern("/api/users/123", "/api/users/*"));
        assert!(RequestValidator::matches_pattern("/api/users/", "/api/users/*"));
        assert!(!RequestValidator::matches_pattern("/api/posts/123", "/api/users/*"));
        assert!(RequestValidator::matches_pattern("/exact/path", "/exact/path"));
        assert!(!RequestValidator::matches_pattern("/different/path", "/exact/path"));
    }

    #[test]
    fn test_is_file_content_type() {
        let _validator = RequestValidator::new(ValidationConfig::default());

        assert!(RequestValidator::is_file_content_type("image/jpeg"));
        assert!(RequestValidator::is_file_content_type("video/mp4"));
        assert!(RequestValidator::is_file_content_type("audio/mpeg"));
        assert!(RequestValidator::is_file_content_type("application/pdf"));
        assert!(RequestValidator::is_file_content_type("application/octet-stream"));
        assert!(!RequestValidator::is_file_content_type("application/json"));
        assert!(!RequestValidator::is_file_content_type("text/html"));
    }

    #[tokio::test]
    async fn test_multipart_content_type_validation() {
        let config = ValidationConfig::default();
        let validator = RequestValidator::new(config);

        let mut headers = HeaderMap::new();
        headers.insert(
            header::CONTENT_TYPE,
            axum::http::HeaderValue::from_static(
                "multipart/form-data; boundary=----WebKitFormBoundary7MA4YWxkTrZu0gW",
            ),
        );

        let result = validator.validate_request(&Method::POST, "/upload", &headers, None);

        assert!(result.valid);
    }
}
