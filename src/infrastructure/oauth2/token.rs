use chrono::{DateTime, Utc};
#[allow(unused_imports)]
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::RwLock;

/// Cached token validation result
#[derive(Debug, Clone)]
pub struct CachedTokenInfo {
    pub active: bool,
    pub client_id: Option<String>,
    pub username: Option<String>,
    pub scopes: Vec<String>,
    pub token_type: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub subject: Option<String>,
    pub audience: Option<Vec<String>>,
    pub issuer: Option<String>,
    pub cached_at: DateTime<Utc>,
}

/// Client credentials token cache entry
#[derive(Debug, Clone)]
pub struct CachedClientToken {
    pub access_token: String,
    pub token_type: String,
    pub expires_at: DateTime<Utc>,
    pub scopes: Vec<String>,
}

/// Token cache for validation results and client credentials
pub struct TokenCache {
    validation_cache: RwLock<HashMap<String, CachedTokenInfo>>,
    client_cache: RwLock<HashMap<String, CachedClientToken>>, // Key: scope combination
    validation_ttl_seconds: u64,
    #[allow(dead_code)]
    client_token_ttl_seconds: u64,
}

impl TokenCache {
    pub fn new(validation_ttl_seconds: u64, client_token_ttl_seconds: u64) -> Self {
        Self {
            validation_cache: RwLock::new(HashMap::new()),
            client_cache: RwLock::new(HashMap::new()),
            validation_ttl_seconds,
            client_token_ttl_seconds,
        }
    }

    /// Get cached token validation result
    pub async fn get_validation(&self, token: &str) -> Option<CachedTokenInfo> {
        let cache = self.validation_cache.read().await;
        if let Some(cached) = cache.get(token) {
            let now = Utc::now();
            let cache_expires =
                cached.cached_at + chrono::Duration::seconds(self.validation_ttl_seconds as i64);

            if now < cache_expires {
                Some(cached.clone())
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Cache token validation result
    pub async fn cache_validation(&self, token: String, info: CachedTokenInfo) {
        let mut cache = self.validation_cache.write().await;
        cache.insert(token, info);
    }

    /// Get cached client credentials token
    pub async fn get_client_token(&self, scopes: &[String]) -> Option<CachedClientToken> {
        let scope_key = scopes.join(" ");
        let cache = self.client_cache.read().await;

        if let Some(cached) = cache.get(&scope_key) {
            let now = Utc::now();
            // Add 60-second buffer before expiry to avoid edge cases
            let expires_with_buffer = cached.expires_at - chrono::Duration::seconds(60);

            if now < expires_with_buffer {
                Some(cached.clone())
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Cache client credentials token
    pub async fn cache_client_token(&self, scopes: Vec<String>, token: CachedClientToken) {
        let scope_key = scopes.join(" ");
        let mut cache = self.client_cache.write().await;
        cache.insert(scope_key, token);
    }

    /// Clear expired entries from both caches
    pub async fn cleanup_expired(&self) {
        let now = Utc::now();

        // Clean validation cache
        {
            let mut cache = self.validation_cache.write().await;
            cache.retain(|_, cached| {
                let cache_expires = cached.cached_at
                    + chrono::Duration::seconds(self.validation_ttl_seconds as i64);
                now < cache_expires
            });
        }

        // Clean client token cache
        {
            let mut cache = self.client_cache.write().await;
            cache.retain(|_, cached| now < cached.expires_at);
        }
    }
}

impl CachedTokenInfo {
    pub fn from_introspection(
        response: &crate::infrastructure::oauth2::models::IntrospectionResponse,
    ) -> Self {
        let scopes = response
            .scope
            .as_ref()
            .map(|s| s.split_whitespace().map(String::from).collect())
            .unwrap_or_default();

        let expires_at =
            response.exp.map(|exp| DateTime::from_timestamp(exp, 0).unwrap_or_else(Utc::now));

        Self {
            active: response.active,
            client_id: response.client_id.clone(),
            username: response.username.clone(),
            scopes,
            token_type: response.token_type.clone(),
            expires_at,
            subject: response.sub.clone(),
            audience: response.aud.clone(),
            issuer: response.iss.clone(),
            cached_at: Utc::now(),
        }
    }
}

impl CachedClientToken {
    pub fn from_token_response(
        response: &crate::infrastructure::oauth2::models::TokenResponse,
        requested_scopes: &[String],
    ) -> Self {
        let expires_at = Utc::now() + chrono::Duration::seconds(response.expires_in);

        let scopes = response.scope.as_ref().map_or_else(
            || requested_scopes.to_vec(),
            |s| s.split_whitespace().map(String::from).collect(),
        );

        Self {
            access_token: response.access_token.clone(),
            token_type: response.token_type.clone(),
            expires_at,
            scopes,
        }
    }
}
