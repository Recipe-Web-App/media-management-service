use anyhow::Result;
use chrono::{Duration, Utc};
use media_management_service::infrastructure::{
    config::OAuth2Config,
    oauth2::{CachedClientToken, CachedTokenInfo, OAuth2Client, OAuth2Error},
};
use serde_json::json;
use wiremock::{
    matchers::{body_string_contains, header, method, path},
    Mock, MockServer, ResponseTemplate,
};

fn create_test_oauth2_config(base_url: &str) -> OAuth2Config {
    OAuth2Config {
        enabled: true,
        service_to_service_enabled: true,
        introspection_enabled: true,
        client_id: "test-client-id".to_string(),
        client_secret: "test-client-secret".to_string(),
        service_base_url: base_url.to_string(),
        jwt_secret: "test-jwt-secret-at-least-32-chars-long".to_string(), // gitleaks:allow
        token_cache_ttl_seconds: 300,
        client_credentials_cache_ttl_seconds: 1800,
        request_timeout_seconds: 10,
        max_retries: 3,
        retry_delay_ms: 100,
    }
}

#[tokio::test]
async fn test_oauth2_client_creation() {
    let config = create_test_oauth2_config("http://localhost:8080/api/v1/auth");
    let client = OAuth2Client::new(config);
    assert!(client.is_ok());
}

#[tokio::test]
async fn test_oauth2_client_creation_missing_base_url() {
    let mut config = create_test_oauth2_config("");
    config.service_base_url = String::new();

    let result = OAuth2Client::new(config);
    assert!(result.is_err());
    let error_msg = format!("{}", result.err().unwrap());
    assert!(error_msg.contains("OAuth2 service base URL is required"));
}

#[tokio::test]
async fn test_oauth2_client_creation_missing_client_id() {
    let mut config = create_test_oauth2_config("http://localhost:8080");
    config.client_id = String::new();

    let result = OAuth2Client::new(config);
    assert!(result.is_err());
    let error_msg = format!("{}", result.err().unwrap());
    assert!(error_msg.contains("OAuth2 client ID is required when service is enabled"));
}

#[tokio::test]
async fn test_successful_token_introspection() -> Result<()> {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/oauth2/introspect"))
        .and(header("content-type", "application/x-www-form-urlencoded"))
        .and(body_string_contains("token=valid-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "active": true,
            "client_id": "test-client",
            "username": "test-user",
            "scope": "read write",
            "token_type": "Bearer",
            "exp": (Utc::now() + Duration::hours(1)).timestamp(),
            "sub": "test-user-id",
            "aud": ["test-service"],
            "iss": "auth-service"
        })))
        .mount(&mock_server)
        .await;

    let config = create_test_oauth2_config(&mock_server.uri());
    let client = OAuth2Client::new(config)?;

    let result = client.introspect_token("valid-token").await;
    assert!(result.is_ok());

    let token_info = result.unwrap();
    assert!(token_info.active);
    assert_eq!(token_info.client_id, Some("test-client".to_string()));
    assert_eq!(token_info.username, Some("test-user".to_string()));
    assert_eq!(token_info.scopes, vec!["read".to_string(), "write".to_string()]);

    Ok(())
}

#[tokio::test]
async fn test_inactive_token_introspection() -> Result<()> {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/oauth2/introspect"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "active": false
        })))
        .mount(&mock_server)
        .await;

    let config = create_test_oauth2_config(&mock_server.uri());
    let client = OAuth2Client::new(config)?;

    let result = client.introspect_token("invalid-token").await;
    assert!(result.is_err());

    match result.unwrap_err() {
        OAuth2Error::InactiveToken => {}
        other => panic!("Expected InactiveToken error, got: {:?}", other),
    }

    Ok(())
}

#[tokio::test]
async fn test_client_error_during_introspection() -> Result<()> {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/oauth2/introspect"))
        .respond_with(ResponseTemplate::new(401).set_body_string("Unauthorized"))
        .mount(&mock_server)
        .await;

    let config = create_test_oauth2_config(&mock_server.uri());
    let client = OAuth2Client::new(config)?;

    let result = client.introspect_token("bad-token").await;
    assert!(result.is_err());

    match result.unwrap_err() {
        OAuth2Error::AuthenticationFailed(_) => {}
        other => panic!("Expected AuthenticationFailed error, got: {:?}", other),
    }

    Ok(())
}

#[tokio::test]
async fn test_server_error_with_retry_during_introspection() -> Result<()> {
    let mock_server = MockServer::start().await;

    // First two requests return server error, third succeeds
    Mock::given(method("POST"))
        .and(path("/oauth2/introspect"))
        .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
        .up_to_n_times(2)
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/oauth2/introspect"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "active": true,
            "client_id": "test-client",
            "username": "test-user",
            "scope": "read",
            "token_type": "Bearer",
            "exp": (Utc::now() + Duration::hours(1)).timestamp()
        })))
        .mount(&mock_server)
        .await;

    let config = create_test_oauth2_config(&mock_server.uri());
    let client = OAuth2Client::new(config)?;

    let result = client.introspect_token("retry-token").await;
    assert!(result.is_ok());

    Ok(())
}

