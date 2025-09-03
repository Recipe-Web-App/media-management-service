use crate::infrastructure::config::OAuth2Config;
use crate::infrastructure::oauth2::{
    models::{
        ClientCredentialsRequest, IntrospectionRequest, IntrospectionResponse, OAuth2ErrorResponse,
        TokenResponse,
    },
    CachedClientToken, CachedTokenInfo, TokenCache,
};
use anyhow::{anyhow, Context, Result};
use reqwest::Client as HttpClient;
use serde_json;
use serde_urlencoded;
use std::{sync::Arc, time::Duration};
use tokio::time::sleep;
use tracing::{debug, error, warn};

/// `OAuth2` client for token introspection and client credentials flow
#[derive(Clone)]
pub struct OAuth2Client {
    config: OAuth2Config,
    http_client: HttpClient,
    token_cache: Arc<TokenCache>,
}

#[derive(Debug, thiserror::Error)]
pub enum OAuth2Error {
    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),

    #[error("OAuth2 server error: {error} - {description}")]
    ServerError { error: String, description: String },

    #[error("Token is inactive")]
    InactiveToken,

    #[error("Invalid response format: {0}")]
    InvalidResponse(String),

    #[error("Service configuration error: {0}")]
    ConfigError(String),

    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),
}

impl OAuth2Client {
    pub fn new(config: OAuth2Config) -> Result<Self> {
        if config.service_base_url.is_empty() {
            return Err(anyhow!("OAuth2 service base URL is required"));
        }

        if config.enabled && config.client_id.is_empty() {
            return Err(anyhow!("OAuth2 client ID is required when service is enabled"));
        }

        let http_client = HttpClient::builder()
            .timeout(Duration::from_secs(config.request_timeout_seconds))
            .build()
            .context("Failed to create HTTP client")?;

        let token_cache = Arc::new(TokenCache::new(
            config.token_cache_ttl_seconds,
            config.client_credentials_cache_ttl_seconds,
        ));

        Ok(Self { config, http_client, token_cache })
    }

    /// Introspect a token with the `OAuth2` service
    pub async fn introspect_token(&self, token: &str) -> Result<CachedTokenInfo, OAuth2Error> {
        // Check cache first
        if let Some(cached) = self.token_cache.get_validation(token).await {
            debug!("Token validation cache hit");
            return Ok(cached);
        }

        debug!("Token validation cache miss, calling introspection endpoint");

        let request =
            IntrospectionRequest::new(token.to_string(), Some("access_token".to_string()));

        let introspection_url = format!("{}/oauth2/introspect", self.config.service_base_url);

        let mut retries = 0;
        while retries <= self.config.max_retries {
            let form_data = serde_urlencoded::to_string(&request).map_err(|e| {
                OAuth2Error::InvalidResponse(format!("Failed to encode form data: {e}"))
            })?;

            let response = self
                .http_client
                .post(&introspection_url)
                .header("Content-Type", "application/x-www-form-urlencoded")
                .basic_auth(&self.config.client_id, Some(&self.config.client_secret))
                .body(form_data)
                .send()
                .await;

            match response {
                Ok(resp) => {
                    if resp.status().is_success() {
                        let introspection_result: IntrospectionResponse =
                            resp.json().await.map_err(|e| {
                                OAuth2Error::InvalidResponse(format!(
                                    "Failed to parse response: {e}"
                                ))
                            })?;

                        let cached_info =
                            CachedTokenInfo::from_introspection(&introspection_result);

                        // Cache the result
                        self.token_cache
                            .cache_validation(token.to_string(), cached_info.clone())
                            .await;

                        if !cached_info.active {
                            return Err(OAuth2Error::InactiveToken);
                        }

                        return Ok(cached_info);
                    } else if resp.status().is_client_error() {
                        // Don't retry client errors
                        let error_text =
                            resp.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                        return Err(OAuth2Error::AuthenticationFailed(error_text));
                    }
                    // Server error, might be worth retrying
                    let status = resp.status();
                    let error_text =
                        resp.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                    warn!("Server error during introspection: {} - {}", status, error_text);

                    if retries >= self.config.max_retries {
                        return Err(OAuth2Error::ServerError {
                            error: status.to_string(),
                            description: error_text,
                        });
                    }
                }
                Err(e) => {
                    warn!("HTTP error during introspection (attempt {}): {}", retries + 1, e);
                    if retries >= self.config.max_retries {
                        return Err(OAuth2Error::HttpError(e));
                    }
                }
            }

            retries += 1;
            let delay = Duration::from_millis(self.config.retry_delay_ms * (1 << (retries - 1))); // Exponential backoff
            sleep(delay).await;
        }

        Err(OAuth2Error::AuthenticationFailed("Max retries exceeded".to_string()))
    }

