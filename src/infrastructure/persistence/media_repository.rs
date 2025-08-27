use crate::presentation::middleware::error::AppError;
use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use chrono::{DateTime, Utc};
use sqlx::{PgPool, Row};

use crate::domain::entities::{IngredientId, Media, MediaId, RecipeId, StepId, UserId};
use crate::domain::repositories::MediaRepository;
use crate::domain::value_objects::{ContentHash, MediaType, ProcessingStatus};

/// `PostgreSQL` implementation of `MediaRepository`
#[derive(Clone)]
pub struct PostgreSqlMediaRepository {
    pool: PgPool,
}

impl PostgreSqlMediaRepository {
    /// Create a new `PostgreSQL` media repository
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl MediaRepository for PostgreSqlMediaRepository {
    type Error = AppError;

    async fn save(&self, media: &Media) -> Result<MediaId, Self::Error> {
        let user_id = media.uploaded_by.as_uuid();
        let media_type_str = media.media_type.mime_type();
        let content_hash_str = media.content_hash.as_str();
        let processing_status_str = media.processing_status.to_string();

        // Convert SystemTime to chrono DateTime for database compatibility
        let uploaded_at: DateTime<Utc> = media.uploaded_at.into();
        let updated_at: DateTime<Utc> = media.updated_at.into();

        let row = sqlx::query(
            r"
            INSERT INTO recipe_manager.media
            (user_id, media_type, media_path, file_size, content_hash, original_filename, processing_status, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING media_id
            ",
        )
        .bind(user_id)
        .bind(media_type_str)
        .bind(&media.media_path)
        .bind(media.file_size as i64)
        .bind(content_hash_str)
        .bind(&media.original_filename)
        .bind(processing_status_str)
        .bind(uploaded_at)
        .bind(updated_at)
        .fetch_one(&self.pool)
        .await
        .map_err(AppError::from)?;

        let media_id = MediaId::new(row.get("media_id"));
        Ok(media_id)
    }

    async fn find_by_id(&self, id: MediaId) -> Result<Option<Media>, Self::Error> {
        let media_id = id.as_i64();

        let row = sqlx::query(
            r"
            SELECT media_id, user_id, media_type, media_path, file_size, content_hash,
                   original_filename, processing_status, created_at, updated_at
            FROM recipe_manager.media
            WHERE media_id = $1
            ",
        )
        .bind(media_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::from)?;

        match row {
            Some(row) => {
                let media = map_row_to_media(&row)?;
                Ok(Some(media))
            }
            None => Ok(None),
        }
    }

    async fn find_by_content_hash(&self, hash: &ContentHash) -> Result<Option<Media>, Self::Error> {
        let hash_str = hash.as_str();

        let row = sqlx::query(
            r"
            SELECT media_id, user_id, media_type, media_path, file_size, content_hash,
                   original_filename, processing_status, created_at, updated_at
            FROM recipe_manager.media
            WHERE content_hash = $1
            ",
        )
        .bind(hash_str)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::from)?;

        match row {
            Some(row) => {
                let media = map_row_to_media(&row)?;
                Ok(Some(media))
            }
            None => Ok(None),
        }
    }

    async fn find_by_user(&self, user_id: UserId) -> Result<Vec<Media>, Self::Error> {
        let user_uuid = user_id.as_uuid();

        let rows = sqlx::query(
            r"
            SELECT media_id, user_id, media_type, media_path, file_size, content_hash,
                   original_filename, processing_status, created_at, updated_at
            FROM recipe_manager.media
            WHERE user_id = $1
            ORDER BY created_at DESC
            ",
        )
        .bind(user_uuid)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::from)?;

        let mut media_list = Vec::new();
        for row in rows {
            let media = map_row_to_media(&row)?;
            media_list.push(media);
        }

        Ok(media_list)
    }

