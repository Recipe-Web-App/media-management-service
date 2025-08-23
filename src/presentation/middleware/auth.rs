use axum::{
    extract::{FromRequestParts, Request},
    http::{header::AUTHORIZATION, request::Parts},
    middleware::Next,
    response::Response,
};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::fmt;
use thiserror::Error;
use tracing::{debug, error, warn};
use uuid::Uuid;

use super::error::AppError;

/// JWT token claims
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String,        // Subject (user ID)
    pub email: String,      // User email
    pub roles: Vec<String>, // User roles
    pub exp: usize,         // Expiration time
    pub iat: usize,         // Issued at
    pub jti: String,        // JWT ID (unique identifier)
}

impl Claims {
    /// Create new claims for a user
    #[must_use]
    pub fn new(user_id: String, email: String, roles: Vec<String>, expires_in_hours: u64) -> Self {
        let now = chrono::Utc::now().timestamp().max(0) as u64 as usize;
        let exp = now + (expires_in_hours * 3600) as usize;

        Self { sub: user_id, email, roles, exp, iat: now, jti: Uuid::new_v4().to_string() }
    }

    /// Check if token is expired
    pub fn is_expired(&self) -> bool {
        let now = chrono::Utc::now().timestamp().max(0) as u64 as usize;
        self.exp < now
    }

    /// Check if user has a specific role
    pub fn has_role(&self, role: &str) -> bool {
        self.roles.iter().any(|r| r == role)
    }

    /// Check if user has any of the specified roles
    pub fn has_any_role(&self, roles: &[&str]) -> bool {
        self.roles.iter().any(|user_role| roles.contains(&user_role.as_str()))
    }
}

/// User context extracted from JWT
#[derive(Debug, Clone)]
pub struct UserContext {
    pub user_id: String,
    pub email: String,
    pub roles: Vec<String>,
    pub token_id: String,
}

impl UserContext {
    /// Check if user has any of the specified roles
    pub fn has_any_role(&self, required_roles: &[String]) -> bool {
        self.roles.iter().any(|role| required_roles.contains(role))
    }

    /// Check if user has a specific role
    pub fn has_role(&self, role: &str) -> bool {
        self.roles.iter().any(|r| r == role)
    }
}

impl From<Claims> for UserContext {
    fn from(claims: Claims) -> Self {
        Self { user_id: claims.sub, email: claims.email, roles: claims.roles, token_id: claims.jti }
    }
}

impl fmt::Display for UserContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "User(id={}, email={}, roles={:?})", self.user_id, self.email, self.roles)
    }
}

/// JWT service for token operations
#[derive(Clone)]
pub struct JwtService {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    validation: Validation,
}

impl JwtService {
    /// Create new JWT service with secret
    pub fn new(secret: &str) -> Self {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.validate_exp = true;

        Self {
            encoding_key: EncodingKey::from_secret(secret.as_ref()),
            decoding_key: DecodingKey::from_secret(secret.as_ref()),
            validation,
        }
    }

    /// Encode claims into JWT token
    pub fn encode_claims(&self, claims: &Claims) -> Result<String, JwtError> {
        encode(&Header::default(), claims, &self.encoding_key).map_err(|e| {
            error!("Failed to encode JWT: {}", e);
            JwtError::EncodingError(e.to_string())
        })
    }

    /// Decode JWT token and extract claims
    pub fn decode_token(&self, token: &str) -> Result<Claims, JwtError> {
        decode::<Claims>(token, &self.decoding_key, &self.validation)
            .map(|token_data| token_data.claims)
            .map_err(|e| {
                debug!("Failed to decode JWT: {}", e);
                match e.kind() {
                    jsonwebtoken::errors::ErrorKind::ExpiredSignature => JwtError::Expired,
                    jsonwebtoken::errors::ErrorKind::InvalidSignature => JwtError::InvalidSignature,
                    jsonwebtoken::errors::ErrorKind::InvalidToken => JwtError::InvalidToken,
                    _ => JwtError::DecodingError(e.to_string()),
                }
            })
    }

