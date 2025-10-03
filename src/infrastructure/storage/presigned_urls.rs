use chrono::{DateTime, Utc};
use hmac::{Hmac, Mac};
use rand::distr::Alphanumeric;
use rand::Rng;
use sha2::Sha256;
use std::time::Duration;

use crate::domain::entities::MediaId;
use crate::infrastructure::config::AppConfig;

type HmacSha256 = Hmac<Sha256>;

/// Configuration for presigned upload URLs
#[derive(Debug, Clone)]
pub struct PresignedUrlConfig {
    /// Secret key for HMAC signing
    pub secret_key: String,
    /// Base URL for the service
    pub base_url: String,
    /// Default URL expiration time
    pub default_expiration: Duration,
    /// Maximum file size allowed
    pub max_file_size: u64,
}

impl Default for PresignedUrlConfig {
    fn default() -> Self {
        Self {
            secret_key: "default-dev-secret-change-in-production".to_string(),
            base_url: "http://localhost:3000".to_string(),
            default_expiration: Duration::from_secs(15 * 60), // 15 minutes
            max_file_size: 50 * 1024 * 1024,                  // 50MB
        }
    }
}

/// Represents an upload session with presigned URL
#[derive(Debug, Clone)]
pub struct UploadSession {
    pub media_id: MediaId,
    pub upload_token: String,
    pub upload_url: String,
    pub expires_at: DateTime<Utc>,
    pub max_file_size: u64,
    pub expected_content_type: String,
}

/// Service for generating and validating presigned upload URLs
#[derive(Clone)]
pub struct PresignedUrlService {
    config: PresignedUrlConfig,
}

impl PresignedUrlService {
    /// Create a new presigned URL service
    pub fn new(config: PresignedUrlConfig) -> Self {
        Self { config }
    }

    /// Create from app configuration
    pub fn from_app_config(app_config: &AppConfig) -> Self {
        let config = PresignedUrlConfig {
            secret_key: std::env::var("UPLOAD_URL_SECRET")
                .unwrap_or_else(|_| "default-dev-secret-change-in-production".to_string()),
            base_url: format!("http://{}:{}", app_config.server.host, app_config.server.port),
            default_expiration: Duration::from_secs(15 * 60),
            max_file_size: app_config.middleware.validation.max_file_size_mb * 1024 * 1024,
        };
        Self::new(config)
    }

    /// Generate a new upload session with presigned URL
    pub fn create_upload_session(
        &self,
        media_id: MediaId,
        filename: &str,
        content_type: &str,
        file_size: u64,
    ) -> Result<UploadSession, PresignedUrlError> {
        // Validate file size
        if file_size > self.config.max_file_size {
            return Err(PresignedUrlError::FileTooLarge {
                size: file_size,
                max_size: self.config.max_file_size,
            });
        }

        // Generate unique upload token
        let upload_token = Self::generate_upload_token();

        // Calculate expiration
        let expires_at = Utc::now()
            + chrono::Duration::from_std(self.config.default_expiration)
                .map_err(|_| PresignedUrlError::InvalidExpiration)?;

        // Create signature payload
        let signature_payload = Self::create_signature_payload(
            &upload_token,
            media_id,
            filename,
            content_type,
            file_size,
            expires_at,
        );

        // Generate HMAC signature
        let signature = self.sign_payload(&signature_payload)?;

        // Build presigned URL
        let upload_url = format!(
            "{}/api/v1/media-management/media/upload/{}?signature={}&expires={}&size={}&type={}",
            self.config.base_url,
            upload_token,
            signature,
            expires_at.timestamp(),
            file_size,
            urlencoding::encode(content_type)
        );

        Ok(UploadSession {
            media_id,
            upload_token,
            upload_url,
            expires_at,
            max_file_size: file_size,
            expected_content_type: content_type.to_string(),
        })
    }

    /// Validate a presigned URL and extract session information
    pub fn validate_upload_url(
        &self,
        _upload_token: &str,
        signature: &str,
        expires_timestamp: i64,
        _expected_size: u64,
        _content_type: &str,
    ) -> Result<(), PresignedUrlError> {
        // Check expiration
        let expires_at = DateTime::from_timestamp(expires_timestamp, 0)
            .ok_or(PresignedUrlError::InvalidExpiration)?;

        if Utc::now() > expires_at {
            return Err(PresignedUrlError::Expired { expired_at: expires_at });
        }

        // Reconstruct signature payload (we need media_id from database)
        // For now, just validate the signature format
        if signature.is_empty() {
            return Err(PresignedUrlError::InvalidSignature);
        }

        // Additional validations would go here (size, content-type, etc.)

        Ok(())
    }