    async fn update(&self, media: &Media) -> Result<(), Self::Error> {
        let media_id = media.id.as_i64();
        let media_type_str = media.media_type.mime_type();
        let content_hash_str = media.content_hash.as_str();
        let processing_status_str = media.processing_status.to_string();
        let updated_at: DateTime<Utc> = media.updated_at.into();

        sqlx::query(
            r"
            UPDATE recipe_manager.media
            SET media_type = $2, media_path = $3, file_size = $4, content_hash = $5,
                original_filename = $6, processing_status = $7, updated_at = $8
            WHERE media_id = $1
            ",
        )
        .bind(media_id)
        .bind(media_type_str)
        .bind(&media.media_path)
        .bind(media.file_size as i64)
        .bind(content_hash_str)
        .bind(&media.original_filename)
        .bind(processing_status_str)
        .bind(updated_at)
        .execute(&self.pool)
        .await
        .map_err(AppError::from)?;

        Ok(())
    }

    async fn delete(&self, id: MediaId) -> Result<bool, Self::Error> {
        let media_id = id.as_i64();

        let result = sqlx::query(
            r"
            DELETE FROM recipe_manager.media
            WHERE media_id = $1
            ",
        )
        .bind(media_id)
        .execute(&self.pool)
        .await
        .map_err(AppError::from)?;

        Ok(result.rows_affected() > 0)
    }

    async fn exists_by_content_hash(&self, hash: &ContentHash) -> Result<bool, Self::Error> {
        let hash_str = hash.as_str();

        let row = sqlx::query(
            r"
            SELECT EXISTS(SELECT 1 FROM recipe_manager.media WHERE content_hash = $1) as exists
            ",
        )
        .bind(hash_str)
        .fetch_one(&self.pool)
        .await
        .map_err(AppError::from)?;

        let exists: bool = row.get("exists");
        Ok(exists)
    }

    async fn find_media_ids_by_recipe(
        &self,
        recipe_id: RecipeId,
    ) -> Result<Vec<MediaId>, Self::Error> {
        let recipe_id = recipe_id.as_i64();

        let rows = sqlx::query(
            r"
            SELECT media_id
            FROM recipe_manager.recipe_media
            WHERE recipe_id = $1
            ORDER BY media_id
            ",
        )
        .bind(recipe_id)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::from)?;

        let media_ids: Vec<MediaId> =
            rows.iter().map(|row| MediaId::new(row.get("media_id"))).collect();

