use axum::extract::FromRequestParts;
use axum::extract::{Request, State};
use axum::http::header::AUTHORIZATION;
use axum::http::{HeaderMap, request::Parts};
use axum::middleware::Next;
use axum::response::Response;
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode};
use reqwest::Client;
use serde::Deserialize;
use uuid::Uuid;

use crate::config::AuthModeConfig;
use crate::error::AppError;
use crate::state::AppState;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub enum AuthMode {
    OAuth2 {
        client: Client,
        base_url: String,
        client_id: String,
        client_secret: String,
    },
    Jwt {
        decoding_key: DecodingKey,
    },
    Dev,
}

#[derive(Clone, Debug)]
pub struct AuthUser {
    pub user_id: Uuid,
    pub scopes: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct Claims {
    sub: String,
    #[serde(default)]
    scope: Option<String>,
}

#[derive(Debug, Deserialize)]
struct IntrospectionResponse {
    active: bool,
    sub: Option<String>,
    scope: Option<String>,
}

// ---------------------------------------------------------------------------
// AuthMode construction
// ---------------------------------------------------------------------------

impl AuthMode {
    pub fn from_config(config: &AuthModeConfig) -> Self {
        match config {
            AuthModeConfig::OAuth2 {
                base_url,
                client_id,
                client_secret,
            } => Self::OAuth2 {
                client: Client::new(),
                base_url: base_url.clone(),
                client_id: client_id.clone(),
                client_secret: client_secret.clone(),
            },
            AuthModeConfig::Jwt { secret } => Self::Jwt {
                decoding_key: DecodingKey::from_secret(secret.as_bytes()),
            },
            AuthModeConfig::Dev => Self::Dev,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::OAuth2 { .. } => "oauth2",
            Self::Jwt { .. } => "jwt",
            Self::Dev => "dev",
        }
    }
}

// ---------------------------------------------------------------------------
// Middleware
// ---------------------------------------------------------------------------

pub async fn auth_middleware(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Result<Response, AppError> {
    let auth_user = authenticate(&state.auth_mode, request.headers()).await?;
    request.extensions_mut().insert(auth_user);
    Ok(next.run(request).await)
}

// ---------------------------------------------------------------------------
// Core authentication (also callable from handlers for dual-auth)
// ---------------------------------------------------------------------------

pub async fn authenticate(auth_mode: &AuthMode, headers: &HeaderMap) -> Result<AuthUser, AppError> {
    match auth_mode {
        AuthMode::Dev => authenticate_dev(headers),
        AuthMode::Jwt { decoding_key } => authenticate_jwt(headers, decoding_key),
        AuthMode::OAuth2 {
            client,
            base_url,
            client_id,
            client_secret,
        } => authenticate_oauth2(headers, client, base_url, client_id, client_secret).await,
    }
}

// ---------------------------------------------------------------------------
// Auth strategies
// ---------------------------------------------------------------------------

fn extract_bearer_token(headers: &HeaderMap) -> Result<&str, AppError> {
    let header = headers
        .get(AUTHORIZATION)
        .ok_or_else(|| AppError::Unauthorized("missing authorization header".into()))?
        .to_str()
        .map_err(|_| AppError::Unauthorized("invalid authorization header".into()))?;

    header
        .strip_prefix("Bearer ")
        .ok_or_else(|| AppError::Unauthorized("invalid bearer token format".into()))
}

fn authenticate_dev(headers: &HeaderMap) -> Result<AuthUser, AppError> {
    let value = headers
        .get("x-user-id")
        .ok_or_else(|| AppError::Unauthorized("missing x-user-id header".into()))?
        .to_str()
        .map_err(|_| AppError::Unauthorized("invalid x-user-id header".into()))?;

    let user_id = Uuid::parse_str(value)
        .map_err(|_| AppError::Unauthorized("x-user-id is not a valid UUID".into()))?;

    Ok(AuthUser {
        user_id,
        scopes: vec!["media:read".into(), "media:write".into()],
    })
}

fn authenticate_jwt(headers: &HeaderMap, decoding_key: &DecodingKey) -> Result<AuthUser, AppError> {
    let token = extract_bearer_token(headers)?;

    let mut validation = Validation::new(Algorithm::HS256);
    validation.set_issuer(&["auth-service"]);
    validation.set_required_spec_claims(&["sub", "exp", "iss"]);

    let token_data = decode::<Claims>(token, decoding_key, &validation)
        .map_err(|e| AppError::Unauthorized(format!("invalid token: {e}")))?;

    let user_id = Uuid::parse_str(&token_data.claims.sub)
        .map_err(|_| AppError::Unauthorized("sub claim is not a valid UUID".into()))?;

    let scopes = token_data
        .claims
        .scope
        .map(|s| s.split_whitespace().map(String::from).collect())
        .unwrap_or_default();

    Ok(AuthUser { user_id, scopes })
}

async fn authenticate_oauth2(
    headers: &HeaderMap,
    client: &Client,
    base_url: &str,
    client_id: &str,
    client_secret: &str,
) -> Result<AuthUser, AppError> {
    let token = extract_bearer_token(headers)?;

    let response = client
        .post(format!("{base_url}/oauth2/introspect"))
        .basic_auth(client_id, Some(client_secret))
        .form(&[("token", token)])
        .send()
        .await
        .map_err(|e| AppError::ServiceUnavailable(format!("auth service request failed: {e}")))?;

    if !response.status().is_success() {
        return Err(AppError::ServiceUnavailable(
            "auth service returned an error".into(),
        ));
    }

    let body: IntrospectionResponse = response
        .json()
        .await
        .map_err(|e| AppError::Internal(format!("auth service response parse error: {e}")))?;

    if !body.active {
        return Err(AppError::Unauthorized("token is not active".into()));
    }

    let sub = body
        .sub
        .ok_or_else(|| AppError::Unauthorized("missing sub in introspection response".into()))?;

    let user_id = Uuid::parse_str(&sub)
        .map_err(|_| AppError::Unauthorized("sub is not a valid UUID".into()))?;

    let scopes: Vec<String> = body
        .scope
        .map(|s| s.split_whitespace().map(String::from).collect())
        .unwrap_or_default();

    Ok(AuthUser { user_id, scopes })
}

// ---------------------------------------------------------------------------
// Extractor
// ---------------------------------------------------------------------------

impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<Self>()
            .cloned()
            .ok_or_else(|| AppError::Unauthorized("not authenticated".into()))
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;
    use jsonwebtoken::{EncodingKey, Header};
    use serde::Serialize;

    const TEST_SECRET: &str = "test-secret-key-for-unit-tests";

    fn dev_mode() -> AuthMode {
        AuthMode::Dev
    }

    fn jwt_mode() -> AuthMode {
        AuthMode::Jwt {
            decoding_key: DecodingKey::from_secret(TEST_SECRET.as_bytes()),
        }
    }

    #[derive(Debug, Serialize)]
    struct TestClaims {
        sub: String,
        iss: String,
        exp: u64,
        #[serde(skip_serializing_if = "Option::is_none")]
        scope: Option<String>,
    }

    fn make_jwt(claims: &TestClaims) -> String {
        let key = EncodingKey::from_secret(TEST_SECRET.as_bytes());
        jsonwebtoken::encode(&Header::new(Algorithm::HS256), claims, &key).unwrap()
    }

    fn future_exp() -> u64 {
        (std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs())
            + 3600
    }

    fn past_exp() -> u64 {
        (std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs())
        .saturating_sub(3600)
    }

    // -- Dev mode tests --

    #[tokio::test]
    async fn dev_mode_valid_uuid() {
        let mut headers = HeaderMap::new();
        let uid = Uuid::new_v4();
        headers.insert(
            "x-user-id",
            HeaderValue::from_str(&uid.to_string()).unwrap(),
        );

        let result = authenticate(&dev_mode(), &headers).await;
        let user = result.unwrap();
        assert_eq!(user.user_id, uid);
        assert_eq!(user.scopes, vec!["media:read", "media:write"]);
    }

    #[tokio::test]
    async fn dev_mode_missing_header() {
        let headers = HeaderMap::new();
        let result = authenticate(&dev_mode(), &headers).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn dev_mode_invalid_uuid() {
        let mut headers = HeaderMap::new();
        headers.insert("x-user-id", HeaderValue::from_static("not-a-uuid"));

        let result = authenticate(&dev_mode(), &headers).await;
        assert!(result.is_err());
    }

    // -- JWT mode tests --

    #[tokio::test]
    async fn jwt_mode_valid_token() {
        let uid = Uuid::new_v4();
        let claims = TestClaims {
            sub: uid.to_string(),
            iss: "auth-service".into(),
            exp: future_exp(),
            scope: Some("media:read media:write".into()),
        };
        let token = make_jwt(&claims);

        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {token}")).unwrap(),
        );

        let result = authenticate(&jwt_mode(), &headers).await;
        let user = result.unwrap();
        assert_eq!(user.user_id, uid);
        assert_eq!(user.scopes, vec!["media:read", "media:write"]);
    }

    #[tokio::test]
    async fn jwt_mode_expired_token() {
        let claims = TestClaims {
            sub: Uuid::new_v4().to_string(),
            iss: "auth-service".into(),
            exp: past_exp(),
            scope: None,
        };
        let token = make_jwt(&claims);

        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {token}")).unwrap(),
        );

        let result = authenticate(&jwt_mode(), &headers).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn jwt_mode_wrong_issuer() {
        let claims = TestClaims {
            sub: Uuid::new_v4().to_string(),
            iss: "wrong-issuer".into(),
            exp: future_exp(),
            scope: None,
        };
        let token = make_jwt(&claims);

        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {token}")).unwrap(),
        );

        let result = authenticate(&jwt_mode(), &headers).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn jwt_mode_missing_bearer() {
        let headers = HeaderMap::new();
        let result = authenticate(&jwt_mode(), &headers).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn jwt_mode_no_scope_claim() {
        let uid = Uuid::new_v4();
        let claims = TestClaims {
            sub: uid.to_string(),
            iss: "auth-service".into(),
            exp: future_exp(),
            scope: None,
        };
        let token = make_jwt(&claims);

        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {token}")).unwrap(),
        );

        let result = authenticate(&jwt_mode(), &headers).await;
        let user = result.unwrap();
        assert_eq!(user.user_id, uid);
        assert!(user.scopes.is_empty());
    }

    // -- Bearer extraction tests --

    #[test]
    fn bearer_extraction_valid() {
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_static("Bearer my-token-123"),
        );
        let token = extract_bearer_token(&headers).unwrap();
        assert_eq!(token, "my-token-123");
    }