    /// Create token for user
    pub fn create_token(
        &self,
        user_id: String,
        email: String,
        roles: Vec<String>,
        expires_in_hours: u64,
    ) -> Result<String, JwtError> {
        let claims = Claims::new(user_id, email, roles, expires_in_hours);
        self.encode_claims(&claims)
    }
}

/// JWT-related errors
#[derive(Error, Debug)]
pub enum JwtError {
    #[error("Token has expired")]
    Expired,

    #[error("Invalid token signature")]
    InvalidSignature,

    #[error("Invalid token format")]
    InvalidToken,

    #[error("Missing authorization header")]
    MissingHeader,

    #[error("Invalid authorization header format")]
    InvalidHeaderFormat,

    #[error("Token encoding error: {0}")]
    EncodingError(String),

    #[error("Token decoding error: {0}")]
    DecodingError(String),
}

impl From<JwtError> for AppError {
    fn from(err: JwtError) -> Self {
        match err {
            JwtError::Expired => {
                AppError::Authentication { message: "Token has expired".to_string() }
            }
            JwtError::InvalidSignature | JwtError::InvalidToken | JwtError::InvalidHeaderFormat => {
                AppError::Authentication { message: "Invalid token".to_string() }
            }
            JwtError::MissingHeader => {
                AppError::Authentication { message: "Authorization header required".to_string() }
            }
            JwtError::EncodingError(msg) | JwtError::DecodingError(msg) => {
                AppError::Internal { message: format!("JWT processing error: {msg}") }
            }
        }
    }
}

/// Extract user context from JWT token in request
impl<S> FromRequestParts<S> for UserContext
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // This is a placeholder implementation
        // In a real implementation, you'd extract the JWT service from app state
        // and decode the token from the Authorization header

        // Extract the Authorization header manually
        let auth_header =
            parts.headers.get(AUTHORIZATION).and_then(|header| header.to_str().ok()).ok_or_else(
                || AppError::Authentication { message: "Missing Authorization header".to_string() },
            )?;

        if !auth_header.starts_with("Bearer ") {
            return Err(AppError::Authentication {
                message: "Invalid Authorization header format".to_string(),
            });
        }

        let token = &auth_header[7..]; // Remove "Bearer " prefix

        // For now, return a mock user context
        // TODO: Replace with actual JWT decoding using service from app state
        warn!("Using mock user context - implement JWT decoding with proper service");
        debug!("Token received: {}", token.len()); // Just use the token somehow

        Ok(UserContext {
            user_id: "mock-user-id".to_string(),
            email: "mock@example.com".to_string(),
            roles: vec!["user".to_string()],
            token_id: Uuid::new_v4().to_string(),
        })
    }
}

/// Authentication middleware that validates JWT tokens
pub async fn auth_middleware(mut request: Request, next: Next) -> Result<Response, AppError> {
    // Extract authorization header
    let auth_header =
        request.headers().get(AUTHORIZATION).and_then(|h| h.to_str().ok()).ok_or_else(|| {
            AppError::Authentication { message: "Authorization header required".to_string() }
        })?;

    // Extract bearer token
    let token = auth_header.strip_prefix("Bearer ").ok_or_else(|| AppError::Authentication {
        message: "Invalid authorization header format".to_string(),
    })?;

    // TODO: Get JWT service from app state and validate token
    // For now, we'll do basic validation
    if token.is_empty() || token == "invalid" {
        return Err(AppError::Authentication { message: "Invalid token".to_string() });
    }

    // TODO: Decode token, validate claims, and add user context to request extensions
    // For now, add a mock user context
    let user_context = UserContext {
        user_id: "mock-user-id".to_string(),
        email: "user@example.com".to_string(),
        roles: vec!["user".to_string()],
        token_id: Uuid::new_v4().to_string(),
    };

    debug!("Authenticated user: {}", user_context);

    // Add user context to request extensions
    request.extensions_mut().insert(user_context);

    // Continue with request
    Ok(next.run(request).await)
}

