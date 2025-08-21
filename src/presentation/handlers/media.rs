use axum::{
    extract::{Path, Query},
    http::StatusCode,
    response::Json,
};
use serde_json::{json, Value};

use crate::application::dto::{ListMediaQuery, MediaDto, UploadMediaResponse};
use crate::domain::entities::MediaId;

/// Upload a new media file
///
/// # Errors
/// Returns a 501 Not Implemented error as this functionality is not yet implemented
pub async fn upload_media() -> Result<Json<UploadMediaResponse>, (StatusCode, Json<Value>)> {
    // TODO: Implement actual upload logic
    Err((
        StatusCode::NOT_IMPLEMENTED,
        Json(json!({
            "error": "Not Implemented",
            "message": "Media upload functionality is not yet implemented"
        })),
    ))
}

/// List media files
///
/// # Errors
/// Currently returns an empty list but may return errors in future implementations
pub async fn list_media(
    Query(_query): Query<ListMediaQuery>,
) -> Result<Json<Vec<MediaDto>>, (StatusCode, Json<Value>)> {
    // TODO: Implement actual listing logic
    Ok(Json(vec![]))
}

/// Get media information by ID
///
/// # Errors
/// Returns a 404 Not Found error as this functionality is not yet implemented
pub async fn get_media(
    Path(_id): Path<MediaId>,
) -> Result<Json<MediaDto>, (StatusCode, Json<Value>)> {
    // TODO: Implement actual get logic
    Err((
        StatusCode::NOT_FOUND,
        Json(json!({
            "error": "Not Found",
            "message": "Media not found"
        })),
    ))
}

/// Download media file
///
/// # Errors
/// Returns a 501 Not Implemented error as this functionality is not yet implemented
pub async fn download_media(
    Path(_id): Path<MediaId>,
) -> Result<Vec<u8>, (StatusCode, Json<Value>)> {
    // TODO: Implement actual download logic
    Err((
        StatusCode::NOT_IMPLEMENTED,
        Json(json!({
            "error": "Not Implemented",
            "message": "Media download functionality is not yet implemented"
        })),
    ))
}