    #[test]
    fn bearer_extraction_missing() {
        let headers = HeaderMap::new();
        assert!(extract_bearer_token(&headers).is_err());
    }

    #[test]
    fn bearer_extraction_wrong_scheme() {
        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, HeaderValue::from_static("Basic abc123"));
        assert!(extract_bearer_token(&headers).is_err());
    }

    // -- OAuth2 mode tests --

    #[tokio::test]
    async fn oauth2_mode_active_token() {
        let server = wiremock::MockServer::start().await;
        let uid = Uuid::new_v4();

        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::path("/oauth2/introspect"))
            .respond_with(
                wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "active": true,
                    "sub": uid.to_string(),
                    "scope": "media:read media:write",
                })),
            )
            .mount(&server)
            .await;

        let mode = AuthMode::OAuth2 {
            client: Client::new(),
            base_url: server.uri(),
            client_id: "test-client".into(),
            client_secret: "test-secret".into(),
        };

        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_static("Bearer some-opaque-token"),
        );

        let user = authenticate(&mode, &headers).await.unwrap();
        assert_eq!(user.user_id, uid);
        assert_eq!(user.scopes, vec!["media:read", "media:write"]);
    }

    #[tokio::test]
    async fn oauth2_mode_inactive_token() {
        let server = wiremock::MockServer::start().await;

        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::path("/oauth2/introspect"))
            .respond_with(
                wiremock::ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({ "active": false })),
            )
            .mount(&server)
            .await;

        let mode = AuthMode::OAuth2 {
            client: Client::new(),
            base_url: server.uri(),
            client_id: "test-client".into(),
            client_secret: "test-secret".into(),
        };

        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_static("Bearer expired-token"),
        );

        let result = authenticate(&mode, &headers).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn oauth2_mode_service_error() {
        let server = wiremock::MockServer::start().await;

        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::path("/oauth2/introspect"))
            .respond_with(wiremock::ResponseTemplate::new(500))
            .mount(&server)
            .await;

        let mode = AuthMode::OAuth2 {
            client: Client::new(),
            base_url: server.uri(),
            client_id: "test-client".into(),
            client_secret: "test-secret".into(),
        };

        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, HeaderValue::from_static("Bearer some-token"));

        let result = authenticate(&mode, &headers).await;
        assert!(result.is_err());
    }
}