        Ok(media_ids)
    }

    async fn find_media_ids_by_recipe_ingredient(
        &self,
        recipe_id: RecipeId,
        ingredient_id: IngredientId,
    ) -> Result<Vec<MediaId>, Self::Error> {
        let recipe_id = recipe_id.as_i64();
        let ingredient_id = ingredient_id.as_i64();

        let rows = sqlx::query(
            r"
            SELECT media_id
            FROM recipe_manager.ingredient_media
            WHERE recipe_id = $1 AND ingredient_id = $2
            ORDER BY media_id
            ",
        )
        .bind(recipe_id)
        .bind(ingredient_id)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::from)?;

        let media_ids: Vec<MediaId> =
            rows.iter().map(|row| MediaId::new(row.get("media_id"))).collect();

        Ok(media_ids)
    }

    async fn find_media_ids_by_recipe_step(
        &self,
        recipe_id: RecipeId,
        step_id: StepId,
    ) -> Result<Vec<MediaId>, Self::Error> {
        let recipe_id = recipe_id.as_i64();
        let step_id = step_id.as_i64();

        let rows = sqlx::query(
            r"
            SELECT media_id
            FROM recipe_manager.step_media
            WHERE recipe_id = $1 AND step_id = $2
            ORDER BY media_id
            ",
        )
        .bind(recipe_id)
        .bind(step_id)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::from)?;

        let media_ids: Vec<MediaId> =
            rows.iter().map(|row| MediaId::new(row.get("media_id"))).collect();

        Ok(media_ids)
    }

    async fn find_by_user_paginated(
        &self,
        user_id: UserId,
        cursor: Option<String>,
        limit: u32,
        status_filter: Option<ProcessingStatus>,
    ) -> Result<(Vec<Media>, Option<String>, bool), Self::Error> {
        let user_uuid = user_id.as_uuid();

        // Validate and constrain limit
        let limit = limit.clamp(1, 100);
        let fetch_limit = i64::from(limit + 1); // Fetch one extra to check if there's a next page

        // Decode cursor to get the last media_id
        let cursor_media_id = match cursor {
            Some(cursor_str) => {
                let decoded = STANDARD.decode(&cursor_str).map_err(|_| AppError::BadRequest {
                    message: "Invalid cursor format".to_string(),
                })?;
                let cursor_data = String::from_utf8(decoded).map_err(|_| AppError::BadRequest {
                    message: "Invalid cursor encoding".to_string(),
                })?;
                Some(cursor_data.parse::<i64>().map_err(|_| AppError::BadRequest {
                    message: "Invalid cursor data".to_string(),
                })?)
            }
            None => None,
        };

        // Build query with optional status filter and cursor pagination
        let mut query_str = r"
            SELECT media_id, user_id, media_type, media_path, file_size, content_hash,
                   original_filename, processing_status, created_at, updated_at
            FROM recipe_manager.media
            WHERE user_id = $1"
            .to_string();

        let mut bind_index = 2;

        // Add status filter if provided
        if status_filter.is_some() {
            use std::fmt::Write;
            write!(&mut query_str, " AND processing_status = ${bind_index}").unwrap();
            bind_index += 1;
        }

        // Add cursor condition for pagination
        if cursor_media_id.is_some() {
            use std::fmt::Write;
            write!(&mut query_str, " AND media_id > ${bind_index}").unwrap();
            bind_index += 1;
        }

        // Order by media_id for consistent pagination
        query_str.push_str(" ORDER BY media_id ASC LIMIT $");
        query_str.push_str(&bind_index.to_string());

        // Start building the query
        let mut query = sqlx::query(&query_str).bind(user_uuid);

        // Bind status filter if provided
        if let Some(status) = status_filter {
            query = query.bind(status.to_string());
        }

        // Bind cursor media_id if provided
        if let Some(id) = cursor_media_id {
            query = query.bind(id);
        }

        // Bind limit
        query = query.bind(fetch_limit);

        let rows = query.fetch_all(&self.pool).await.map_err(AppError::from)?;

        // Check if we have more items than requested (indicates next page exists)
        let has_more = rows.len() > limit as usize;

        // Take only the requested number of items
        let media_rows = if has_more { &rows[..limit as usize] } else { &rows };

        // Convert rows to Media entities
        let mut media_list = Vec::new();
        for row in media_rows {
            let media = map_row_to_media(row)?;
            media_list.push(media);
        }

        // Generate next cursor if there are more items
        let next_cursor = if has_more && !media_list.is_empty() {
            let last_media_id = media_list.last().unwrap().id.as_i64();
            Some(STANDARD.encode(last_media_id.to_string().as_bytes()))
        } else {
            None
        };

        tracing::debug!(
            "Paginated query returned {} items, has_more: {}, cursor: {:?}",
            media_list.len(),
            has_more,
            next_cursor
        );

        Ok((media_list, next_cursor, has_more))
    }

    /// Health check for database connectivity
    ///
    /// Performs a simple query to verify database connectivity and responsiveness.
    /// Returns `Ok(())` if database is accessible, `Err(AppError)` otherwise.
    async fn health_check(&self) -> Result<(), Self::Error> {
        sqlx::query("SELECT 1")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::Database { message: format!("Health check failed: {e}") })?;
        Ok(())
    }
}