    /// Generate a unique upload token
    fn generate_upload_token() -> String {
        let random_part: String =
            rand::rng().sample_iter(&Alphanumeric).take(32).map(char::from).collect();

        format!("upload_{random_part}")
    }

    /// Create the payload to be signed
    fn create_signature_payload(
        upload_token: &str,
        media_id: MediaId,
        filename: &str,
        content_type: &str,
        file_size: u64,
        expires_at: DateTime<Utc>,
    ) -> String {
        format!(
            "{}|{}|{}|{}|{}|{}",
            upload_token,
            media_id.as_i64(),
            filename,
            content_type,
            file_size,
            expires_at.timestamp()
        )
    }

    /// Generate HMAC signature for the payload
    fn sign_payload(&self, payload: &str) -> Result<String, PresignedUrlError> {
        let mut mac = HmacSha256::new_from_slice(self.config.secret_key.as_bytes())
            .map_err(|_| PresignedUrlError::SigningError)?;

        mac.update(payload.as_bytes());
        let result = mac.finalize();

        Ok(hex::encode(result.into_bytes()))
    }

    /// Verify HMAC signature
    pub fn verify_signature(
        &self,
        payload: &str,
        signature: &str,
    ) -> Result<(), PresignedUrlError> {
        let expected_signature = self.sign_payload(payload)?;

        if signature != expected_signature {
            return Err(PresignedUrlError::InvalidSignature);
        }

        Ok(())
    }
}

/// Errors that can occur during presigned URL operations
#[derive(Debug, thiserror::Error)]
pub enum PresignedUrlError {
    #[error("File size {size} exceeds maximum allowed size of {max_size} bytes")]
    FileTooLarge { size: u64, max_size: u64 },

    #[error("Upload URL has expired at {expired_at}")]
    Expired { expired_at: DateTime<Utc> },

    #[error("Invalid signature")]
    InvalidSignature,

    #[error("Invalid expiration time")]
    InvalidExpiration,

    #[error("Failed to sign payload")]
    SigningError,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::MediaId;

    fn create_test_service() -> PresignedUrlService {
        let config = PresignedUrlConfig {
            secret_key: "test-secret-key".to_string(),
            base_url: "http://localhost:3000".to_string(),
            default_expiration: Duration::from_secs(900), // 15 minutes
            max_file_size: 10 * 1024 * 1024,              // 10MB
        };
        PresignedUrlService::new(config)
    }

    #[test]
    fn test_create_upload_session_success() {
        let service = create_test_service();
        let media_id = MediaId::new(123);

        let session = service
            .create_upload_session(
                media_id,
                "test.jpg",
                "image/jpeg",
                1024 * 1024, // 1MB
            )
            .unwrap();

        assert_eq!(session.media_id, media_id);
        assert!(!session.upload_token.is_empty());
        assert!(session.upload_url.contains(&session.upload_token));
        assert!(session.upload_url.contains("signature="));
        assert!(session.expires_at > Utc::now());
    }

    #[test]
    fn test_file_too_large_rejection() {
        let service = create_test_service();
        let media_id = MediaId::new(123);

        let result = service.create_upload_session(
            media_id,
            "large.jpg",
            "image/jpeg",
            50 * 1024 * 1024, // 50MB (exceeds 10MB limit)
        );

        assert!(matches!(result, Err(PresignedUrlError::FileTooLarge { .. })));
    }

    #[test]
    fn test_url_contains_security_parameters() {
        let service = create_test_service();
        let media_id = MediaId::new(456);

        let session =
            service.create_upload_session(media_id, "secure.png", "image/png", 2048).unwrap();

        // Verify URL contains required security parameters
        assert!(session.upload_url.contains("signature="));
        assert!(session.upload_url.contains("expires="));
        assert!(session.upload_url.contains("size=2048"));
        assert!(session.upload_url.contains("type=image%2Fpng"));
    }

    #[test]
    fn test_signature_consistency() {
        let service = create_test_service();
        let payload = "test|123|file.jpg|image/jpeg|1024|1640995200";

        let signature1 = service.sign_payload(payload).unwrap();
        let signature2 = service.sign_payload(payload).unwrap();

        assert_eq!(signature1, signature2);
        assert!(!signature1.is_empty());
    }

    #[test]
    fn test_signature_verification() {
        let service = create_test_service();
        let payload = "test|789|verify.jpg|image/jpeg|2048|1640995200";

        let signature = service.sign_payload(payload).unwrap();
        let verification = service.verify_signature(payload, &signature);

        assert!(verification.is_ok());
    }

    #[test]
    fn test_invalid_signature_rejection() {
        let service = create_test_service();
        let payload = "test|789|verify.jpg|image/jpeg|2048|1640995200";

        let verification = service.verify_signature(payload, "invalid_signature");

        assert!(matches!(verification, Err(PresignedUrlError::InvalidSignature)));
    }
}
