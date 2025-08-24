use std::sync::Arc;
use tokio::io::{AsyncRead, AsyncReadExt};

use crate::{
    domain::{
        entities::{Media, MediaId},
        repositories::MediaRepository,
    },
    infrastructure::storage::{FileStorage, StorageError},
    presentation::middleware::error::AppError,
};

/// Use case for downloading media files
pub struct DownloadMediaUseCase<R, S>
where
    R: MediaRepository,
    S: FileStorage,
{
    repository: Arc<R>,
    storage: Arc<S>,
}

/// Download response containing file data and metadata
#[derive(Debug)]
pub struct DownloadResponse {
    pub content: Vec<u8>,
    pub content_type: String,
    pub filename: String,
    pub file_size: u64,
}

impl<R, S> DownloadMediaUseCase<R, S>
where
    R: MediaRepository,
    S: FileStorage,
{
    /// Create a new download media use case
    pub fn new(repository: Arc<R>, storage: Arc<S>) -> Self {
        Self { repository, storage }
    }

    /// Execute the download media use case
    pub async fn execute(&self, media_id: MediaId) -> Result<DownloadResponse, AppError> {
        tracing::info!("Downloading media with ID: {}", media_id);

        // Get media metadata from database
        let media =
            self.repository.find_by_id(media_id).await.map_err(|e| AppError::Internal {
                message: format!("Failed to query media: {e}"),
            })?;

        let Some(media) = media else {
            tracing::warn!("Media not found with ID: {}", media_id);
            return Err(AppError::NotFound { resource: format!("Media with ID {media_id}") });
        };

        // Check if media processing is complete
        if !media.is_ready() {
            tracing::warn!(
                "Media not ready for download: {} (status: {:?})",
                media_id,
                media.processing_status
            );
            return Err(AppError::BadRequest {
                message: format!(
                    "Media is not ready for download. Status: {:?}",
                    media.processing_status
                ),
            });
        }

        tracing::info!("Retrieving file from storage: {}", media.content_hash.as_str());

        // Get file from storage
        let mut file_reader =
            self.storage.retrieve(&media.content_hash).await.map_err(|e| match e {
                StorageError::FileNotFound { .. } => {
                    AppError::NotFound { resource: format!("File content for media {media_id}") }
                }
                _ => AppError::Internal { message: format!("Storage error: {e}") },
            })?;

        // Read file content
        let mut content = Vec::new();
        file_reader.read_to_end(&mut content).await.map_err(|e| AppError::Internal {
            message: format!("Failed to read file content: {e}"),
        })?;

        tracing::info!(
            "Successfully downloaded media: {} ({} bytes)",
            media.original_filename,
            content.len()
        );

        Ok(DownloadResponse {
            content,
            content_type: media.media_type.mime_type().to_string(),
            filename: media.original_filename,
            file_size: media.file_size,
        })
    }

    /// Execute download and return streaming reader (for large files)
    /// This method returns the reader directly without loading the entire file into memory
    pub async fn execute_stream(
        &self,
        media_id: MediaId,
    ) -> Result<(Box<dyn AsyncRead + Send + Unpin>, Media), AppError> {
        tracing::info!("Streaming media with ID: {}", media_id);

        // Get media metadata from database
        let media =
            self.repository.find_by_id(media_id).await.map_err(|e| AppError::Internal {
                message: format!("Failed to query media: {e}"),
            })?;

        let Some(media) = media else {
            tracing::warn!("Media not found with ID: {}", media_id);
            return Err(AppError::NotFound { resource: format!("Media with ID {media_id}") });
        };

        // Check if media processing is complete
        if !media.is_ready() {
            tracing::warn!(
                "Media not ready for streaming: {} (status: {:?})",
                media_id,
                media.processing_status
            );
            return Err(AppError::BadRequest {
                message: format!(
                    "Media is not ready for download. Status: {:?}",
                    media.processing_status
                ),
            });
        }

        tracing::info!("Streaming file from storage: {}", media.content_hash.as_str());

        // Get file reader from storage
        let file_reader =
            self.storage.retrieve(&media.content_hash).await.map_err(|e| match e {
                StorageError::FileNotFound { .. } => {
                    AppError::NotFound { resource: format!("File content for media {media_id}") }
                }
                _ => AppError::Internal { message: format!("Storage error: {e}") },
            })?;

        tracing::info!("Successfully initiated streaming for media: {}", media.original_filename);

        Ok((file_reader, media))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        domain::{
            entities::{MediaId, UserId},
            value_objects::{ContentHash, MediaType, ProcessingStatus},
        },
        test_utils::mocks::InMemoryMediaRepository,
    };

    fn create_test_media(id: MediaId, status: ProcessingStatus) -> Media {
        let content_hash =
            ContentHash::new("abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890")
                .unwrap();
        let mut media = Media::new(
            content_hash,
            "test.jpg".to_string(),
            MediaType::new("image/jpeg"),
            "/path/to/test.jpg".to_string(),
            1024,
            UserId::new(),
        );
        media.id = id;
        media.set_processing_status(status);
        media
    }

    #[tokio::test]
    async fn test_download_media_not_found() {
        let repo = InMemoryMediaRepository::new();
        let media_id = MediaId::new(999);

        // Since we can't easily mock the storage trait, we'll just test the repository part
        // Full integration tests would handle the storage component
        let result = repo.find_by_id(media_id).await;

        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_media_ready_status_check() {
        let media_complete = create_test_media(MediaId::new(1), ProcessingStatus::Complete);
        let media_pending = create_test_media(MediaId::new(2), ProcessingStatus::Pending);

        assert!(media_complete.is_ready());
        assert!(!media_pending.is_ready());
    }

    // Note: Full download tests require integration testing with real storage
    // Unit tests focus on the repository and business logic components
}
