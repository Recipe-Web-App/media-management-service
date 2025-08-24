use crate::presentation::middleware::error::AppError;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{PgPool, Row};

use crate::domain::entities::{Media, MediaId, UserId};
use crate::domain::repositories::MediaRepository;
use crate::domain::value_objects::{ContentHash, MediaType, ProcessingStatus};

/// `PostgreSQL` implementation of `MediaRepository`
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

    async fn save(&self, media: &Media) -> Result<(), Self::Error> {
        let user_id = media.uploaded_by.as_uuid();
        let media_type_str = media.media_type.mime_type();
        let content_hash_str = media.content_hash.as_str();
        let processing_status_str = media.processing_status.to_string();

        // Convert SystemTime to chrono DateTime for database compatibility
        let uploaded_at: DateTime<Utc> = media.uploaded_at.into();
        let updated_at: DateTime<Utc> = media.updated_at.into();

        sqlx::query(
            r"
            INSERT INTO recipe_manager.media
            (user_id, media_type, media_path, file_size, content_hash, original_filename, processing_status, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
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
        .execute(&self.pool)
        .await
        .map_err(AppError::from)?;

        Ok(())
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
    use crate::domain::value_objects::ContentHash;

    fn _create_test_media() -> Media {
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

    // Note: Cannot test repository creation without async test setup and actual database
    // Integration tests requiring actual database connections should be in the integration test directory

    // Note: Integration tests requiring actual database connections should be in the integration test directory
    // These unit tests focus on repository creation and data mapping logic
}