#[tokio::test]
async fn test_successful_client_credentials_token() -> Result<()> {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/oauth2/token"))
        .and(header("content-type", "application/x-www-form-urlencoded"))
        .and(body_string_contains("grant_type=client_credentials"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "client-access-token",
            "token_type": "Bearer",
            "expires_in": 3600,
            "scope": "read write"
        })))
        .mount(&mock_server)
        .await;

    let config = create_test_oauth2_config(&mock_server.uri());
    let client = OAuth2Client::new(config)?;

    let scopes = vec!["read".to_string(), "write".to_string()];
    let result = client.get_client_credentials_token(&scopes).await;
    assert!(result.is_ok());

    let token = result.unwrap();
    assert_eq!(token.access_token, "client-access-token");
    assert_eq!(token.token_type, "Bearer");
    assert_eq!(token.scopes, vec!["read".to_string(), "write".to_string()]);

    Ok(())
}

#[tokio::test]
async fn test_client_credentials_disabled() -> Result<()> {
    let mock_server = MockServer::start().await;
    let mut config = create_test_oauth2_config(&mock_server.uri());
    config.service_to_service_enabled = false;

    let client = OAuth2Client::new(config)?;

    let result = client.get_client_credentials_token(&[]).await;
    assert!(result.is_err());

    match result.unwrap_err() {
        OAuth2Error::ConfigError(msg) => {
            assert!(msg.contains("Service-to-service authentication is disabled"));
        }
        other => panic!("Expected ConfigError, got: {:?}", other),
    }

    Ok(())
}

#[tokio::test]
async fn test_oauth2_error_response_parsing() -> Result<()> {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/oauth2/token"))
        .respond_with(ResponseTemplate::new(400).set_body_json(json!({
            "error": "invalid_client",
            "error_description": "Client authentication failed"
        })))
        .mount(&mock_server)
        .await;

    let config = create_test_oauth2_config(&mock_server.uri());
    let client = OAuth2Client::new(config)?;

    let result = client.get_client_credentials_token(&["read".to_string()]).await;
    assert!(result.is_err());

    match result.unwrap_err() {
        OAuth2Error::ServerError { error, description } => {
            assert_eq!(error, "invalid_client");
            assert_eq!(description, "Client authentication failed");
        }
        other => panic!("Expected ServerError, got: {:?}", other),
    }

    Ok(())
}

#[tokio::test]
async fn test_token_cache_validation() -> Result<()> {
    let cache = media_management_service::infrastructure::oauth2::TokenCache::new(300, 1800);

    let token_info = CachedTokenInfo {
        active: true,
        client_id: Some("test-client".to_string()),
        username: Some("test-user".to_string()),
        scopes: vec!["read".to_string(), "write".to_string()],
        token_type: Some("Bearer".to_string()),
        expires_at: Some(Utc::now() + Duration::hours(1)),
        subject: Some("test-user".to_string()),
        audience: Some(vec!["test-client".to_string()]),
        issuer: Some("auth-service".to_string()),
        cached_at: Utc::now(),
    };

    cache.cache_validation("test-token".to_string(), token_info.clone()).await;

    let cached = cache.get_validation("test-token").await;
    assert!(cached.is_some());
    assert_eq!(cached.unwrap().client_id, token_info.client_id);

    Ok(())
}

#[tokio::test]
async fn test_token_cache_expiry() -> Result<()> {
    let cache = media_management_service::infrastructure::oauth2::TokenCache::new(1, 1800); // 1 second TTL

    let token_info = CachedTokenInfo {
        active: true,
        client_id: Some("test-client".to_string()),
        username: Some("test-user".to_string()),
        scopes: vec!["read".to_string()],
        token_type: Some("Bearer".to_string()),
        expires_at: Some(Utc::now() + Duration::hours(1)),
        subject: Some("test-user".to_string()),
        audience: Some(vec!["test-client".to_string()]),
        issuer: Some("auth-service".to_string()),
        cached_at: Utc::now(),
    };

    cache.cache_validation("test-token".to_string(), token_info).await;

    // Should be cached immediately
    let cached = cache.get_validation("test-token").await;
    assert!(cached.is_some());

    // Wait for cache to expire
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Should no longer be cached
    let cached = cache.get_validation("test-token").await;
    assert!(cached.is_none());

    Ok(())
}

#[tokio::test]
async fn test_client_token_cache() -> Result<()> {
    let cache = media_management_service::infrastructure::oauth2::TokenCache::new(300, 1800);

    let scopes = vec!["read".to_string(), "write".to_string()];
    let client_token = CachedClientToken {
        access_token: "test-access-token".to_string(),
        token_type: "Bearer".to_string(),
        expires_at: Utc::now() + Duration::hours(1),
        scopes: scopes.clone(),
    };

    cache.cache_client_token(scopes.clone(), client_token.clone()).await;

    let cached = cache.get_client_token(&scopes).await;
    assert!(cached.is_some());
    assert_eq!(cached.unwrap().access_token, client_token.access_token);

    Ok(())
}

