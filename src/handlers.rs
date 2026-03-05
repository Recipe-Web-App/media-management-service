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

use crate::auth::{self, AuthUser};
use crate::db;
use crate::error::AppError;
use crate::models::{
    ContentHash, InitiateUploadRequest, InitiateUploadResponse, ListMediaQuery, Media, MediaDto,
    NewMedia, PaginatedMediaResponse, PaginationInfo, UploadStatusResponse,
};
use crate::presigned;
use crate::state::AppState;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

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

#[derive(Debug, Deserialize)]
pub struct UploadQuery {
    pub signature: String,
    pub expires: u64,
    pub size: i64,
    #[serde(rename = "type")]
    pub content_type: String,
}

// ---------------------------------------------------------------------------
// Handlers (behind auth middleware)
// ---------------------------------------------------------------------------

#[tracing::instrument(
    name = "upload_media",
    skip_all,
    fields(
        operation = "upload_media",
        user_id = %auth_user.user_id,
        media_id = tracing::field::Empty,
        original_filename = tracing::field::Empty,
        content_hash = tracing::field::Empty,
    )
)]
pub async fn upload_media(
    State(state): State<AppState>,
    auth_user: AuthUser,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, AppError> {
    let user_id = auth_user.user_id;

    let field = multipart
        .next_field()
        .await
        .map_err(|e| AppError::BadRequest(format!("multipart error: {e}")))?
        .ok_or_else(|| AppError::BadRequest("missing file field".into()))?;

    let original_filename = field.file_name().unwrap_or("unnamed").to_string();
    tracing::Span::current().record("original_filename", &original_filename);
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
    tracing::Span::current().record("content_hash", content_hash.as_str());

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
    tracing::Span::current().record("media_id", media_id);

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

#[tracing::instrument(name = "get_media", skip_all, fields(operation = "get_media", media_id = %media_id))]
pub async fn get_media(
    State(state): State<AppState>,
    _auth_user: AuthUser,
    Path(media_id): Path<i64>,
) -> Result<Json<MediaDto>, AppError> {
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

#[tracing::instrument(name = "list_media", skip_all, fields(operation = "list_media", user_id = %auth_user.user_id))]
pub async fn list_media(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Query(query): Query<ListMediaQuery>,
) -> Result<Json<PaginatedMediaResponse>, AppError> {
    let user_id = auth_user.user_id;

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

#[tracing::instrument(name = "delete_media", skip_all, fields(operation = "delete_media", media_id = %media_id))]
pub async fn delete_media(
    State(state): State<AppState>,
    _auth_user: AuthUser,
    Path(media_id): Path<i64>,
) -> Result<StatusCode, AppError> {
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

#[tracing::instrument(name = "download_media", skip_all, fields(operation = "download_media", media_id = %media_id))]
pub async fn download_media(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(media_id): Path<i64>,
    Query(query): Query<DownloadQuery>,
) -> Result<Response, AppError> {
    // Dual auth: try bearer token first, fall back to signed URL.
    let is_authed = auth::authenticate(&state.auth_mode, &headers).await.is_ok();

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

#[tracing::instrument(name = "get_upload_status", skip_all, fields(operation = "get_upload_status", media_id = %media_id))]
pub async fn get_upload_status(
    State(state): State<AppState>,
    _auth_user: AuthUser,
    Path(media_id): Path<i64>,
) -> Result<Json<UploadStatusResponse>, AppError> {
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

// ---------------------------------------------------------------------------
// Presigned upload handlers
// ---------------------------------------------------------------------------

#[tracing::instrument(
    name = "initiate_upload",
    skip_all,
    fields(operation = "initiate_upload", user_id = %auth_user.user_id, media_id = tracing::field::Empty)
)]
pub async fn initiate_upload(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(req): Json<InitiateUploadRequest>,
) -> Result<impl IntoResponse, AppError> {
    let user_id = auth_user.user_id;

    if req.filename.trim().is_empty() {
        return Err(AppError::BadRequest("filename must not be empty".into()));
    }
    if req.content_type.trim().is_empty() {
        return Err(AppError::BadRequest(
            "content_type must not be empty".into(),
        ));
    }
    if req.file_size <= 0 {
        return Err(AppError::BadRequest("file_size must be positive".into()));
    }
    #[allow(clippy::cast_sign_loss)]
    if req.file_size as u64 > state.config.max_upload_size {
        return Err(AppError::PayloadTooLarge);
    }

    // Use a random placeholder hash to avoid UNIQUE constraint collisions
    // across concurrent pending uploads.
    let placeholder_hash = hex::encode(rand::random::<[u8; 32]>());

    let new_media = NewMedia {
        user_id,
        content_hash: placeholder_hash,
        original_filename: req.filename,
        media_type: req.content_type.clone(),
        media_path: String::new(),
        file_size: req.file_size,
        processing_status: "pending".to_string(),
    };
    let media_id = db::save_media(&state.db_pool, &new_media).await?;
    tracing::Span::current().record("media_id", media_id);

    let token = presigned::generate_upload_token(media_id);
    let (upload_url, expires) = presigned::sign_upload_url(
        &token,
        req.file_size,
        &req.content_type,
        &state.config.signing_secret,
        state.config.upload_url_ttl_secs,
    );

    #[allow(clippy::cast_possible_wrap)]
    let expires_at =
        chrono::DateTime::from_timestamp(expires as i64, 0).unwrap_or_else(chrono::Utc::now);

    let response = InitiateUploadResponse {
        media_id,
        upload_url,
        upload_token: token,
        expires_at,
    };

    Ok((StatusCode::OK, Json(response)))
}

#[tracing::instrument(
    name = "upload_file",
    skip_all,
    fields(operation = "upload_file", token = %token, media_id = tracing::field::Empty)
)]
pub async fn upload_file(
    State(state): State<AppState>,
    Path(token): Path<String>,
    Query(query): Query<UploadQuery>,
    body: axum::body::Bytes,
) -> Result<impl IntoResponse, AppError> {
    // URL-decode the content_type (e.g. "image%2Fjpeg" -> "image/jpeg")
    let content_type = query.content_type.replace("%2F", "/");

    presigned::verify_upload_signature(
        &token,
        &query.signature,
        query.expires,
        query.size,
        &content_type,
        &state.config.signing_secret,
    )?;

    let media_id = presigned::decode_upload_token(&token)?;
    tracing::Span::current().record("media_id", media_id);

    let mut media = db::find_media_by_id(&state.db_pool, media_id)
        .await?
        .ok_or(AppError::NotFound("media"))?;

    if media.processing_status != "pending" {
        return Err(AppError::Conflict(format!(
            "media {media_id} is not in pending status (current: {})",
            media.processing_status
        )));
    }

    let body_size = i64::try_from(body.len()).map_err(|_| AppError::PayloadTooLarge)?;
    if body_size != query.size {
        return Err(AppError::BadRequest(format!(
            "file size mismatch: expected {} bytes, got {body_size} bytes",
            query.size
        )));
    }

    let hash_bytes = Sha256::digest(&body);
    let hash_hex = hex::encode(hash_bytes);
    let content_hash = ContentHash::new(&hash_hex)?;

    // Dedup: if this content already exists, delete the pending record
    // and return the existing one (avoids UNIQUE constraint violation).
    if let Some(existing) =
        db::find_media_by_content_hash(&state.db_pool, content_hash.as_str()).await?
    {
        db::delete_media(&state.db_pool, media_id).await?;
        let dto = media_to_dto(
            &existing,
            &state.config.signing_secret,
            state.config.download_url_ttl_secs,
        );
        return Ok((StatusCode::OK, Json(dto)));
    }

    let media_path = state.storage.store(&content_hash, &body).await?;

    media.content_hash = hash_hex;
    media.media_path = media_path;
    media.file_size = body_size;
    media.processing_status = "complete".to_string();
    db::update_media(&state.db_pool, &media).await?;

    // Re-fetch for updated timestamps
    let updated = db::find_media_by_id(&state.db_pool, media_id)
        .await?
        .ok_or_else(|| AppError::Internal("media not found after update".into()))?;

    let dto = media_to_dto(
        &updated,
        &state.config.signing_secret,
        state.config.download_url_ttl_secs,
    );
    Ok((StatusCode::OK, Json(dto)))
}

// ---------------------------------------------------------------------------
// Association handlers (behind auth middleware)
// ---------------------------------------------------------------------------

#[tracing::instrument(name = "get_media_by_recipe", skip_all, fields(operation = "get_media_by_recipe", recipe_id = %recipe_id))]
pub async fn get_media_by_recipe(
    State(state): State<AppState>,
    _auth_user: AuthUser,
    Path(recipe_id): Path<i64>,
) -> Result<Json<MediaIdsResponse>, AppError> {
    let ids = db::find_media_ids_by_recipe(&state.db_pool, recipe_id).await?;
    Ok(Json(MediaIdsResponse { media_ids: ids }))
}

#[tracing::instrument(name = "get_media_by_ingredient", skip_all, fields(operation = "get_media_by_ingredient", recipe_id = %recipe_id, ingredient_id = %ingredient_id))]
pub async fn get_media_by_ingredient(
    State(state): State<AppState>,
    _auth_user: AuthUser,
    Path((recipe_id, ingredient_id)): Path<(i64, i64)>,
) -> Result<Json<MediaIdsResponse>, AppError> {
    let ids =
        db::find_media_ids_by_recipe_ingredient(&state.db_pool, recipe_id, ingredient_id).await?;
    Ok(Json(MediaIdsResponse { media_ids: ids }))
}

#[tracing::instrument(name = "get_media_by_step", skip_all, fields(operation = "get_media_by_step", recipe_id = %recipe_id, step_id = %step_id))]
pub async fn get_media_by_step(
    State(state): State<AppState>,
    _auth_user: AuthUser,
    Path((recipe_id, step_id)): Path<(i64, i64)>,
) -> Result<Json<MediaIdsResponse>, AppError> {
    let ids = db::find_media_ids_by_recipe_step(&state.db_pool, recipe_id, step_id).await?;
    Ok(Json(MediaIdsResponse { media_ids: ids }))
}