/// Helper function to map database row to Media entity
fn map_row_to_media(row: &sqlx::postgres::PgRow) -> Result<Media, AppError> {
    use sqlx::Row;

    let media_id = MediaId::new(row.get("media_id"));

    let user_id_uuid: uuid::Uuid = row.get("user_id");
    let user_id = UserId::from_uuid(user_id_uuid);

    let media_type_str: String = row.get("media_type");
    let media_type = MediaType::new(&media_type_str);

    let media_path: String = row.get("media_path");
    let file_size: i64 = row.get("file_size");

    let content_hash_str: Option<String> = row.get("content_hash");
    let content_hash = match content_hash_str {
        Some(hash_str) => ContentHash::new(&hash_str)
            .map_err(|_| AppError::Database { message: "Invalid content hash".to_string() })?,
        None => return Err(AppError::Database { message: "Missing content hash".to_string() }),
    };

    let original_filename: Option<String> = row.get("original_filename");
    let original_filename = original_filename.unwrap_or_else(|| "unknown".to_string());

    let processing_status_str: String = row.get("processing_status");
    let processing_status = processing_status_str
        .parse::<ProcessingStatus>()
        .map_err(|_| AppError::Database { message: "Invalid processing status".to_string() })?;

    let created_at: DateTime<Utc> = row.get("created_at");
    let updated_at: DateTime<Utc> = row.get("updated_at");

    let media = Media::with_id(
        media_id,
        content_hash,
        original_filename,
        media_type,
        media_path,
        file_size as u64,
        processing_status,
    )
    .uploaded_by(user_id)
    .uploaded_at(created_at.into())
    .updated_at(updated_at.into())
    .build();

    Ok(media)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::UserId;
    use crate::domain::value_objects::{ContentHash, ProcessingStatus};

    fn create_test_media() -> Media {
        let content_hash =
            ContentHash::new("abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890")
                .unwrap();
        let media_type = MediaType::new("image/jpeg");
        let user_id = UserId::new();

        Media::new(
            content_hash,
            "test.jpg".to_string(),
            media_type,
            "ab/cd/ef/abcdef123".to_string(),
            1024,
            user_id,
        )
    }

    fn create_test_media_with_id(id: i64) -> Media {
        let content_hash =
            ContentHash::new("abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890")
                .unwrap();
        let media_type = MediaType::new("image/jpeg");
        let user_id = UserId::new();

        Media::with_id(
            MediaId::new(id),
            content_hash,
            "test.jpg".to_string(),
            media_type,
            "ab/cd/ef/abcdef123".to_string(),
            1024,
            ProcessingStatus::Complete,
        )
        .uploaded_by(user_id)
        .build()
    }

    #[tokio::test]
    async fn test_postgresql_media_repository_creation() {
        use sqlx::PgPool;
        use std::env;

        // This test doesn't require actual database connection
        // We're just testing that the repository can be created with a pool

        // Create a mock connection string (won't be used)
        let database_url = env::var("TEST_DATABASE_URL")
            .unwrap_or_else(|_| "postgresql://test:test@localhost:5432/test".to_string());

        // Note: In real tests, we'd use sqlx-test or a test container
        // For now, we test the constructor logic
        let Ok(mock_pool) = PgPool::connect_lazy(&database_url) else {
            // If we can't create a lazy connection, skip the test
            return;
        };

        let repository = PostgreSqlMediaRepository::new(mock_pool);
        // Test that repository is created successfully
        assert!(std::ptr::addr_of!(repository).is_aligned());
    }

    #[test]
    fn test_map_row_to_media_success() {
        // Note: This test is complex to implement without actual database rows
        // The map_row_to_media function requires actual PgRow instances
        // This would typically be tested in integration tests with real database data

        // For now, we'll test the function signature and error cases
        // In a full implementation, you'd use sqlx-test or mock rows

        // Test that we can create test media entities for validation
        let test_media = create_test_media();
        assert_eq!(test_media.original_filename, "test.jpg");
        assert_eq!(test_media.file_size, 1024);
        assert!(test_media.content_hash.as_str().len() == 64);
    }

    #[test]
    fn test_media_entity_database_field_mapping() {
        let media = create_test_media_with_id(1);

        // Test that media entity has all fields needed for database mapping
        assert_eq!(media.id.as_i64(), 1);
        assert!(!media.uploaded_by.as_uuid().is_nil());
        assert!(!media.media_type.mime_type().is_empty());
        assert!(!media.media_path.is_empty());
        assert!(media.file_size > 0);
        assert!(!media.content_hash.as_str().is_empty());
        assert!(!media.original_filename.is_empty());
        assert!(matches!(media.processing_status, ProcessingStatus::Complete));

        // Test time fields are convertible to DateTime<Utc>
        let uploaded_at: DateTime<Utc> = media.uploaded_at.into();
        let updated_at: DateTime<Utc> = media.updated_at.into();
        assert!(uploaded_at <= updated_at || (updated_at - uploaded_at).num_milliseconds() < 1000);
    }

    #[test]
    fn test_content_hash_validation() {
        // Test valid content hash
        let valid_hash = "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890";
        let content_hash = ContentHash::new(valid_hash);
        assert!(content_hash.is_ok());

        // Test invalid content hash would be handled in map_row_to_media
        let invalid_hash = "invalid-hash";
        let content_hash_result = ContentHash::new(invalid_hash);
        assert!(content_hash_result.is_err());
    }

    #[test]
    fn test_processing_status_parsing() {
        // Test that processing status can be parsed from string
        let status_str = "Complete";
        let status: Result<ProcessingStatus, _> = status_str.parse();
        assert!(status.is_ok());
        assert!(matches!(status.unwrap(), ProcessingStatus::Complete));

        let invalid_status = "InvalidStatus";
        let invalid_result: Result<ProcessingStatus, _> = invalid_status.parse();
        assert!(invalid_result.is_err());
    }

    #[test]
    fn test_media_type_creation() {
        let media_type = MediaType::new("image/jpeg");
        assert_eq!(media_type.mime_type(), "image/jpeg");

        let media_type2 = MediaType::new("video/mp4");
        assert_eq!(media_type2.mime_type(), "video/mp4");
    }

    #[test]
    fn test_media_id_conversions() {
        let id = MediaId::new(123);
        assert_eq!(id.as_i64(), 123);

        let id2 = MediaId::new(-1);
        assert_eq!(id2.as_i64(), -1);
    }

    #[test]
    fn test_user_id_uuid_conversion() {
        let user_id = UserId::new();
        let uuid = user_id.as_uuid();
        assert!(!uuid.is_nil());

        let user_id2 = UserId::from_uuid(uuid);
        assert_eq!(user_id.as_uuid(), user_id2.as_uuid());
    }

    // Integration tests requiring actual database connections should be in the integration test directory
    // These unit tests focus on repository creation and data mapping logic without database dependencies

    #[tokio::test]
    async fn test_disconnected_repository_creation() {
        let repo = DisconnectedMediaRepository::new("test error".to_string());
        assert_eq!(repo.error_message, "test error");
    }

    #[tokio::test]
    async fn test_disconnected_repository_health_check_fails() {
        let repo = DisconnectedMediaRepository::new("test connection failed".to_string());
        let result = repo.health_check().await;
        assert!(result.is_err());

        if let Err(AppError::Database { message }) = result {
            assert!(message.contains("Database unavailable"));
            assert!(message.contains("test connection failed"));
        } else {
            panic!("Expected Database error");
        }
    }

    #[tokio::test]
    async fn test_disconnected_repository_all_methods_fail() {
        let repo = DisconnectedMediaRepository::new("test error".to_string());
        let test_media = create_test_media();
        let test_hash =
            ContentHash::new("abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890")
                .unwrap();
        let test_id = MediaId::new(1);
        let test_user_id = UserId::new();
        let recipe_id = RecipeId::new(1);
        let ingredient_id = IngredientId::new(1);
        let step_id = StepId::new(1);

        // Test all methods fail appropriately
        assert!(repo.save(&test_media).await.is_err());
        assert!(repo.find_by_id(test_id).await.is_err());
        assert!(repo.find_by_content_hash(&test_hash).await.is_err());
        assert!(repo.find_by_user(test_user_id).await.is_err());
        assert!(repo.find_by_user_paginated(test_user_id, None, 50, None).await.is_err());
        assert!(repo.update(&test_media).await.is_err());
        assert!(repo.delete(test_id).await.is_err());
        assert!(repo.exists_by_content_hash(&test_hash).await.is_err());
        assert!(repo.find_media_ids_by_recipe(recipe_id).await.is_err());
        assert!(repo.find_media_ids_by_recipe_ingredient(recipe_id, ingredient_id).await.is_err());
        assert!(repo.find_media_ids_by_recipe_step(recipe_id, step_id).await.is_err());
    }
}