#[tokio::test]
async fn test_client_token_cache_with_buffer() -> Result<()> {
    let cache = media_management_service::infrastructure::oauth2::TokenCache::new(300, 1800);

    let scopes = vec!["read".to_string()];
    // Token that expires in 30 seconds (within the 60-second buffer)
    let client_token = CachedClientToken {
        access_token: "test-access-token".to_string(),
        token_type: "Bearer".to_string(),
        expires_at: Utc::now() + Duration::seconds(30),
        scopes: scopes.clone(),
    };

    cache.cache_client_token(scopes.clone(), client_token).await;

    // Should not be returned due to 60-second buffer
    let cached = cache.get_client_token(&scopes).await;
    assert!(cached.is_none());

    Ok(())
}

#[tokio::test]
async fn test_cache_cleanup() -> Result<()> {
    let cache = media_management_service::infrastructure::oauth2::TokenCache::new(1, 1); // 1 second TTL

    // Add expired validation cache entry
    let token_info = CachedTokenInfo {
        active: true,
        client_id: Some("test-client".to_string()),
        username: Some("test-user".to_string()),
        scopes: vec!["read".to_string()],
        token_type: Some("Bearer".to_string()),
        expires_at: Some(Utc::now() + Duration::hours(1)),
        subject: Some("test-user".to_string()),
        audience: Some(vec!["test-client".to_string()]),
        issuer: Some("auth-service".to_string()),
        cached_at: Utc::now() - Duration::seconds(5), // Expired
    };
    cache.cache_validation("expired-token".to_string(), token_info).await;

    // Add expired client token
    let client_token = CachedClientToken {
        access_token: "expired-token".to_string(),
        token_type: "Bearer".to_string(),
        expires_at: Utc::now() - Duration::seconds(5), // Expired
        scopes: vec!["read".to_string()],
    };
    cache.cache_client_token(vec!["read".to_string()], client_token).await;

    // Cleanup expired entries
    cache.cleanup_expired().await;

    // Both should be cleaned up
    let validation_cached = cache.get_validation("expired-token").await;
    assert!(validation_cached.is_none());

    let client_cached = cache.get_client_token(&["read".to_string()]).await;
    assert!(client_cached.is_none());

    Ok(())
}

#[tokio::test]
async fn test_health_check_success() -> Result<()> {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/health"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    let config = create_test_oauth2_config(&mock_server.uri());
    let client = OAuth2Client::new(config)?;

    let result = client.health_check().await;
    assert!(result.is_ok());
    assert!(result.unwrap());

    Ok(())
}

#[tokio::test]
async fn test_health_check_failure() -> Result<()> {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/health"))
        .respond_with(ResponseTemplate::new(503))
        .mount(&mock_server)
        .await;

    let config = create_test_oauth2_config(&mock_server.uri());
    let client = OAuth2Client::new(config)?;

    let result = client.health_check().await;
    assert!(result.is_ok());
    assert!(!result.unwrap());

    Ok(())
}

#[tokio::test]
async fn test_introspection_caching_behavior() -> Result<()> {
    let mock_server = MockServer::start().await;

    // Should only be called once due to caching
    Mock::given(method("POST"))
        .and(path("/oauth2/introspect"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "active": true,
            "client_id": "test-client",
            "username": "test-user",
            "scope": "read",
            "token_type": "Bearer",
            "exp": (Utc::now() + Duration::hours(1)).timestamp()
        })))
        .expect(1)
        .mount(&mock_server)
        .await;

    let config = create_test_oauth2_config(&mock_server.uri());
    let client = OAuth2Client::new(config)?;

    // First call should hit the server
    let result1 = client.introspect_token("cached-token").await;
    assert!(result1.is_ok());

    // Second call should use cache
    let result2 = client.introspect_token("cached-token").await;
    assert!(result2.is_ok());

    // Both results should be identical
    let info1 = result1.unwrap();
    let info2 = result2.unwrap();
    assert_eq!(info1.client_id, info2.client_id);
    assert_eq!(info1.username, info2.username);

    Ok(())
}

#[tokio::test]
async fn test_client_credentials_caching_behavior() -> Result<()> {
    let mock_server = MockServer::start().await;

    // Should only be called once due to caching
    Mock::given(method("POST"))
        .and(path("/oauth2/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "cached-client-token",
            "token_type": "Bearer",
            "expires_in": 3600,
            "scope": "read write"
        })))
        .expect(1)
        .mount(&mock_server)
        .await;

    let config = create_test_oauth2_config(&mock_server.uri());
    let client = OAuth2Client::new(config)?;

    let scopes = vec!["read".to_string(), "write".to_string()];

    // First call should hit the server
    let result1 = client.get_client_credentials_token(&scopes).await;
    assert!(result1.is_ok());

    // Second call should use cache
    let result2 = client.get_client_credentials_token(&scopes).await;
    assert!(result2.is_ok());

    // Both results should be identical
    let token1 = result1.unwrap();
    let token2 = result2.unwrap();
    assert_eq!(token1.access_token, token2.access_token);

    Ok(())
}
