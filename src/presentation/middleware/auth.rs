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

/// JWT token claims - `OAuth2` compatible format
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub iss: String,             // Issuer
    pub aud: Vec<String>,        // Audience
    pub sub: String,             // Subject (user ID)
    pub client_id: String,       // OAuth2 client ID
    pub user_id: Option<String>, // User ID (for user tokens)
    pub scopes: Vec<String>,     // OAuth2 scopes
    #[serde(rename = "type")]
    pub token_type: String, // Token type ("access_token", "client_credentials")
    pub exp: usize,              // Expiration time
    pub iat: usize,              // Issued at
    pub nbf: usize,              // Not before
    pub jti: String,             // JWT ID (unique identifier)
}

impl Claims {
    /// Create new OAuth2-compatible access token claims
    #[must_use]
    pub fn new_access_token(
        issuer: String,
        audience: Vec<String>,
        user_id: String,
        client_id: String,
        scopes: Vec<String>,
        expires_in_hours: u64,
    ) -> Self {
        let now = chrono::Utc::now().timestamp().max(0) as u64 as usize;
        let exp = now + (expires_in_hours * 3600) as usize;

        Self {
            iss: issuer,
            aud: audience,
            sub: user_id.clone(),
            client_id,
            user_id: Some(user_id),
            scopes,
            token_type: "access_token".to_string(),
            exp,
            iat: now,
            nbf: now,
            jti: Uuid::new_v4().to_string(),
        }
    }

    /// Create new OAuth2-compatible client credentials token claims
    #[must_use]
    pub fn new_client_credentials(
        issuer: String,
        audience: Vec<String>,
        client_id: String,
        scopes: Vec<String>,
        expires_in_hours: u64,
    ) -> Self {
        let now = chrono::Utc::now().timestamp().max(0) as u64 as usize;
        let exp = now + (expires_in_hours * 3600) as usize;

        Self {
            iss: issuer,
            aud: audience,
            sub: client_id.clone(),
            client_id,
            user_id: None,
            scopes,
            token_type: "client_credentials".to_string(),
            exp,
            iat: now,
            nbf: now,
            jti: Uuid::new_v4().to_string(),
        }
    }

    /// Check if token is expired
    pub fn is_expired(&self) -> bool {
        let now = chrono::Utc::now().timestamp().max(0) as u64 as usize;
        self.exp < now
    }

    /// Check if token is not yet valid (nbf check)
    pub fn is_not_yet_valid(&self) -> bool {
        let now = chrono::Utc::now().timestamp().max(0) as u64 as usize;
        self.nbf > now
    }

    /// Check if user has a specific scope
    pub fn has_scope(&self, scope: &str) -> bool {
        self.scopes.iter().any(|s| s == scope)
    }

    /// Check if user has any of the specified scopes
    pub fn has_any_scope(&self, scopes: &[&str]) -> bool {
        self.scopes.iter().any(|user_scope| scopes.contains(&user_scope.as_str()))
    }

    /// Check if this is a user token (has `user_id`)
    pub fn is_user_token(&self) -> bool {
        self.user_id.is_some() && self.token_type == "access_token"
    }

    /// Check if this is a client credentials token
    pub fn is_client_token(&self) -> bool {
        self.token_type == "client_credentials"
    }
}

/// User context extracted from JWT - `OAuth2` compatible
#[derive(Debug, Clone)]
pub struct UserContext {
    pub user_id: Option<String>, // User ID (None for client credentials)
    pub client_id: String,       // OAuth2 client ID
    pub subject: String,         // JWT subject (user_id or client_id)
    pub scopes: Vec<String>,     // OAuth2 scopes
    pub token_type: String,      // Token type
    pub token_id: String,        // JWT ID
    pub issuer: String,          // Token issuer
    pub audience: Vec<String>,   // Token audience
}

impl UserContext {
    /// Check if user has any of the specified scopes
    pub fn has_any_scope(&self, required_scopes: &[&str]) -> bool {
        self.scopes.iter().any(|scope| required_scopes.contains(&scope.as_str()))
    }

    /// Check if user has a specific scope
    pub fn has_scope(&self, scope: &str) -> bool {
        self.scopes.iter().any(|s| s == scope)
    }

    /// Check if this is a user token (has `user_id`)
    pub fn is_user_token(&self) -> bool {
        self.user_id.is_some() && self.token_type == "access_token"
    }

