use axum::{
    body::Body,
    extract::{Multipart, Path, Query, State},
    http::{header, StatusCode},
    response::{Json, Response},
};
use std::sync::Arc;

use crate::{
    application::{
        dto::{ListMediaQuery, MediaDto, UploadMediaResponse},
        use_cases::{DownloadMediaUseCase, GetMediaUseCase, ListMediaUseCase, UploadMediaUseCase},
    },
    domain::entities::{MediaId, UserId},
    presentation::middleware::error::AppError,
};

use crate::infrastructure::{persistence::PostgreSqlMediaRepository, storage::FilesystemStorage};

/// Application state containing dependencies
#[derive(Clone)]
pub struct AppState {
    pub repository: Arc<PostgreSqlMediaRepository>,
    pub storage: Arc<FilesystemStorage>,
    pub max_file_size: u64,
}

impl AppState {
    pub fn new(
        repository: Arc<PostgreSqlMediaRepository>,
        storage: Arc<FilesystemStorage>,
        max_file_size: u64,
    ) -> Self {
        Self { repository, storage, max_file_size }
    }
}

/// Upload a new media file
///
/// # Errors
/// Returns appropriate HTTP status codes for various error conditions
pub async fn upload_media(
    State(app_state): State<AppState>,
    mut multipart: Multipart,
) -> Result<Json<UploadMediaResponse>, AppError> {
    tracing::info!("Processing media upload request");

    let mut file_data: Option<Vec<u8>> = None;
    let mut filename: Option<String> = None;
    let mut content_type_detected: Option<String> = None;

    // Process multipart form fields
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::BadRequest { message: format!("Invalid multipart data: {e}") })?
    {
        let field_name = field.name().unwrap_or("unknown").to_string();
        tracing::debug!("Processing multipart field: {}", field_name);

        match field_name.as_str() {
            "file" => {
                // Get file content type from field
                content_type_detected = field.content_type().map(std::string::ToString::to_string);

                // Get filename from field
                if let Some(field_filename) = field.file_name() {
                    filename = Some(field_filename.to_string());
                }

                // Read file data
                let data = field.bytes().await.map_err(|e| AppError::BadRequest {
                    message: format!("Failed to read file data: {e}"),
                })?;

                // Check size limit
                if data.len() as u64 > app_state.max_file_size {
                    return Err(AppError::BadRequest { message: "File too large".to_string() });
                }

                file_data = Some(data.to_vec());
                tracing::info!("Received file data: {} bytes", data.len());
            }
            "filename" => {
                // Alternative way to get filename if not in file field
                if filename.is_none() {
                    let data = field.bytes().await.map_err(|e| AppError::BadRequest {
                        message: format!("Failed to read filename field: {e}"),
                    })?;
                    filename = Some(String::from_utf8_lossy(&data).to_string());
                }
            }
            _ => {
                tracing::debug!("Ignoring unknown field: {}", field_name);
                // Skip unknown fields
            }
        }
    }

    // Validate required fields
    let file_data = file_data
        .ok_or_else(|| AppError::BadRequest { message: "No file data provided".to_string() })?;

    let filename = filename
        .ok_or_else(|| AppError::BadRequest { message: "No filename provided".to_string() })?;

    tracing::info!(
        "Upload request validated - file: {}, size: {} bytes",
        filename,
        file_data.len()
    );

    // Create upload use case and execute
    let upload_use_case = UploadMediaUseCase::new(
        app_state.repository.clone(),
        app_state.storage.clone(),
        app_state.max_file_size,
    );

    // For now, use a default user ID. In production, this would come from authentication
    let user_id = UserId::new();

    let file_cursor = std::io::Cursor::new(file_data);
    let response =
        upload_use_case.execute(file_cursor, filename, user_id, content_type_detected).await?;

    tracing::info!("Media upload completed successfully: {}", response.media_id);

    Ok(Json(response))
}

/// List media files
///
/// # Errors
/// Returns appropriate HTTP status codes for various error conditions
pub async fn list_media(
    State(app_state): State<AppState>,
    Query(query): Query<ListMediaQuery>,
) -> Result<Json<Vec<MediaDto>>, AppError> {
    tracing::info!("Processing media list request with query: {:?}", query);

    let list_use_case = ListMediaUseCase::new(app_state.repository.clone());

    // For now, use a default user ID. In production, this would come from authentication
    let user_id = UserId::new();

    let media_list = list_use_case.execute(query, user_id).await?;

    tracing::info!("Retrieved {} media files", media_list.len());

    Ok(Json(media_list))
}

/// Get media information by ID
///
/// # Errors
/// Returns appropriate HTTP status codes for various error conditions
pub async fn get_media(
    State(app_state): State<AppState>,
    Path(id): Path<MediaId>,
) -> Result<Json<MediaDto>, AppError> {
    tracing::info!("Processing get media request for ID: {}", id);

    let get_use_case = GetMediaUseCase::new(app_state.repository.clone());
    let media_dto = get_use_case.execute(id).await?;

    tracing::info!("Retrieved media: {}", media_dto.original_filename);

    Ok(Json(media_dto))
}

/// Download media file
///
/// # Errors
/// Returns appropriate HTTP status codes for various error conditions
pub async fn download_media(
    State(app_state): State<AppState>,
    Path(id): Path<MediaId>,
) -> Result<Response<Body>, AppError> {
    tracing::info!("Processing download request for media ID: {}", id);

    let download_use_case =
        DownloadMediaUseCase::new(app_state.repository.clone(), app_state.storage.clone());

    let download_response = download_use_case.execute(id).await?;

    tracing::info!(
        "Serving download: {} ({} bytes)",
        download_response.filename,
        download_response.content.len()
    );

    // Create HTTP response with appropriate headers
    let response = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, download_response.content_type)
        .header(header::CONTENT_LENGTH, download_response.content.len())
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", download_response.filename),
        )
        .header(header::CACHE_CONTROL, "private, max-age=3600") // Cache for 1 hour
        .body(Body::from(download_response.content))
        .map_err(|e| AppError::Internal { message: format!("Failed to build response: {e}") })?;

    Ok(response)
}

#[cfg(test)]
mod tests {
    // Test functions would need to be updated to use concrete types
    // For now, handlers are tested through use case unit tests

    // Note: Testing handlers with multipart data requires more complex setup
    // These tests would typically be integration tests with a full HTTP server
    // For now, we'll test the use cases directly in their respective test modules
}
