use std::sync::Arc;
use tokio::io::AsyncRead;

use crate::{
    application::dto::UploadMediaResponse,
    domain::{
        entities::{Media, UserId},
        repositories::MediaRepository,
        value_objects::MediaType,
    },
    infrastructure::storage::{
        utils::{
            detect_content_type, generate_content_hash_async, validate_content_type,
            validate_file_size,
        },
        FileStorage, StorageError,
    },
    presentation::middleware::error::AppError,
};

/// Use case for uploading media files
pub struct UploadMediaUseCase<R, S>
where
    R: MediaRepository + ?Sized,
    S: FileStorage + ?Sized,
{
    repository: Arc<R>,
    storage: Arc<S>,
    max_file_size: u64,
}

impl<R, S> UploadMediaUseCase<R, S>
where
    R: MediaRepository + ?Sized,
    S: FileStorage + ?Sized,
{
    /// Create a new upload media use case
    pub fn new(repository: Arc<R>, storage: Arc<S>, max_file_size: u64) -> Self {
        Self { repository, storage, max_file_size }
    }

    /// Execute the upload media use case
    pub async fn execute<Reader>(
        &self,
        file_reader: Reader,
        filename: String,
        user_id: UserId,
        expected_content_type: Option<String>,
    ) -> Result<UploadMediaResponse, AppError>
    where
        Reader: AsyncRead + Send + Unpin,
    {
        tracing::info!("Starting media upload for file: {}", filename);

        // Generate content hash and read file data
        let (content_hash, file_data) =
            generate_content_hash_async(file_reader).await.map_err(|e| AppError::BadRequest {
                message: format!("Failed to process file: {e}"),
            })?;

        // Validate file size
        validate_file_size(file_data.len() as u64, self.max_file_size)
            .map_err(|e| AppError::BadRequest { message: format!("File too large: {e}") })?;

        // Check if file already exists (deduplication)
        if let Ok(Some(media)) = self.repository.find_by_content_hash(&content_hash).await {
            tracing::info!(
                "File already exists with hash: {}, returning existing media",
                content_hash.as_str()
            );

            return Ok(UploadMediaResponse {
                media_id: media.id,
                content_hash: content_hash.as_str().to_string(),
                processing_status: media.processing_status,
                upload_url: None, // Could add direct access URL if needed
            });
        }

        // Detect content type
        let detected_content_type = detect_content_type(&file_data, Some(&filename));

        // Validate content type if expected type is provided
        if let Some(expected) = &expected_content_type {
            validate_content_type(&file_data, expected).map_err(|e| AppError::BadRequest {
                message: format!("Content type validation failed: {e}"),
            })?;
        }

        // Create media type from detected content type
        let media_type = MediaType::new(&detected_content_type);

        // Store file in storage system
        let cursor = std::io::Cursor::new(&file_data);
        let storage_path =
            self.storage.store(&content_hash, cursor).await.map_err(|e| match e {
                StorageError::StorageFull => {
                    AppError::BadRequest { message: "Storage full or file too large".to_string() }
                }
                _ => AppError::Internal { message: format!("Storage error: {e}") },
            })?;

        tracing::info!("File stored at path: {}", storage_path);

        // Create media entity
        let media = Media::new(
            content_hash.clone(),
            filename,
            media_type,
            storage_path,
            file_data.len() as u64,
            user_id,
        );

        // Save media metadata to database
        let saved_media = match self.repository.save(&media).await {
            Ok(()) => {
                // Since save doesn't return the media with ID, we need to fetch it
                // In a real implementation, the repository might return the saved entity
                // For now, we'll create a response with a temporary ID
                media
            }
            Err(e) => {
                // If database save fails, try to clean up stored file
                let _ = self.storage.delete(&content_hash).await;

                return Err(AppError::Internal {
                    message: format!("Failed to save media metadata: {e}"),
                });
            }
        };

        tracing::info!(
            "Media upload completed successfully for file: {}",
            saved_media.original_filename
        );

        Ok(UploadMediaResponse {
            media_id: saved_media.id,
            content_hash: content_hash.as_str().to_string(),
            processing_status: saved_media.processing_status,
            upload_url: None,
        })
    }

    /// Execute upload with automatic user ID (for testing or when user is known from context)
    pub async fn execute_with_default_user<Reader>(
        &self,
        file_reader: Reader,
        filename: String,
        expected_content_type: Option<String>,
    ) -> Result<UploadMediaResponse, AppError>
    where
        Reader: AsyncRead + Send + Unpin,
    {
        let default_user = UserId::new();
        self.execute(file_reader, filename, default_user, expected_content_type).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use tempfile::TempDir;

    use crate::{
        domain::value_objects::{ContentHash, ProcessingStatus},
        infrastructure::storage::FilesystemStorage,
        test_utils::mocks::InMemoryMediaRepository,
    };

    #[tokio::test]
    async fn test_upload_media_success() {
        let temp_dir = TempDir::new().unwrap();
        let repo = InMemoryMediaRepository::new();
        let storage = FilesystemStorage::new(temp_dir.path());

        let use_case = UploadMediaUseCase::new(
            Arc::new(repo),
            Arc::new(storage),
            10_000_000, // 10MB limit
        );

        let file_data = b"hello world";
        let file_reader = Cursor::new(file_data);
        let user_id = UserId::new();

        let result = use_case.execute(file_reader, "test.txt".to_string(), user_id, None).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(
            response.content_hash,
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
        assert!(matches!(response.processing_status, ProcessingStatus::Pending));
    }

    #[tokio::test]
    async fn test_upload_media_file_too_large() {
        let temp_dir = TempDir::new().unwrap();
        let repo = InMemoryMediaRepository::new();
        let storage = FilesystemStorage::new(temp_dir.path());

        let use_case = UploadMediaUseCase::new(
            Arc::new(repo),
            Arc::new(storage),
            5, // 5 bytes limit
        );

        let file_data = b"this is a longer file that exceeds the limit";
        let file_reader = Cursor::new(file_data);
        let user_id = UserId::new();

        let result =
            use_case.execute(file_reader, "large_file.txt".to_string(), user_id, None).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::BadRequest { message } => {
                assert!(message.contains("File too large"));
            }
            _ => panic!("Expected BadRequest error"),
        }
    }

    #[tokio::test]
    async fn test_upload_media_deduplication() {
        let temp_dir = TempDir::new().unwrap();
        let content_hash =
            ContentHash::new("b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9")
                .unwrap();
        let existing_media = Media::new(
            content_hash.clone(),
            "existing.txt".to_string(),
            MediaType::new("text/plain"),
            "/path/to/existing".to_string(),
            11,
            UserId::new(),
        );

        let repo = InMemoryMediaRepository::new().with_media(existing_media);
        let storage = FilesystemStorage::new(temp_dir.path());

        let use_case = UploadMediaUseCase::new(Arc::new(repo), Arc::new(storage), 10_000_000);

        let file_data = b"hello world";
        let file_reader = Cursor::new(file_data);
        let user_id = UserId::new();

        let result =
            use_case.execute(file_reader, "duplicate.txt".to_string(), user_id, None).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.content_hash, content_hash.as_str());
    }

    // Note: Additional integration tests with real filesystem storage would go in the integration test directory
}
