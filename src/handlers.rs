use axum::Json;
use axum::body::Body;
use axum::extract::{Multipart, Path, Query, State};
use axum::http::header;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio_util::io::ReaderStream;
use uuid::Uuid;

use crate::db;
use crate::error::AppError;
use crate::models::{
    ContentHash, ListMediaQuery, Media, MediaDto, NewMedia, PaginatedMediaResponse, PaginationInfo,
    UploadStatusResponse,
};
use crate::presigned;
use crate::state::AppState;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Temporary user-ID extraction from the `x-user-id` header.
/// Replaced by auth middleware in Phase 5.
fn extract_user_id(headers: &HeaderMap) -> Result<Uuid, AppError> {
    let value = headers
        .get("x-user-id")
        .ok_or_else(|| AppError::Unauthorized("missing x-user-id header".into()))?
        .to_str()
        .map_err(|_| AppError::BadRequest("x-user-id header is not valid UTF-8".into()))?;

    Uuid::parse_str(value).map_err(|_| AppError::BadRequest("x-user-id is not a valid UUID".into()))
}

fn media_to_dto(media: &Media, signing_secret: &str, ttl_secs: u64) -> MediaDto {
    let download_url = presigned::generate_download_url(
        media.media_id,
        &media.processing_status,
        signing_secret,
        ttl_secs,
    );
    MediaDto {
        id: media.media_id,
        content_hash: media.content_hash.clone(),
        original_filename: media.original_filename.clone(),
        media_type: media.media_type.clone(),
        file_size: media.file_size,
        processing_status: media.processing_status.clone(),
        uploaded_by: media.user_id.to_string(),
        uploaded_at: media.created_at,
        updated_at: media.updated_at,
        download_url,
    }
}

#[derive(Debug, Serialize)]
pub struct MediaIdsResponse {
    pub media_ids: Vec<i64>,
}