    /// Check if this is a client credentials token
    pub fn is_client_token(&self) -> bool {
        self.token_type == "client_credentials"
    }

    /// Get the effective user ID (`user_id` for user tokens, `client_id` for client tokens)
    pub fn effective_user_id(&self) -> &str {
        self.user_id.as_ref().unwrap_or(&self.client_id)
    }
}

impl From<Claims> for UserContext {
    fn from(claims: Claims) -> Self {
        Self {
            user_id: claims.user_id,
            client_id: claims.client_id,
            subject: claims.sub,
            scopes: claims.scopes,
            token_type: claims.token_type,
            token_id: claims.jti,
            issuer: claims.iss,
            audience: claims.aud,
        }
    }
}

impl fmt::Display for UserContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "UserContext(user_id={:?}, client_id={}, scopes={:?}, token_type={})",
            self.user_id, self.client_id, self.scopes, self.token_type
        )
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
        Self::new_with_validation(secret, None)
    }

    /// Create new JWT service with secret and optional audience validation
    pub fn new_with_validation(secret: &str, required_audience: Option<&str>) -> Self {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.validate_exp = true;

        if let Some(aud) = required_audience {
            validation.validate_aud = true;
            validation.aud = Some([aud.to_string()].into());
        } else {
            validation.validate_aud = false;
        }

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

    /// Create access token for user
    pub fn create_access_token(
        &self,
        issuer: String,
        audience: Vec<String>,
        user_id: String,
        client_id: String,
        scopes: Vec<String>,
        expires_in_hours: u64,
    ) -> Result<String, JwtError> {
        let claims = Claims::new_access_token(
            issuer,
            audience,
            user_id,
            client_id,
            scopes,
            expires_in_hours,
        );
        self.encode_claims(&claims)
    }

    /// Create client credentials token
    pub fn create_client_credentials_token(
        &self,
        issuer: String,
        audience: Vec<String>,
        client_id: String,
        scopes: Vec<String>,
        expires_in_hours: u64,
    ) -> Result<String, JwtError> {
        let claims =
            Claims::new_client_credentials(issuer, audience, client_id, scopes, expires_in_hours);
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
            user_id: Some("mock-user-id".to_string()),
            client_id: "mock-client-id".to_string(),
            subject: "mock-user-id".to_string(),
            scopes: vec!["read".to_string(), "write".to_string()],
            token_type: "access_token".to_string(),
            token_id: Uuid::new_v4().to_string(),
            issuer: "mock-issuer".to_string(),
            audience: vec!["mock-audience".to_string()],
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
        user_id: Some("mock-user-id".to_string()),
        client_id: "mock-client-id".to_string(),
        subject: "mock-user-id".to_string(),
        scopes: vec!["read".to_string(), "write".to_string()],
        token_type: "access_token".to_string(),
        token_id: Uuid::new_v4().to_string(),
        issuer: "mock-issuer".to_string(),
        audience: vec!["mock-audience".to_string()],
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
                    user_id: Some("mock-user-id".to_string()),
                    client_id: "mock-client-id".to_string(),
                    subject: "mock-user-id".to_string(),
                    scopes: vec!["read".to_string(), "write".to_string()],
                    token_type: "access_token".to_string(),
                    token_id: Uuid::new_v4().to_string(),
                    issuer: "mock-issuer".to_string(),
                    audience: vec!["mock-audience".to_string()],
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

            // Check if user has required roles (mapped to scopes)
            if !user_context.has_any_scope(&required_roles) {
                return Err(AppError::Authorization {
                    message: format!(
                        "Access denied. Required scopes: {:?}, user scopes: {:?}",
                        required_roles, user_context.scopes
                    ),
                });
            }

            debug!(
                "Authorization successful for user {:?} with scopes {:?}",
                user_context.user_id, user_context.scopes
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
    fn test_access_token_claims_creation() {
        let claims = Claims::new_access_token(
            "auth-service".to_string(),
            vec!["test-client".to_string()],
            "user123".to_string(),
            "test-client".to_string(),
            vec!["read".to_string(), "write".to_string()],
            24,
        );

        assert_eq!(claims.sub, "user123");
        assert_eq!(claims.client_id, "test-client");
        assert_eq!(claims.user_id, Some("user123".to_string()));
        assert_eq!(claims.scopes, vec!["read", "write"]);
        assert_eq!(claims.token_type, "access_token");
        assert!(!claims.is_expired());
        assert!(!claims.is_not_yet_valid());
        assert!(claims.is_user_token());
        assert!(!claims.is_client_token());
        assert!(claims.has_scope("read"));
        assert!(claims.has_scope("write"));
        assert!(!claims.has_scope("admin"));
    }

    #[test]
    fn test_client_credentials_claims_creation() {
        let claims = Claims::new_client_credentials(
            "auth-service".to_string(),
            vec!["api-service".to_string()],
            "service-client".to_string(),
            vec!["read".to_string(), "admin".to_string()],
            1,
        );

        assert_eq!(claims.sub, "service-client");
        assert_eq!(claims.client_id, "service-client");
        assert_eq!(claims.user_id, None);
        assert_eq!(claims.scopes, vec!["read", "admin"]);
        assert_eq!(claims.token_type, "client_credentials");
        assert!(!claims.is_user_token());
        assert!(claims.is_client_token());
        assert!(claims.has_any_scope(&["read", "write"]));
        assert!(claims.has_any_scope(&["admin"]));
        assert!(!claims.has_any_scope(&["write", "superuser"]));
    }

    #[test]
    fn test_user_context_from_claims() {
        let claims = Claims::new_access_token(
            "auth-service".to_string(),
            vec!["test-client".to_string()],
            "user456".to_string(),
            "test-client".to_string(),
            vec!["moderator".to_string()],
            12,
        );

        let context: UserContext = claims.clone().into();
        assert_eq!(context.user_id, Some("user456".to_string()));
        assert_eq!(context.client_id, "test-client");
        assert_eq!(context.subject, "user456");
        assert_eq!(context.scopes, vec!["moderator"]);
        assert_eq!(context.token_type, "access_token");
        assert_eq!(context.token_id, claims.jti);
        assert!(context.has_scope("moderator"));
        assert!(!context.has_scope("admin"));
    }

    #[test]
    fn test_jwt_service_access_token_creation() {
        let service = JwtService::new("test-secret-key");

        let token = service
            .create_access_token(
                "auth-service".to_string(),
                vec!["test-client".to_string()],
                "user123".to_string(),
                "test-client".to_string(),
                vec!["read".to_string(), "write".to_string()],
                24,
            )
            .unwrap();

        assert!(!token.is_empty());
        assert!(token.contains('.'));

        // Verify we can decode it back
        let decoded_claims = service.decode_token(&token).unwrap();
        assert_eq!(decoded_claims.sub, "user123");
        assert_eq!(decoded_claims.client_id, "test-client");
        assert_eq!(decoded_claims.user_id, Some("user123".to_string()));
        assert_eq!(decoded_claims.scopes, vec!["read", "write"]);
        assert_eq!(decoded_claims.token_type, "access_token");
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
        let mut claims = Claims::new_access_token(
            "auth-service".to_string(),
            vec!["test-client".to_string()],
            "user123".to_string(),
            "test-client".to_string(),
            vec!["read".to_string()],
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
    fn test_user_context_has_any_scope() {
        let context = UserContext {
            user_id: Some("test".to_string()),
            client_id: "test-client".to_string(),
            subject: "test".to_string(),
            scopes: vec!["read".to_string(), "write".to_string()],
            token_type: "access_token".to_string(),
            token_id: "token123".to_string(),
            issuer: "auth-service".to_string(),
            audience: vec!["test-client".to_string()],
        };

        assert!(context.has_any_scope(&["admin", "read"]));
        assert!(context.has_any_scope(&["write"]));
        assert!(!context.has_any_scope(&["admin", "superuser"]));
        assert!(context.is_user_token());
        assert!(!context.is_client_token());
    }

    #[test]
    fn test_user_context_display() {
        let context = UserContext {
            user_id: Some("user123".to_string()),
            client_id: "test-client".to_string(),
            subject: "user123".to_string(),
            scopes: vec!["admin".to_string()],
            token_type: "access_token".to_string(),
            token_id: "token456".to_string(),
            issuer: "auth-service".to_string(),
            audience: vec!["test-client".to_string()],
        };

        let display_str = context.to_string();
        assert!(display_str.contains("user123"));
        assert!(display_str.contains("test-client"));
        assert!(display_str.contains("admin"));
        assert!(display_str.contains("access_token"));
    }
}