    /// Get a client credentials token for service-to-service authentication
    pub async fn get_client_credentials_token(
        &self,
        scopes: &[String],
    ) -> Result<CachedClientToken, OAuth2Error> {
        if !self.config.service_to_service_enabled {
            return Err(OAuth2Error::ConfigError(
                "Service-to-service authentication is disabled".to_string(),
            ));
        }

        // Check cache first
        if let Some(cached) = self.token_cache.get_client_token(scopes).await {
            debug!("Client credentials cache hit for scopes: {:?}", scopes);
            return Ok(cached);
        }

        debug!("Client credentials cache miss, requesting new token for scopes: {:?}", scopes);

        let request =
            ClientCredentialsRequest::new(if scopes.is_empty() { None } else { Some(scopes) });
        let token_url = format!("{}/oauth2/token", self.config.service_base_url);

        let mut retries = 0;
        while retries <= self.config.max_retries {
            let form_data = serde_urlencoded::to_string(&request).map_err(|e| {
                OAuth2Error::InvalidResponse(format!("Failed to encode form data: {e}"))
            })?;

            let response = self
                .http_client
                .post(&token_url)
                .header("Content-Type", "application/x-www-form-urlencoded")
                .basic_auth(&self.config.client_id, Some(&self.config.client_secret))
                .body(form_data)
                .send()
                .await;

            match response {
                Ok(resp) => {
                    if resp.status().is_success() {
                        let token_response: TokenResponse = resp.json().await.map_err(|e| {
                            OAuth2Error::InvalidResponse(format!(
                                "Failed to parse token response: {e}"
                            ))
                        })?;

                        let cached_token =
                            CachedClientToken::from_token_response(&token_response, scopes);

                        // Cache the token
                        self.token_cache
                            .cache_client_token(scopes.to_vec(), cached_token.clone())
                            .await;

                        return Ok(cached_token);
                    } else if resp.status().is_client_error() {
                        // Try to parse OAuth2 error response
                        let response_text =
                            resp.text().await.unwrap_or_else(|_| "Unknown error".to_string());

                        // Try to parse as OAuth2 error first
                        if let Ok(oauth2_error) =
                            serde_json::from_str::<OAuth2ErrorResponse>(&response_text)
                        {
                            return Err(OAuth2Error::ServerError {
                                error: oauth2_error.error,
                                description: oauth2_error.error_description.unwrap_or_default(),
                            });
                        }
                        return Err(OAuth2Error::AuthenticationFailed(response_text));
                    }
                    // Server error, might be worth retrying
                    let status = resp.status();
                    let error_text =
                        resp.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                    warn!(
                        "Server error during client credentials request: {} - {}",
                        status, error_text
                    );

                    if retries >= self.config.max_retries {
                        return Err(OAuth2Error::ServerError {
                            error: status.to_string(),
                            description: error_text,
                        });
                    }
                }
                Err(e) => {
                    warn!(
                        "HTTP error during client credentials request (attempt {}): {}",
                        retries + 1,
                        e
                    );
                    if retries >= self.config.max_retries {
                        return Err(OAuth2Error::HttpError(e));
                    }
                }
            }

            retries += 1;
            let delay = Duration::from_millis(self.config.retry_delay_ms * (1 << (retries - 1))); // Exponential backoff
            sleep(delay).await;
        }

        Err(OAuth2Error::AuthenticationFailed("Max retries exceeded".to_string()))
    }

    /// Perform periodic cache cleanup
    pub async fn cleanup_cache(&self) {
        self.token_cache.cleanup_expired().await;
    }

    /// Check if the `OAuth2` service is healthy
    pub async fn health_check(&self) -> Result<bool, OAuth2Error> {
        let health_url = format!("{}/health", self.config.service_base_url);

        match self.http_client.get(&health_url).send().await {
            Ok(response) => Ok(response.status().is_success()),
            Err(e) => {
                warn!("OAuth2 service health check failed: {}", e);
                Err(OAuth2Error::HttpError(e))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_config() -> OAuth2Config {
        OAuth2Config {
            enabled: true,
            service_to_service_enabled: true,
            introspection_enabled: true,
            client_id: "test-client".to_string(),
            client_secret: "test-secret".to_string(),
            service_base_url: "http://localhost:8080/api/v1/auth".to_string(),
            jwt_secret: "test-jwt-secret".to_string(),
            token_cache_ttl_seconds: 300,
            client_credentials_cache_ttl_seconds: 1800,
            request_timeout_seconds: 10,
            max_retries: 3,
            retry_delay_ms: 1000,
        }
    }

    #[test]
    fn test_oauth2_client_creation() {
        let config = create_test_config();
        let client = OAuth2Client::new(config);
        assert!(client.is_ok());
    }

    #[test]
    fn test_oauth2_client_creation_missing_base_url() {
        let mut config = create_test_config();
        config.service_base_url = String::new();

        let client = OAuth2Client::new(config);
        assert!(client.is_err());
    }

    #[test]
    fn test_oauth2_client_creation_missing_client_id() {
        let mut config = create_test_config();
        config.client_id = String::new();

        let client = OAuth2Client::new(config);
        assert!(client.is_err());
    }

    #[tokio::test]
    async fn test_token_cache_validation() {
        let cache = TokenCache::new(300, 1800);

        let token_info = CachedTokenInfo {
            active: true,
            client_id: Some("test-client".to_string()),
            username: Some("test-user".to_string()),
            scopes: vec!["read".to_string(), "write".to_string()],
            token_type: Some("Bearer".to_string()),
            expires_at: Some(chrono::Utc::now() + chrono::Duration::hours(1)),
            subject: Some("test-user".to_string()),
            audience: Some(vec!["test-client".to_string()]),
            issuer: Some("auth-service".to_string()),
            cached_at: chrono::Utc::now(),
        };

        cache.cache_validation("test-token".to_string(), token_info.clone()).await;

        let cached = cache.get_validation("test-token").await;
        assert!(cached.is_some());
        assert_eq!(cached.unwrap().client_id, token_info.client_id);
    }

    #[tokio::test]
    async fn test_client_token_cache() {
        let cache = TokenCache::new(300, 1800);

        let scopes = vec!["read".to_string(), "write".to_string()];
        let client_token = CachedClientToken {
            access_token: "test-access-token".to_string(),
            token_type: "Bearer".to_string(),
            expires_at: chrono::Utc::now() + chrono::Duration::hours(1),
            scopes: scopes.clone(),
        };

        cache.cache_client_token(scopes.clone(), client_token.clone()).await;

        let cached = cache.get_client_token(&scopes).await;
        assert!(cached.is_some());
        assert_eq!(cached.unwrap().access_token, client_token.access_token);
    }
}