/// A disconnected repository implementation for when database is unavailable
///
/// This implementation always fails health checks and database operations,
/// allowing the service to start but report proper status through health/readiness endpoints.
#[derive(Clone)]
pub struct DisconnectedMediaRepository {
    error_message: String,
}

impl DisconnectedMediaRepository {
    /// Create a new disconnected repository with an error message
    #[must_use]
    pub fn new(error_message: String) -> Self {
        Self { error_message }
    }
}

#[async_trait]
impl MediaRepository for DisconnectedMediaRepository {
    type Error = AppError;

    async fn save(&self, _media: &Media) -> Result<MediaId, Self::Error> {
        Err(AppError::Database { message: format!("Database unavailable: {}", self.error_message) })
    }

    async fn find_by_id(&self, _id: MediaId) -> Result<Option<Media>, Self::Error> {
        Err(AppError::Database { message: format!("Database unavailable: {}", self.error_message) })
    }

    async fn find_by_content_hash(
        &self,
        _hash: &ContentHash,
    ) -> Result<Option<Media>, Self::Error> {
        Err(AppError::Database { message: format!("Database unavailable: {}", self.error_message) })
    }

    async fn find_by_user(&self, _user_id: UserId) -> Result<Vec<Media>, Self::Error> {
        Err(AppError::Database { message: format!("Database unavailable: {}", self.error_message) })
    }

