use std::sync::Arc;

use crate::{
    application::dto::{InitiateUploadRequest, InitiateUploadResponse},
    domain::{
        entities::{Media, UserId},
        repositories::MediaRepository,
        value_objects::{ContentHash, MediaType, ProcessingStatus},
    },
    infrastructure::storage::PresignedUrlService,
    presentation::middleware::error::AppError,
};

/// Use case for initiating presigned URL uploads
pub struct InitiateUploadUseCase<R>
where
    R: MediaRepository + ?Sized,
{
    repository: Arc<R>,
    presigned_service: PresignedUrlService,
    max_file_size: u64,
}

impl<R> InitiateUploadUseCase<R>
where
    R: MediaRepository + ?Sized,
{
    /// Create a new initiate upload use case
    pub fn new(
        repository: Arc<R>,
        presigned_service: PresignedUrlService,
        max_file_size: u64,
    ) -> Self {
        Self { repository, presigned_service, max_file_size }
    }

    /// Execute the upload initiation
    pub async fn execute(
        &self,
        request: InitiateUploadRequest,
        user_id: UserId,
    ) -> Result<InitiateUploadResponse, AppError> {
        tracing::info!(
            "Initiating upload session for file: {} (size: {} bytes, type: {})",
            request.filename,
            request.file_size,
            request.content_type
        );

        // Validate the upload request
        Self::validate_upload_request(&request)?;

        // Validate file size
        if request.file_size > self.max_file_size {
            return Err(AppError::BadRequest {
                message: format!(
                    "File size {} bytes exceeds maximum allowed size of {} bytes",
                    request.file_size, self.max_file_size
                ),
            });
        }

        // Validate content type
        let media_type = MediaType::new(&request.content_type);

        // Create a placeholder media entity for upload session
        // We'll update it with actual content details when upload completes
        let placeholder_media = Self::create_upload_placeholder(
            &request.filename,
            &media_type,
            request.file_size,
            user_id,
        );

        // Save placeholder to database to get media ID
        let media_id = self.repository.save(&placeholder_media).await.map_err(|e| {
            tracing::error!("Failed to create upload session record: {}", e);
            AppError::Internal { message: format!("Failed to create upload session: {e}") }
        })?;

        // Generate presigned URL session
        let upload_session = self
            .presigned_service
            .create_upload_session(
                media_id,
                &request.filename,
                &request.content_type,
                request.file_size,
            )
            .map_err(|e| {
                tracing::error!("Failed to create presigned URL: {}", e);
                AppError::Internal { message: format!("Failed to generate upload URL: {e}") }
            })?;

        tracing::info!(
            "Upload session created successfully - media_id: {}, expires at: {}",
            media_id,
            upload_session.expires_at
        );

        Ok(InitiateUploadResponse {
            media_id,
            upload_url: upload_session.upload_url,
            upload_token: upload_session.upload_token,
            expires_at: upload_session.expires_at.to_rfc3339(),
            status: ProcessingStatus::Pending,
        })
    }

    /// Create a placeholder media entity for the upload session
    fn create_upload_placeholder(
        filename: &str,
        media_type: &MediaType,
        file_size: u64,
        user_id: UserId,
    ) -> Media {
        // Create a placeholder content hash - will be updated when file is uploaded
        let placeholder_hash = ContentHash::new(
            &"0".repeat(64), // Placeholder hash
        )
        .unwrap();

        // Create placeholder media entity
        let mut media = Media::new(
            placeholder_hash,
            filename.to_string(),
            media_type.clone(),
            "pending".to_string(), // Placeholder path
            file_size,
            user_id,
        );

        // Set status to Pending for upload session
        media.processing_status = ProcessingStatus::Pending;

        media
    }

    /// Validate upload session constraints
    fn validate_upload_request(request: &InitiateUploadRequest) -> Result<(), AppError> {
        // Validate filename
        if request.filename.is_empty() {
            return Err(AppError::BadRequest { message: "Filename cannot be empty".to_string() });
        }

        // Check for potentially dangerous file extensions
        let filename_lower = request.filename.to_lowercase();
        let dangerous_extensions = [".exe", ".bat", ".cmd", ".com", ".scr", ".vbs", ".js"];

        if dangerous_extensions.iter().any(|ext| filename_lower.ends_with(ext)) {
            return Err(AppError::BadRequest {
                message: format!("File type not allowed: {}", request.filename),
            });
        }

        // Validate content type format
        if !request.content_type.contains('/') {
            return Err(AppError::BadRequest {
                message: "Invalid content type format".to_string(),
            });
        }

        // Validate file size is reasonable (not zero, not negative due to u64)
        if request.file_size == 0 {
            return Err(AppError::BadRequest {
                message: "File size must be greater than zero".to_string(),
            });
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        infrastructure::storage::PresignedUrlConfig, test_utils::mocks::InMemoryMediaRepository,
    };
    use std::time::Duration;

    fn create_test_use_case() -> InitiateUploadUseCase<InMemoryMediaRepository> {
        let repository = Arc::new(InMemoryMediaRepository::new());
        let config = PresignedUrlConfig {
            secret_key: "test-secret".to_string(),
            base_url: "http://localhost:3000".to_string(),
            default_expiration: Duration::from_secs(900),
            max_file_size: 10 * 1024 * 1024, // 10MB
        };
        let presigned_service = PresignedUrlService::new(config);

        InitiateUploadUseCase::new(repository, presigned_service, 10 * 1024 * 1024)
    }

    #[tokio::test]
    async fn test_initiate_upload_success() {
        let use_case = create_test_use_case();
        let user_id = UserId::new();

        let request = InitiateUploadRequest {
            filename: "test.jpg".to_string(),
            content_type: "image/jpeg".to_string(),
            file_size: 1024 * 1024, // 1MB
        };

        let result = use_case.execute(request, user_id).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(!response.upload_url.is_empty());
        assert!(!response.upload_token.is_empty());
        assert!(response.upload_url.contains(&response.upload_token));
        assert_eq!(response.status, ProcessingStatus::Pending);
    }

    #[tokio::test]
    async fn test_file_too_large_rejection() {
        let use_case = create_test_use_case();
        let user_id = UserId::new();

        let request = InitiateUploadRequest {
            filename: "large.jpg".to_string(),
            content_type: "image/jpeg".to_string(),
            file_size: 50 * 1024 * 1024, // 50MB (exceeds 10MB limit)
        };

        let result = use_case.execute(request, user_id).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::BadRequest { message } => {
                assert!(message.contains("exceeds maximum allowed size"));
            }
            _ => panic!("Expected BadRequest error"),
        }
    }

    #[tokio::test]
    async fn test_empty_filename_rejection() {
        let _use_case = create_test_use_case();
        let _user_id = UserId::new();

        let request = InitiateUploadRequest {
            filename: String::new(),
            content_type: "image/jpeg".to_string(),
            file_size: 1024,
        };

        let result =
            InitiateUploadUseCase::<InMemoryMediaRepository>::validate_upload_request(&request);

        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::BadRequest { message } => {
                assert!(message.contains("Filename cannot be empty"));
            }
            _ => panic!("Expected BadRequest error"),
        }
    }

    #[tokio::test]
    async fn test_dangerous_file_extension_rejection() {
        let _use_case = create_test_use_case();

        let dangerous_files = [
            "malware.exe",
            "script.bat",
            "command.cmd",
            "program.com",
            "screen.scr",
            "visual.vbs",
            "javascript.js",
        ];

        for filename in dangerous_files {
            let request = InitiateUploadRequest {
                filename: filename.to_string(),
                content_type: "application/octet-stream".to_string(),
                file_size: 1024,
            };

            let result =
                InitiateUploadUseCase::<InMemoryMediaRepository>::validate_upload_request(&request);
            assert!(result.is_err(), "Should reject dangerous file: {filename}");
        }
    }

    #[tokio::test]
    async fn test_zero_file_size_rejection() {
        let _use_case = create_test_use_case();

        let request = InitiateUploadRequest {
            filename: "empty.txt".to_string(),
            content_type: "text/plain".to_string(),
            file_size: 0,
        };

        let result =
            InitiateUploadUseCase::<InMemoryMediaRepository>::validate_upload_request(&request);

        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::BadRequest { message } => {
                assert!(message.contains("File size must be greater than zero"));
            }
            _ => panic!("Expected BadRequest error"),
        }
    }

    #[tokio::test]
    async fn test_invalid_content_type_rejection() {
        let _use_case = create_test_use_case();

        let request = InitiateUploadRequest {
            filename: "test.txt".to_string(),
            content_type: "invalid_content_type".to_string(), // Missing slash
            file_size: 1024,
        };

        let result =
            InitiateUploadUseCase::<InMemoryMediaRepository>::validate_upload_request(&request);

        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::BadRequest { message } => {
                assert!(message.contains("Invalid content type format"));
            }
            _ => panic!("Expected BadRequest error"),
        }
    }

    #[tokio::test]
    async fn test_upload_url_contains_security_parameters() {
        let use_case = create_test_use_case();
        let user_id = UserId::new();

        let request = InitiateUploadRequest {
            filename: "secure.png".to_string(),
            content_type: "image/png".to_string(),
            file_size: 2048,
        };

        let result = use_case.execute(request, user_id).await.unwrap();

        // Verify URL contains required security parameters
        assert!(result.upload_url.contains("signature="));
        assert!(result.upload_url.contains("expires="));
        assert!(result.upload_url.contains("size=2048"));
        assert!(result.upload_url.contains("type=image%2Fpng"));
    }
}