#[derive(Debug, Deserialize)]
pub struct DownloadQuery {
    pub signature: Option<String>,
    pub expires: Option<u64>,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

pub async fn upload_media(
    State(state): State<AppState>,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, AppError> {
    let user_id = extract_user_id(&headers)?;

    let field = multipart
        .next_field()
        .await
        .map_err(|e| AppError::BadRequest(format!("multipart error: {e}")))?
        .ok_or_else(|| AppError::BadRequest("missing file field".into()))?;

    let original_filename = field.file_name().unwrap_or("unnamed").to_string();
    let media_type = field
        .content_type()
        .unwrap_or("application/octet-stream")
        .to_string();

    let data = field
        .bytes()
        .await
        .map_err(|e| AppError::BadRequest(format!("failed to read file: {e}")))?;

    if data.len() as u64 > state.config.max_upload_size {
        return Err(AppError::PayloadTooLarge);
    }

    let file_size = i64::try_from(data.len()).map_err(|_| AppError::PayloadTooLarge)?;

    let hash_bytes = Sha256::digest(&data);
    let hash_hex = hex::encode(hash_bytes);
    let content_hash = ContentHash::new(&hash_hex)?;

    // Dedup: return existing record if same content already stored
    if let Some(existing) =
        db::find_media_by_content_hash(&state.db_pool, content_hash.as_str()).await?
    {
        let dto = media_to_dto(
            &existing,
            &state.config.signing_secret,
            state.config.download_url_ttl_secs,
        );
        return Ok((StatusCode::OK, Json(dto)));
    }

    let media_path = state.storage.store(&content_hash, &data).await?;

    let new_media = NewMedia {
        user_id,
        content_hash: hash_hex,
        original_filename,
        media_type,
        media_path,
        file_size,
        processing_status: "complete".to_string(),
    };
    let media_id = db::save_media(&state.db_pool, &new_media).await?;

    let media = db::find_media_by_id(&state.db_pool, media_id)
        .await?
        .ok_or_else(|| AppError::Internal("media record not found after insert".into()))?;

    let dto = media_to_dto(
        &media,
        &state.config.signing_secret,
        state.config.download_url_ttl_secs,
    );
    Ok((StatusCode::CREATED, Json(dto)))
}

pub async fn get_media(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(media_id): Path<i64>,
) -> Result<Json<MediaDto>, AppError> {
    let _user_id = extract_user_id(&headers)?;

    let media = db::find_media_by_id(&state.db_pool, media_id)
        .await?
        .ok_or(AppError::NotFound("media"))?;

    let dto = media_to_dto(
        &media,
        &state.config.signing_secret,
        state.config.download_url_ttl_secs,
    );
    Ok(Json(dto))
}

pub async fn list_media(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<ListMediaQuery>,
) -> Result<Json<PaginatedMediaResponse>, AppError> {
    let user_id = extract_user_id(&headers)?;

    let scope_user_id = query
        .uploaded_by
        .as_deref()
        .map(Uuid::parse_str)
        .transpose()
        .map_err(|_| AppError::BadRequest("uploaded_by is not a valid UUID".into()))?
        .unwrap_or(user_id);

    let limit = query.limit.unwrap_or(50).clamp(1, 100);

    let (media_rows, next_cursor) = db::find_media_by_user_paginated(
        &state.db_pool,
        scope_user_id,
        query.cursor.as_deref(),
        limit,
        query.status.as_deref(),
    )
    .await?;

    let page_size = media_rows.len();
    let data: Vec<MediaDto> = media_rows
        .iter()
        .map(|m| {
            media_to_dto(
                m,
                &state.config.signing_secret,
                state.config.download_url_ttl_secs,
            )
        })
        .collect();

    let response = PaginatedMediaResponse {
        data,
        pagination: PaginationInfo {
            next_cursor: next_cursor.clone(),
            prev_cursor: None,
            page_size,
            has_next: next_cursor.is_some(),
            has_prev: query.cursor.is_some(),
        },
    };

    Ok(Json(response))
}

pub async fn delete_media(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(media_id): Path<i64>,
) -> Result<StatusCode, AppError> {
    let _user_id = extract_user_id(&headers)?;

    let media = db::find_media_by_id(&state.db_pool, media_id)
        .await?
        .ok_or(AppError::NotFound("media"))?;

    db::delete_media(&state.db_pool, media_id).await?;

    // Only delete from storage if no other records reference this hash
    let content_hash = ContentHash::new(&media.content_hash)?;
    if !db::exists_by_content_hash(&state.db_pool, content_hash.as_str()).await? {
        let _ = state.storage.delete(&content_hash).await;
    }

    Ok(StatusCode::NO_CONTENT)
}

pub async fn download_media(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(media_id): Path<i64>,
    Query(query): Query<DownloadQuery>,
) -> Result<Response, AppError> {
    let is_authed = extract_user_id(&headers).is_ok();

    if !is_authed {
        let signature = query.signature.as_deref().ok_or_else(|| {
            AppError::Unauthorized(
                "missing authentication: provide Bearer token or signed URL".into(),
            )
        })?;
        let expires = query
            .expires
            .ok_or_else(|| AppError::Unauthorized("missing expires parameter".into()))?;

        presigned::verify_download_url(media_id, signature, expires, &state.config.signing_secret)?;
    }

    let media = db::find_media_by_id(&state.db_pool, media_id)
        .await?
        .ok_or(AppError::NotFound("media"))?;

    let content_hash = ContentHash::new(&media.content_hash)?;
    let file = state.storage.retrieve(&content_hash).await?;
    let stream = ReaderStream::new(file);
    let body = Body::from_stream(stream);

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, &media.media_type)
        .header(
            header::CONTENT_DISPOSITION,
            format!("inline; filename=\"{}\"", media.original_filename),
        )
        .header(header::CACHE_CONTROL, "public, max-age=31536000, immutable")
        .header(header::ETAG, format!("\"{}\"", media.content_hash))
        .header(header::CONTENT_LENGTH, media.file_size.to_string())
        .body(body)
        .map_err(|e| AppError::Internal(format!("failed to build response: {e}")))
}

pub async fn get_upload_status(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(media_id): Path<i64>,
) -> Result<Json<UploadStatusResponse>, AppError> {
    let _user_id = extract_user_id(&headers)?;

    let media = db::find_media_by_id(&state.db_pool, media_id)
        .await?
        .ok_or(AppError::NotFound("media"))?;

    let download_url = presigned::generate_download_url(
        media.media_id,
        &media.processing_status,
        &state.config.signing_secret,
        state.config.download_url_ttl_secs,
    );

    let completed_at = if media.processing_status == "complete" {
        Some(media.updated_at)
    } else {
        None
    };

    let response = UploadStatusResponse {
        media_id: media.media_id,
        status: media.processing_status,
        error_message: None,
        download_url,
        uploaded_at: Some(media.created_at),
        completed_at,
    };

    Ok(Json(response))
}

pub async fn get_media_by_recipe(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(recipe_id): Path<i64>,
) -> Result<Json<MediaIdsResponse>, AppError> {
    let _user_id = extract_user_id(&headers)?;
    let ids = db::find_media_ids_by_recipe(&state.db_pool, recipe_id).await?;
    Ok(Json(MediaIdsResponse { media_ids: ids }))
}

pub async fn get_media_by_ingredient(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((recipe_id, ingredient_id)): Path<(i64, i64)>,
) -> Result<Json<MediaIdsResponse>, AppError> {
    let _user_id = extract_user_id(&headers)?;
    let ids =
        db::find_media_ids_by_recipe_ingredient(&state.db_pool, recipe_id, ingredient_id).await?;
    Ok(Json(MediaIdsResponse { media_ids: ids }))
}

pub async fn get_media_by_step(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((recipe_id, step_id)): Path<(i64, i64)>,
) -> Result<Json<MediaIdsResponse>, AppError> {
    let _user_id = extract_user_id(&headers)?;
    let ids = db::find_media_ids_by_recipe_step(&state.db_pool, recipe_id, step_id).await?;
    Ok(Json(MediaIdsResponse { media_ids: ids }))
}
