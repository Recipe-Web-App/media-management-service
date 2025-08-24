use std::sync::Arc;

use crate::{
    application::dto::{ListMediaQuery, MediaDto},
    domain::{
        entities::{Media, UserId},
        repositories::MediaRepository,
    },
    presentation::middleware::error::AppError,
};

/// Use case for listing media with pagination and filtering
pub struct ListMediaUseCase<R>
where
    R: MediaRepository,
{
    repository: Arc<R>,
}

impl<R> ListMediaUseCase<R>
where
    R: MediaRepository,
{
    /// Create a new list media use case
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }

    /// Execute the list media use case
    /// Note: Current implementation lists all media for a user and applies filtering/pagination in memory
    /// In a production system, this should be implemented with database-level filtering and pagination
    pub async fn execute(
        &self,
        query: ListMediaQuery,
        user_id: UserId,
    ) -> Result<Vec<MediaDto>, AppError> {
        tracing::info!("Listing media for user: {} with query: {:?}", user_id, query);

        // Get all media for user (in production, this should be paginated at DB level)
        let all_media =
            self.repository.find_by_user(user_id).await.map_err(|e| AppError::Internal {
                message: format!("Failed to query media: {e}"),
            })?;

        tracing::info!("Found {} total media files for user", all_media.len());

        // Apply filtering
        let filtered_media: Vec<_> =
            all_media.into_iter().filter(|media| Self::matches_filter(media, &query)).collect();

        tracing::info!("After filtering: {} media files", filtered_media.len());

        // Apply pagination
        let offset = query.offset.unwrap_or(0) as usize;
        let limit = query.limit.unwrap_or(50) as usize;

        let paginated_media: Vec<_> = filtered_media.into_iter().skip(offset).take(limit).collect();

        tracing::info!("After pagination: {} media files", paginated_media.len());

        // Convert to DTOs
        let media_dtos: Vec<MediaDto> =
            paginated_media.into_iter().map(Self::media_to_dto).collect();

        Ok(media_dtos)
    }

    /// Execute list media for all users (admin function)
    /// Note: This is a simplified implementation for development
    /// In production, this would need proper admin authorization and database-level pagination
    pub fn execute_admin(&self, _query: ListMediaQuery) -> Result<Vec<MediaDto>, AppError> {
        tracing::warn!(
            "Admin list media called - this should be properly authorized in production"
        );

        // For now, we'll return an empty list since we don't have a "list all" method in the repository
        // In a real implementation, we'd add a `find_all_with_pagination` method to the repository
        tracing::info!("Admin media listing not fully implemented - returning empty list");

        Ok(Vec::new())
    }

    /// Check if media matches the filter criteria
    fn matches_filter(media: &Media, query: &ListMediaQuery) -> bool {
        // Filter by processing status if specified
        if let Some(ref status_filter) = query.status {
            if &media.processing_status != status_filter {
                return false;
            }
        }

        true
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
            value_objects::{ContentHash, MediaType, ProcessingStatus},
        },
        test_utils::mocks::InMemoryMediaRepository,
    };

    fn create_test_media(
        id: i64,
        filename: &str,
        status: ProcessingStatus,
        user_id: UserId,
    ) -> Media {
        let content_hash = ContentHash::new(&format!("{:0>64}", id.to_string())).unwrap();
        let mut media = Media::new(
            content_hash,
            filename.to_string(),
            MediaType::new("image/jpeg"),
            format!("/path/to/{filename}"),
            1024,
            user_id,
        );
        media.set_processing_status(status);
        media
    }

    #[tokio::test]
    async fn test_list_media_success() {
        let user_id = UserId::new();

        let mut media1 = create_test_media(1, "file1.jpg", ProcessingStatus::Complete, user_id);
        media1.id = MediaId::new(1);
        let mut media2 = create_test_media(2, "file2.jpg", ProcessingStatus::Pending, user_id);
        media2.id = MediaId::new(2);
        let mut media3 = create_test_media(3, "file3.jpg", ProcessingStatus::Complete, user_id);
        media3.id = MediaId::new(3);

        let repo =
            InMemoryMediaRepository::new().with_media(media1).with_media(media2).with_media(media3);

        let use_case = ListMediaUseCase::new(Arc::new(repo));
        let query = ListMediaQuery { limit: None, offset: None, status: None };

        let result = use_case.execute(query, user_id).await;

        assert!(result.is_ok());
        let media_list = result.unwrap();
        assert_eq!(media_list.len(), 3);

        // Sort by filename for consistent testing since HashMap order is not guaranteed
        let mut sorted_filenames: Vec<_> =
            media_list.iter().map(|m| &m.original_filename).collect();
        sorted_filenames.sort();
        assert_eq!(sorted_filenames, vec!["file1.jpg", "file2.jpg", "file3.jpg"]);
    }

    #[tokio::test]
    async fn test_list_media_with_status_filter() {
        let user_id = UserId::new();

        let mut media1 = create_test_media(1, "file1.jpg", ProcessingStatus::Complete, user_id);
        media1.id = MediaId::new(1);

        let mut media2 = create_test_media(2, "file2.jpg", ProcessingStatus::Pending, user_id);
        media2.id = MediaId::new(2);

        let mut media3 = create_test_media(3, "file3.jpg", ProcessingStatus::Complete, user_id);
        media3.id = MediaId::new(3);

        let repo =
            InMemoryMediaRepository::new().with_media(media1).with_media(media2).with_media(media3);

        let use_case = ListMediaUseCase::new(Arc::new(repo));
        let query =
            ListMediaQuery { limit: None, offset: None, status: Some(ProcessingStatus::Complete) };

        let result = use_case.execute(query, user_id).await;

        assert!(result.is_ok());
        let media_list = result.unwrap();
        assert_eq!(media_list.len(), 2); // Only Complete files
        assert!(media_list
            .iter()
            .all(|m| matches!(m.processing_status, ProcessingStatus::Complete)));
    }

    #[tokio::test]
    async fn test_list_media_with_pagination() {
        let user_id = UserId::new();

        let mut media1 = create_test_media(1, "file1.jpg", ProcessingStatus::Complete, user_id);
        media1.id = MediaId::new(1);
        let mut media2 = create_test_media(2, "file2.jpg", ProcessingStatus::Complete, user_id);
        media2.id = MediaId::new(2);
        let mut media3 = create_test_media(3, "file3.jpg", ProcessingStatus::Complete, user_id);
        media3.id = MediaId::new(3);
        let mut media4 = create_test_media(4, "file4.jpg", ProcessingStatus::Complete, user_id);
        media4.id = MediaId::new(4);

        let repo = InMemoryMediaRepository::new()
            .with_media(media1)
            .with_media(media2)
            .with_media(media3)
            .with_media(media4);

        let use_case = ListMediaUseCase::new(Arc::new(repo));
        let query = ListMediaQuery { limit: Some(2), offset: Some(1), status: None };

        let result = use_case.execute(query, user_id).await;

        assert!(result.is_ok());
        let media_list = result.unwrap();
        assert_eq!(media_list.len(), 2); // Limited to 2 items after offset
                                         // Verify we got 2 items as expected for pagination
        assert!(media_list.len() == 2);
    }

    #[tokio::test]
    async fn test_list_media_empty_result() {
        let repo = InMemoryMediaRepository::new();
        let user_id = UserId::new();

        let use_case = ListMediaUseCase::new(Arc::new(repo));
        let query = ListMediaQuery { limit: None, offset: None, status: None };

        let result = use_case.execute(query, user_id).await;

        assert!(result.is_ok());
        let media_list = result.unwrap();
        assert!(media_list.is_empty());
    }

    // Repository error testing is better handled in integration tests
}
