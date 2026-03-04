use std::time::Duration;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use chrono::Utc;
use sqlx::postgres::PgPoolOptions;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::config::Config;
use crate::error::AppError;
use crate::models::{Media, NewMedia};

// ---------------------------------------------------------------------------
// Connection
// ---------------------------------------------------------------------------

pub async fn connect(config: &Config) -> Result<PgPool, AppError> {
    PgPoolOptions::new()
        .max_connections(config.db_max_connections)
        .acquire_timeout(Duration::from_secs(5))
        .connect(&config.database_url)
        .await
        .map_err(|e| AppError::Internal(format!("failed to connect to database: {e}")))
}

// ---------------------------------------------------------------------------
// Media CRUD
// ---------------------------------------------------------------------------

pub async fn save_media(pool: &PgPool, media: &NewMedia) -> Result<i64, AppError> {
    let now = Utc::now();
    let row = sqlx::query(
        r"INSERT INTO recipe_manager.media
            (user_id, media_type, media_path, file_size, content_hash,
             original_filename, processing_status, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
         RETURNING media_id",
    )
    .bind(media.user_id)
    .bind(&media.media_type)
    .bind(&media.media_path)
    .bind(media.file_size)
    .bind(&media.content_hash)
    .bind(&media.original_filename)
    .bind(&media.processing_status)
    .bind(now)
    .bind(now)
    .fetch_one(pool)
    .await?;

    Ok(row.get("media_id"))
}

pub async fn find_media_by_id(pool: &PgPool, id: i64) -> Result<Option<Media>, AppError> {
    let media = sqlx::query_as::<_, Media>(
        r"SELECT media_id, user_id, media_type, media_path, file_size, content_hash,
                original_filename, processing_status, created_at, updated_at
         FROM recipe_manager.media
         WHERE media_id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    Ok(media)
}

pub async fn find_media_by_content_hash(
    pool: &PgPool,
    hash: &str,
) -> Result<Option<Media>, AppError> {
    let media = sqlx::query_as::<_, Media>(
        r"SELECT media_id, user_id, media_type, media_path, file_size, content_hash,
                original_filename, processing_status, created_at, updated_at
         FROM recipe_manager.media
         WHERE content_hash = $1",
    )
    .bind(hash)
    .fetch_optional(pool)
    .await?;

    Ok(media)
}

/// Returns `(items, next_cursor)`. The cursor is a base64-encoded `media_id`
/// for the next page, or `None` if there are no more results.
pub async fn find_media_by_user_paginated(
    pool: &PgPool,
    user_id: Uuid,
    cursor: Option<&str>,
    limit: i64,
    status: Option<&str>,
) -> Result<(Vec<Media>, Option<String>), AppError> {
    let cursor_id: Option<i64> = cursor.map(decode_cursor).transpose()?;

    let mut qb = sqlx::QueryBuilder::<sqlx::Postgres>::new(
        r"SELECT media_id, user_id, media_type, media_path, file_size, content_hash,
                original_filename, processing_status, created_at, updated_at
         FROM recipe_manager.media
         WHERE user_id = ",
    );
    qb.push_bind(user_id);

    if let Some(s) = status {
        qb.push(" AND processing_status = ");
        qb.push_bind(s.to_owned());
    }

    if let Some(id) = cursor_id {
        qb.push(" AND media_id > ");
        qb.push_bind(id);
    }

    qb.push(" ORDER BY media_id ASC LIMIT ");
    qb.push_bind(limit + 1);

    let mut rows: Vec<Media> = qb.build_query_as::<Media>().fetch_all(pool).await?;

    let limit_usize = usize::try_from(limit).unwrap_or(usize::MAX);
    let has_next = rows.len() > limit_usize;
    if has_next {
        rows.truncate(limit_usize);
    }

    let next_cursor = if has_next {
        rows.last().map(|m| encode_cursor(m.media_id))
    } else {
        None
    };

    Ok((rows, next_cursor))
}

/// Updates all mutable fields on the media record identified by `media.media_id`.
/// Sets `updated_at` to the current time.
pub async fn update_media(pool: &PgPool, media: &Media) -> Result<(), AppError> {
    sqlx::query(
        r"UPDATE recipe_manager.media
         SET media_type = $2, media_path = $3, file_size = $4, content_hash = $5,
             original_filename = $6, processing_status = $7, updated_at = $8
         WHERE media_id = $1",
    )
    .bind(media.media_id)
    .bind(&media.media_type)
    .bind(&media.media_path)
    .bind(media.file_size)
    .bind(&media.content_hash)
    .bind(&media.original_filename)
    .bind(&media.processing_status)
    .bind(Utc::now())
    .execute(pool)
    .await?;

    Ok(())
}

/// Returns `true` if a row was deleted, `false` if no row matched.
pub async fn delete_media(pool: &PgPool, id: i64) -> Result<bool, AppError> {
    let result = sqlx::query(r"DELETE FROM recipe_manager.media WHERE media_id = $1")
        .bind(id)
        .execute(pool)
        .await?;

    Ok(result.rows_affected() > 0)
}

pub async fn exists_by_content_hash(pool: &PgPool, hash: &str) -> Result<bool, AppError> {
    let row = sqlx::query(
        r"SELECT EXISTS(
             SELECT 1 FROM recipe_manager.media WHERE content_hash = $1
         ) AS exists",
    )
    .bind(hash)
    .fetch_one(pool)
    .await?;

    Ok(row.get::<bool, _>("exists"))
}

// ---------------------------------------------------------------------------
// Association queries
// ---------------------------------------------------------------------------

pub async fn find_media_ids_by_recipe(pool: &PgPool, recipe_id: i64) -> Result<Vec<i64>, AppError> {
    let rows = sqlx::query(
        r"SELECT media_id
         FROM recipe_manager.recipe_media
         WHERE recipe_id = $1
         ORDER BY media_id",
    )
    .bind(recipe_id)
    .fetch_all(pool)
    .await?;

    Ok(rows.iter().map(|r| r.get("media_id")).collect())
}

pub async fn find_media_ids_by_recipe_ingredient(
    pool: &PgPool,
    recipe_id: i64,
    ingredient_id: i64,
) -> Result<Vec<i64>, AppError> {
    let rows = sqlx::query(
        r"SELECT media_id
         FROM recipe_manager.ingredient_media
         WHERE recipe_id = $1 AND ingredient_id = $2
         ORDER BY media_id",
    )
    .bind(recipe_id)
    .bind(ingredient_id)
    .fetch_all(pool)
    .await?;

    Ok(rows.iter().map(|r| r.get("media_id")).collect())
}

pub async fn find_media_ids_by_recipe_step(
    pool: &PgPool,
    recipe_id: i64,
    step_id: i64,
) -> Result<Vec<i64>, AppError> {
    let rows = sqlx::query(
        r"SELECT media_id
         FROM recipe_manager.step_media
         WHERE recipe_id = $1 AND step_id = $2
         ORDER BY media_id",
    )
    .bind(recipe_id)
    .bind(step_id)
    .fetch_all(pool)
    .await?;

    Ok(rows.iter().map(|r| r.get("media_id")).collect())
}

// ---------------------------------------------------------------------------
// Infrastructure
// ---------------------------------------------------------------------------

pub async fn db_health_check(pool: &PgPool) -> Result<(), AppError> {
    sqlx::query("SELECT 1")
        .execute(pool)
        .await
        .map(|_| ())
        .map_err(|e| AppError::ServiceUnavailable(format!("database health check failed: {e}")))
}

// ---------------------------------------------------------------------------
// Cursor helpers
// ---------------------------------------------------------------------------

fn encode_cursor(media_id: i64) -> String {
    BASE64.encode(media_id.to_be_bytes())
}

fn decode_cursor(cursor: &str) -> Result<i64, AppError> {
    let bytes = BASE64
        .decode(cursor)
        .map_err(|_| AppError::BadRequest("invalid cursor encoding".into()))?;

    let arr: [u8; 8] = bytes
        .try_into()
        .map_err(|_| AppError::BadRequest("invalid cursor length".into()))?;

    Ok(i64::from_be_bytes(arr))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_roundtrip() {
        for id in [0_i64, 1, 42, 1_000_000, i64::MAX] {
            let encoded = encode_cursor(id);
            let decoded = decode_cursor(&encoded).unwrap();
            assert_eq!(decoded, id, "roundtrip failed for {id}");
        }
    }

    #[test]
    fn cursor_roundtrip_negative() {
        let encoded = encode_cursor(-1);
        let decoded = decode_cursor(&encoded).unwrap();
        assert_eq!(decoded, -1);
    }

    #[test]
    fn decode_cursor_rejects_invalid_base64() {
        let result = decode_cursor("not-valid-base64!!!");
        assert!(
            matches!(result, Err(AppError::BadRequest(_))),
            "expected BadRequest for invalid base64"
        );
    }

    #[test]
    fn decode_cursor_rejects_wrong_length() {
        let short = BASE64.encode([1_u8, 2, 3, 4]);
        let result = decode_cursor(&short);
        assert!(
            matches!(result, Err(AppError::BadRequest(_))),
            "expected BadRequest for wrong-length cursor"
        );
    }

    #[test]
    fn decode_cursor_rejects_empty() {
        let empty = BASE64.encode([]);
        let result = decode_cursor(&empty);
        assert!(
            matches!(result, Err(AppError::BadRequest(_))),
            "expected BadRequest for empty cursor"
        );
    }
}
