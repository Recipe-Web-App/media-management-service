use std::sync::Arc;

use crate::{
    application::dto::MediaDto,
    domain::{
        entities::{Media, MediaId},
        repositories::MediaRepository,
    },
    presentation::middleware::error::AppError,
};

/// Use case for retrieving media metadata by ID
pub struct GetMediaUseCase<R>
where
    R: MediaRepository + ?Sized,
{
    repository: Arc<R>,
}

impl<R> GetMediaUseCase<R>
where
    R: MediaRepository + ?Sized,
{
    /// Create a new get media use case
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }

    /// Execute the get media use case
    pub async fn execute(&self, media_id: MediaId) -> Result<MediaDto, AppError> {
        tracing::info!("Getting media with ID: {}", media_id);

        let media =
            self.repository.find_by_id(media_id).await.map_err(|e| AppError::Internal {
                message: format!("Failed to query media: {e}"),
            })?;

        if let Some(media) = media {
            tracing::info!("Found media: {} ({})", media.original_filename, media.id);
            Ok(Self::media_to_dto(media))
        } else {
            tracing::warn!("Media not found with ID: {}", media_id);
            Err(AppError::NotFound { resource: format!("Media with ID {media_id}") })
        }
    }

    /// Convert Media entity to `MediaDto`
    fn media_to_dto(media: Media) -> MediaDto {
        MediaDto {
            id: media.id,
            content_hash: media.content_hash.as_str().to_string(),
            original_filename: media.original_filename,
            media_type: media.media_type.mime_type().to_string(),
            media_path: media.media_path,
            file_size: media.file_size,
            processing_status: media.processing_status,
            uploaded_at: media.uploaded_at.duration_since(std::time::UNIX_EPOCH).map_or_else(
                |_| chrono::Utc::now().to_rfc3339(),
                |d| {
                    chrono::DateTime::<chrono::Utc>::from_timestamp(
                        d.as_secs() as i64,
                        d.subsec_nanos(),
                    )
                    .unwrap_or_default()
                    .to_rfc3339()
                },
            ),
            updated_at: media.updated_at.duration_since(std::time::UNIX_EPOCH).map_or_else(
                |_| chrono::Utc::now().to_rfc3339(),
                |d| {
                    chrono::DateTime::<chrono::Utc>::from_timestamp(
                        d.as_secs() as i64,
                        d.subsec_nanos(),
                    )
                    .unwrap_or_default()
                    .to_rfc3339()
                },
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        domain::{
            entities::{MediaId, UserId},
            value_objects::{ContentHash, MediaType},
        },
        test_utils::mocks::InMemoryMediaRepository,
    };

    #[tokio::test]
    async fn test_get_media_success() {
        let media_id = MediaId::new(123);
        let content_hash =
            ContentHash::new("abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890")
                .unwrap();
        let mut expected_media = Media::new(
            content_hash.clone(),
            "test.jpg".to_string(),
            MediaType::new("image/jpeg"),
            "/path/to/file".to_string(),
            1024,
            UserId::new(),
        );
        expected_media.id = media_id;

        let repo = InMemoryMediaRepository::new().with_media(expected_media);
        let use_case = GetMediaUseCase::new(Arc::new(repo));
        let result = use_case.execute(media_id).await;

        assert!(result.is_ok());
        let dto = result.unwrap();
        assert_eq!(dto.id, media_id);
        assert_eq!(dto.content_hash, content_hash.as_str());
        assert_eq!(dto.original_filename, "test.jpg");
        assert_eq!(dto.media_type, "image/jpeg");
        assert_eq!(dto.file_size, 1024);
    }

    #[tokio::test]
    async fn test_get_media_not_found() {
        let repo = InMemoryMediaRepository::new();
        let media_id = MediaId::new(999);

        let use_case = GetMediaUseCase::new(Arc::new(repo));
        let result = use_case.execute(media_id).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::NotFound { resource } => {
                assert!(resource.contains("999"));
            }
            _ => panic!("Expected NotFound error"),
        }
    }

    // Repository error testing would require more complex error injection
    // This is better tested with integration tests
}