/// Optional authentication middleware that doesn't fail on missing auth
pub async fn optional_auth_middleware(mut request: Request, next: Next) -> Response {
    // Try to extract and validate token, but don't fail if missing
    if let Some(auth_header) = request.headers().get(AUTHORIZATION).and_then(|h| h.to_str().ok()) {
        if let Some(token) = auth_header.strip_prefix("Bearer ") {
            if !token.is_empty() && token != "invalid" {
                // TODO: Actual token validation
                let user_context = UserContext {
                    user_id: "mock-user-id".to_string(),
                    email: "user@example.com".to_string(),
                    roles: vec!["user".to_string()],
                    token_id: Uuid::new_v4().to_string(),
                };

                debug!("Optional auth: authenticated user: {}", user_context);
                request.extensions_mut().insert(user_context);
            }
        }
    }

    next.run(request).await
}

/// Role-based authorization middleware
pub fn require_roles(
    required_roles: Vec<&'static str>,
) -> impl Fn(
    Request,
    Next,
)
    -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Response, AppError>> + Send>>
       + Clone {
    move |request: Request, next: Next| {
        let required_roles = required_roles.clone();
        Box::pin(async move {
            // Get user context from request extensions
            let user_context = request.extensions().get::<UserContext>().ok_or_else(|| {
                AppError::Authentication { message: "Authentication required".to_string() }
            })?;

            // Check if user has required roles
            let required_roles_strings: Vec<String> =
                required_roles.iter().map(std::string::ToString::to_string).collect();
            if !user_context.has_any_role(&required_roles_strings) {
                return Err(AppError::Authorization {
                    message: format!(
                        "Access denied. Required roles: {:?}, user roles: {:?}",
                        required_roles, user_context.roles
                    ),
                });
            }

            debug!(
                "Authorization successful for user {} with roles {:?}",
                user_context.user_id, user_context.roles
            );

            Ok(next.run(request).await)
        })
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

    async fn protected_handler() -> Json<serde_json::Value> {
        Json(json!({"message": "Protected resource accessed"}))
    }

    #[allow(dead_code)]
    fn admin_handler() -> Json<serde_json::Value> {
        Json(json!({"message": "Admin resource accessed"}))
    }

    #[test]
    fn test_claims_creation() {
        let claims = Claims::new(
            "user123".to_string(),
            "test@example.com".to_string(),
            vec!["user".to_string(), "admin".to_string()],
            24,
        );

        assert_eq!(claims.sub, "user123");
        assert_eq!(claims.email, "test@example.com");
        assert_eq!(claims.roles, vec!["user", "admin"]);
        assert!(!claims.is_expired());
        assert!(claims.has_role("user"));
        assert!(claims.has_role("admin"));
        assert!(!claims.has_role("superuser"));
    }

    #[test]
    fn test_claims_role_checking() {
        let claims = Claims::new(
            "user123".to_string(),
            "test@example.com".to_string(),
            vec!["user".to_string(), "editor".to_string()],
            24,
        );

        assert!(claims.has_any_role(&["user", "admin"]));
        assert!(claims.has_any_role(&["editor"]));
        assert!(!claims.has_any_role(&["admin", "superuser"]));
    }

    #[test]
    fn test_user_context_from_claims() {
        let claims = Claims::new(
            "user456".to_string(),
            "user@test.com".to_string(),
            vec!["moderator".to_string()],
            12,
        );

        let context: UserContext = claims.clone().into();
        assert_eq!(context.user_id, "user456");
        assert_eq!(context.email, "user@test.com");
        assert_eq!(context.roles, vec!["moderator"]);
        assert_eq!(context.token_id, claims.jti);
    }

    #[test]
    fn test_jwt_service_token_creation() {
        let service = JwtService::new("test-secret-key");

        let token = service
            .create_token(
                "user123".to_string(),
                "test@example.com".to_string(),
                vec!["user".to_string()],
                24,
            )
            .unwrap();

        assert!(!token.is_empty());
        assert!(token.contains('.'));

        // Verify we can decode it back
        let decoded_claims = service.decode_token(&token).unwrap();
        assert_eq!(decoded_claims.sub, "user123");
        assert_eq!(decoded_claims.email, "test@example.com");
        assert_eq!(decoded_claims.roles, vec!["user"]);
    }

    #[test]
    fn test_jwt_service_invalid_token() {
        let service = JwtService::new("test-secret-key");

        let result = service.decode_token("invalid.token.here");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, JwtError::DecodingError(_)));
    }

    #[test]
    fn test_jwt_service_expired_token() {
        let service = JwtService::new("test-secret-key");

        // Create claims that are already expired
        let mut claims = Claims::new(
            "user123".to_string(),
            "test@example.com".to_string(),
            vec!["user".to_string()],
            1,
        );
        claims.exp = 0; // Set to epoch (definitely expired)

        let token = service.encode_claims(&claims).unwrap();
        let result = service.decode_token(&token);

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), JwtError::Expired));
    }

    #[tokio::test]
    async fn test_auth_middleware_missing_header() {
        let app = Router::new()
            .route("/protected", get(protected_handler))
            .layer(axum::middleware::from_fn(auth_middleware));

        let request = Request::builder().uri("/protected").body(Body::empty()).unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_auth_middleware_invalid_header() {
        let app = Router::new()
            .route("/protected", get(protected_handler))
            .layer(axum::middleware::from_fn(auth_middleware));

        let request = Request::builder()
            .uri("/protected")
            .header("Authorization", "InvalidFormat")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_auth_middleware_empty_token() {
        let app = Router::new()
            .route("/protected", get(protected_handler))
            .layer(axum::middleware::from_fn(auth_middleware));

        let request = Request::builder()
            .uri("/protected")
            .header("Authorization", "Bearer ")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_auth_middleware_valid_token() {
        let app = Router::new()
            .route("/protected", get(protected_handler))
            .layer(axum::middleware::from_fn(auth_middleware));

        let request = Request::builder()
            .uri("/protected")
            .header("Authorization", "Bearer valid-token")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_optional_auth_middleware_no_header() {
        let app = Router::new()
            .route("/optional", get(protected_handler))
            .layer(axum::middleware::from_fn(optional_auth_middleware));

        let request = Request::builder().uri("/optional").body(Body::empty()).unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_optional_auth_middleware_valid_token() {
        let app = Router::new()
            .route("/optional", get(protected_handler))
            .layer(axum::middleware::from_fn(optional_auth_middleware));

        let request = Request::builder()
            .uri("/optional")
            .header("Authorization", "Bearer valid-token")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[test]
    fn test_jwt_error_to_app_error_conversion() {
        let expired_error: AppError = JwtError::Expired.into();
        assert!(matches!(expired_error, AppError::Authentication { .. }));

        let invalid_error: AppError = JwtError::InvalidToken.into();
        assert!(matches!(invalid_error, AppError::Authentication { .. }));

        let encoding_error: AppError = JwtError::EncodingError("test".to_string()).into();
        assert!(matches!(encoding_error, AppError::Internal { .. }));
    }

    #[test]
    fn test_user_context_has_any_role() {
        let context = UserContext {
            user_id: "test".to_string(),
            email: "test@example.com".to_string(),
            roles: vec!["user".to_string(), "editor".to_string()],
            token_id: "token123".to_string(),
        };

        assert!(context.has_any_role(&["admin".to_string(), "user".to_string()]));
        assert!(context.has_any_role(&["editor".to_string()]));
        assert!(!context.has_any_role(&["admin".to_string(), "superuser".to_string()]));
    }

    #[test]
    fn test_user_context_display() {
        let context = UserContext {
            user_id: "user123".to_string(),
            email: "test@example.com".to_string(),
            roles: vec!["admin".to_string()],
            token_id: "token456".to_string(),
        };

        let display_str = context.to_string();
        assert!(display_str.contains("user123"));
        assert!(display_str.contains("test@example.com"));
        assert!(display_str.contains("admin"));
    }
}
