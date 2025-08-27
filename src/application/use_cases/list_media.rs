use std::sync::Arc;

use crate::{
    application::dto::{MediaDto, PaginatedMediaQuery, PaginatedMediaResponse, PaginationInfo},
    domain::{
        entities::{Media, UserId},
        repositories::MediaRepository,
    },
    presentation::middleware::error::AppError,
};

/// Use case for listing media with pagination and filtering
pub struct ListMediaUseCase<R>
where
    R: MediaRepository + ?Sized,
{
    repository: Arc<R>,
}

impl<R> ListMediaUseCase<R>
where
    R: MediaRepository + ?Sized,
{
    /// Create a new list media use case
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }

    /// Execute the list media use case
    /// Uses database-level cursor-based pagination for efficient querying
    pub async fn execute(
        &self,
        query: PaginatedMediaQuery,
        user_id: UserId,
    ) -> Result<PaginatedMediaResponse, AppError> {
        tracing::info!("Listing paginated media for user: {} with query: {:?}", user_id, query);

        // Set default limit and validate
        let limit = query.limit.unwrap_or(50).clamp(1, 100);

        // Use repository pagination
        let (media_list, next_cursor, has_more) = self
            .repository
            .find_by_user_paginated(user_id, query.cursor.clone(), limit, query.status)
            .await
            .map_err(|e| AppError::Internal {
                message: format!("Failed to query paginated media: {e}"),
            })?;

        tracing::info!("Found {} media files for user (paginated)", media_list.len());

        // Convert to DTOs
        let media_dtos: Vec<MediaDto> = media_list.into_iter().map(Self::media_to_dto).collect();

        // Determine if there's a previous page based on cursor presence
        let has_prev = query.cursor.is_some();
        let prev_cursor = if has_prev {
            // For simplicity, we'll not implement reverse pagination cursor
            // In a full implementation, you'd track both forward and backward cursors
            None
        } else {
            None
        };

        let pagination = PaginationInfo {
            next_cursor,
            prev_cursor,
            page_size: media_dtos.len() as u32,
            has_next: has_more,
            has_prev,
        };

        let response = PaginatedMediaResponse { data: media_dtos, pagination };

        tracing::info!(
            "Returning paginated response with {} items, has_next: {}, has_prev: {}",
            response.pagination.page_size,
            response.pagination.has_next,
            response.pagination.has_prev
        );

        Ok(response)
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
        let query = PaginatedMediaQuery { cursor: None, limit: None, status: None };

        let result = use_case.execute(query, user_id).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.data.len(), 3);

        // Sort by filename for consistent testing since HashMap order is not guaranteed
        let mut sorted_filenames: Vec<_> =
            response.data.iter().map(|m| &m.original_filename).collect();
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
        let query = PaginatedMediaQuery {
            cursor: None,
            limit: None,
            status: Some(ProcessingStatus::Complete),
        };

        let result = use_case.execute(query, user_id).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.data.len(), 2); // Only Complete files
        assert!(response
            .data
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
        let query = PaginatedMediaQuery { cursor: None, limit: Some(2), status: None };

        let result = use_case.execute(query, user_id).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.data.len(), 2); // Limited to 2 items
                                            // Verify we got 2 items as expected for pagination
        assert!(response.data.len() == 2);
    }

    #[tokio::test]
    async fn test_list_media_empty_result() {
        let repo = InMemoryMediaRepository::new();
        let user_id = UserId::new();

        let use_case = ListMediaUseCase::new(Arc::new(repo));
        let query = PaginatedMediaQuery { cursor: None, limit: None, status: None };

        let result = use_case.execute(query, user_id).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response.data.is_empty());
    }

    #[tokio::test]
    async fn test_list_media_paginated_first_page() {
        let user_id = UserId::new();

        let mut media1 = create_test_media(1, "file1.jpg", ProcessingStatus::Complete, user_id);
        media1.id = MediaId::new(1);
        let mut media2 = create_test_media(2, "file2.jpg", ProcessingStatus::Complete, user_id);
        media2.id = MediaId::new(2);
        let mut media3 = create_test_media(3, "file3.jpg", ProcessingStatus::Complete, user_id);
        media3.id = MediaId::new(3);

        let repo =
            InMemoryMediaRepository::new().with_media(media1).with_media(media2).with_media(media3);

        let use_case = ListMediaUseCase::new(Arc::new(repo));
        let query = PaginatedMediaQuery { cursor: None, limit: Some(2), status: None };

        let result = use_case.execute(query, user_id).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.data.len(), 2);
        assert!(!response.pagination.has_prev);
        assert!(response.pagination.has_next);
        assert!(response.pagination.next_cursor.is_some());
    }

    #[tokio::test]
    async fn test_list_media_paginated_with_cursor() {
        let user_id = UserId::new();

        let mut media1 = create_test_media(1, "file1.jpg", ProcessingStatus::Complete, user_id);
        media1.id = MediaId::new(1);
        let mut media2 = create_test_media(2, "file2.jpg", ProcessingStatus::Complete, user_id);
        media2.id = MediaId::new(2);
        let mut media3 = create_test_media(3, "file3.jpg", ProcessingStatus::Complete, user_id);
        media3.id = MediaId::new(3);

        let repo =
            InMemoryMediaRepository::new().with_media(media1).with_media(media2).with_media(media3);

        let use_case = ListMediaUseCase::new(Arc::new(repo));

        // Get first page
        let first_query = PaginatedMediaQuery { cursor: None, limit: Some(1), status: None };

        let first_result = use_case.execute(first_query, user_id).await;
        assert!(first_result.is_ok());
        let first_response = first_result.unwrap();
        assert_eq!(first_response.data.len(), 1);
        assert!(first_response.pagination.next_cursor.is_some());

        // Use cursor for second page
        let second_query = PaginatedMediaQuery {
            cursor: first_response.pagination.next_cursor,
            limit: Some(1),
            status: None,
        };

        let second_result = use_case.execute(second_query, user_id).await;
        assert!(second_result.is_ok());
        let second_response = second_result.unwrap();
        assert_eq!(second_response.data.len(), 1);
        assert!(second_response.pagination.has_prev);
    }

    #[tokio::test]
    async fn test_list_media_paginated_with_status_filter() {
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
        let query = PaginatedMediaQuery {
            cursor: None,
            limit: Some(10),
            status: Some(ProcessingStatus::Complete),
        };

        let result = use_case.execute(query, user_id).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.data.len(), 2); // Only Complete status
        assert!(response
            .data
            .iter()
            .all(|m| matches!(m.processing_status, ProcessingStatus::Complete)));
        assert!(!response.pagination.has_next);
    }

    #[tokio::test]
    async fn test_list_media_paginated_empty_result() {
        let repo = InMemoryMediaRepository::new();
        let user_id = UserId::new();

        let use_case = ListMediaUseCase::new(Arc::new(repo));
        let query = PaginatedMediaQuery { cursor: None, limit: Some(50), status: None };

        let result = use_case.execute(query, user_id).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response.data.is_empty());
        assert!(!response.pagination.has_next);
        assert!(!response.pagination.has_prev);
        assert!(response.pagination.next_cursor.is_none());
    }

    #[tokio::test]
    async fn test_list_media_paginated_limit_validation() {
        let user_id = UserId::new();

        let mut media1 = create_test_media(1, "file1.jpg", ProcessingStatus::Complete, user_id);
        media1.id = MediaId::new(1);

        let repo = InMemoryMediaRepository::new().with_media(media1);
        let use_case = ListMediaUseCase::new(Arc::new(repo));

        // Test default limit
        let query_no_limit = PaginatedMediaQuery { cursor: None, limit: None, status: None };

        let result = use_case.execute(query_no_limit, user_id).await;
        assert!(result.is_ok());

        // Test limit too high (should be capped at 100)
        let query_high_limit = PaginatedMediaQuery { cursor: None, limit: Some(200), status: None };

        let result = use_case.execute(query_high_limit, user_id).await;
        assert!(result.is_ok());

        // Test limit too low (should be minimum 1)
        let query_low_limit = PaginatedMediaQuery { cursor: None, limit: Some(0), status: None };

        let result = use_case.execute(query_low_limit, user_id).await;
        assert!(result.is_ok());
    }

    // Repository error testing is better handled in integration tests
}