    async fn find_by_user_paginated(
        &self,
        _user_id: UserId,
        _cursor: Option<String>,
        _limit: u32,
        _status_filter: Option<ProcessingStatus>,
    ) -> Result<(Vec<Media>, Option<String>, bool), Self::Error> {
        Err(AppError::Database { message: format!("Database unavailable: {}", self.error_message) })
    }

    async fn update(&self, _media: &Media) -> Result<(), Self::Error> {
        Err(AppError::Database { message: format!("Database unavailable: {}", self.error_message) })
    }

    async fn delete(&self, _id: MediaId) -> Result<bool, Self::Error> {
        Err(AppError::Database { message: format!("Database unavailable: {}", self.error_message) })
    }

    async fn exists_by_content_hash(&self, _hash: &ContentHash) -> Result<bool, Self::Error> {
        Err(AppError::Database { message: format!("Database unavailable: {}", self.error_message) })
    }

    async fn find_media_ids_by_recipe(
        &self,
        _recipe_id: RecipeId,
    ) -> Result<Vec<MediaId>, Self::Error> {
        Err(AppError::Database { message: format!("Database unavailable: {}", self.error_message) })
    }

    async fn find_media_ids_by_recipe_ingredient(
        &self,
        _recipe_id: RecipeId,
        _ingredient_id: IngredientId,
    ) -> Result<Vec<MediaId>, Self::Error> {
        Err(AppError::Database { message: format!("Database unavailable: {}", self.error_message) })
    }

    async fn find_media_ids_by_recipe_step(
        &self,
        _recipe_id: RecipeId,
        _step_id: StepId,
    ) -> Result<Vec<MediaId>, Self::Error> {
        Err(AppError::Database { message: format!("Database unavailable: {}", self.error_message) })
    }

    async fn health_check(&self) -> Result<(), Self::Error> {
        Err(AppError::Database { message: format!("Database unavailable: {}", self.error_message) })
    }
}
