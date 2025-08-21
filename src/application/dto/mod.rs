use crate::domain::{
    entities::MediaId,
    value_objects::{MediaType, ProcessingStatus},
};
use serde::{Deserialize, Serialize};

/// Data Transfer Object for media information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaDto {
    pub id: MediaId,
    pub content_hash: String,
    pub original_filename: String,
    pub media_type: MediaType,
    pub file_size: u64,
    pub processing_status: ProcessingStatus,
    pub uploaded_at: String, // ISO 8601 timestamp
    pub updated_at: String,  // ISO 8601 timestamp
}

/// Request DTO for uploading media
#[derive(Debug, Clone, Deserialize)]
pub struct UploadMediaRequest {
    pub filename: String,
    // File content will be handled separately as a stream
}

/// Response DTO for successful upload
#[derive(Debug, Clone, Serialize)]
pub struct UploadMediaResponse {
    pub media_id: MediaId,
    pub content_hash: String,
    pub processing_status: ProcessingStatus,
    pub upload_url: Option<String>, // For direct file access
}

/// Query parameters for listing media
#[derive(Debug, Clone, Deserialize)]
pub struct ListMediaQuery {
    pub limit: Option<u32>,
    pub offset: Option<u32>,
    pub status: Option<ProcessingStatus>,
}
