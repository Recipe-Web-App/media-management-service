use crate::domain::{entities::MediaId, value_objects::ProcessingStatus};
use serde::{Deserialize, Serialize};

/// Data Transfer Object for media information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaDto {
    pub id: MediaId,
    pub content_hash: String,
    pub original_filename: String,
    pub media_type: String, // MIME type string
    pub media_path: String,
    pub file_size: u64,
    pub processing_status: ProcessingStatus,
    pub uploaded_at: String, // ISO 8601 timestamp
    pub updated_at: String,  // ISO 8601 timestamp
}

/// Request DTO for uploading media (legacy direct upload)
#[derive(Debug, Clone, Deserialize)]
pub struct UploadMediaRequest {
    pub filename: String,
    // File content will be handled separately as a stream
}

/// Request DTO for initiating a presigned upload session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitiateUploadRequest {
    pub filename: String,
    pub content_type: String,
    pub file_size: u64,
}

/// Response DTO for upload initiation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitiateUploadResponse {
    pub media_id: MediaId,
    pub upload_url: String,
    pub upload_token: String,
    pub expires_at: String, // ISO 8601 timestamp
    pub status: ProcessingStatus,
}

/// Response DTO for upload status checking
#[derive(Debug, Clone, Serialize)]
pub struct UploadStatusResponse {
    pub media_id: MediaId,
    pub status: ProcessingStatus,
    pub progress: Option<u8>, // 0-100 percentage
    pub error_message: Option<String>,
    pub download_url: Option<String>,
    pub processing_time_ms: Option<u64>,
    pub uploaded_at: Option<String>,
    pub completed_at: Option<String>,
}

/// Response DTO for successful upload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadMediaResponse {
    pub media_id: MediaId,
    pub content_hash: String,
    pub processing_status: ProcessingStatus,
    pub upload_url: Option<String>, // For direct file access
}

/// Query parameters for paginated media listing
#[derive(Debug, Clone, Deserialize)]
pub struct PaginatedMediaQuery {
    /// Cursor for pagination (base64 encoded)
    pub cursor: Option<String>,
    /// Maximum number of items per page (default 50, max 100)
    pub limit: Option<u32>,
    /// Filter by processing status
    pub status: Option<ProcessingStatus>,
}

/// Pagination metadata for cursor-based pagination
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationInfo {
    /// Cursor for the next page (if available)
    pub next_cursor: Option<String>,
    /// Cursor for the previous page (if available)
    pub prev_cursor: Option<String>,
    /// Total number of items in current page
    pub page_size: u32,
    /// Whether there are more items after this page
    pub has_next: bool,
    /// Whether there are items before this page
    pub has_prev: bool,
}

/// Paginated response for media listing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedMediaResponse {
    /// List of media items for current page
    pub data: Vec<MediaDto>,
    /// Pagination metadata
    pub pagination: PaginationInfo,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_media_dto() -> MediaDto {
        MediaDto {
            id: MediaId::new(1),
            content_hash: "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890"
                .to_string(),
            original_filename: "test.jpg".to_string(),
            media_type: "image/jpeg".to_string(),
            media_path: "ab/cd/ef/abcdef123".to_string(),
            file_size: 1024,
            processing_status: ProcessingStatus::Complete,
            uploaded_at: "2023-01-01T00:00:00Z".to_string(),
            updated_at: "2023-01-01T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn test_media_dto_serialization() {
        let dto = create_test_media_dto();

        let json = serde_json::to_string(&dto).unwrap();
        let deserialized: MediaDto = serde_json::from_str(&json).unwrap();

        assert_eq!(dto.id, deserialized.id);
        assert_eq!(dto.content_hash, deserialized.content_hash);
        assert_eq!(dto.original_filename, deserialized.original_filename);
        assert_eq!(dto.media_type, deserialized.media_type);
        assert_eq!(dto.file_size, deserialized.file_size);
        assert_eq!(dto.processing_status, deserialized.processing_status);
    }

    #[test]
    fn test_media_dto_with_video() {
        let dto = MediaDto {
            id: MediaId::new(2),
            content_hash: "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
                .to_string(),
            original_filename: "test.mp4".to_string(),
            media_type: "video/mp4".to_string(),
            media_path: "12/34/56/1234567890".to_string(),
            file_size: 5_000_000,
            processing_status: ProcessingStatus::Processing,
            uploaded_at: "2023-01-01T00:00:00Z".to_string(),
            updated_at: "2023-01-01T01:00:00Z".to_string(),
        };

        let json = serde_json::to_string(&dto).unwrap();
        let deserialized: MediaDto = serde_json::from_str(&json).unwrap();

        assert_eq!(dto.media_type, deserialized.media_type);
        assert_eq!(dto.processing_status, deserialized.processing_status);
    }

    #[test]
    fn test_upload_media_request_deserialization() {
        let json = r#"{"filename": "test-upload.png"}"#;
        let request: UploadMediaRequest = serde_json::from_str(json).unwrap();

        assert_eq!(request.filename, "test-upload.png");
    }

    #[test]
    fn test_upload_media_response_serialization() {
        let response = UploadMediaResponse {
            media_id: MediaId::new(3),
            content_hash: "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890"
                .to_string(),
            processing_status: ProcessingStatus::Pending,
            upload_url: Some("https://example.com/media/abc123".to_string()),
        };

        let json = serde_json::to_string(&response).unwrap();
        let deserialized: UploadMediaResponse = serde_json::from_str(&json).unwrap();

        assert_eq!(response.media_id, deserialized.media_id);
        assert_eq!(response.content_hash, deserialized.content_hash);
        assert_eq!(response.processing_status, deserialized.processing_status);
        assert_eq!(response.upload_url, deserialized.upload_url);
    }

    #[test]
    fn test_upload_media_response_without_url() {
        let response = UploadMediaResponse {
            media_id: MediaId::new(4),
            content_hash: "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
                .to_string(),
            processing_status: ProcessingStatus::Failed,
            upload_url: None,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("null"));

        let deserialized: UploadMediaResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(response.upload_url, deserialized.upload_url);
        assert!(deserialized.upload_url.is_none());
    }

    #[test]
    fn test_dto_clone_and_debug() {
        let dto = create_test_media_dto();
        let cloned = dto.clone();

        assert_eq!(dto.id, cloned.id);
        assert_eq!(dto.content_hash, cloned.content_hash);

        let debug_str = format!("{dto:?}");
        assert!(debug_str.contains("MediaDto"));
        assert!(debug_str.contains("test.jpg"));
    }

    #[test]
    fn test_paginated_media_query_deserialization() {
        let json = r#"{"cursor": "eyJpZCI6MTIzfQ==", "limit": 25, "status": "Complete"}"#;
        let query: PaginatedMediaQuery = serde_json::from_str(json).unwrap();

        assert_eq!(query.cursor, Some("eyJpZCI6MTIzfQ==".to_string()));
        assert_eq!(query.limit, Some(25));
        assert_eq!(query.status, Some(ProcessingStatus::Complete));
    }

    #[test]
    fn test_paginated_media_query_partial() {
        let json = r#"{"limit": 10}"#;
        let query: PaginatedMediaQuery = serde_json::from_str(json).unwrap();

        assert_eq!(query.cursor, None);
        assert_eq!(query.limit, Some(10));
        assert_eq!(query.status, None);
    }

    #[test]
    fn test_pagination_info_serialization() {
        let pagination = PaginationInfo {
            next_cursor: Some("next123".to_string()),
            prev_cursor: Some("prev123".to_string()),
            page_size: 25,
            has_next: true,
            has_prev: true,
        };

        let json = serde_json::to_string(&pagination).unwrap();
        let deserialized: PaginationInfo = serde_json::from_str(&json).unwrap();

        assert_eq!(pagination.next_cursor, deserialized.next_cursor);
        assert_eq!(pagination.prev_cursor, deserialized.prev_cursor);
        assert_eq!(pagination.page_size, deserialized.page_size);
        assert_eq!(pagination.has_next, deserialized.has_next);
        assert_eq!(pagination.has_prev, deserialized.has_prev);
    }

    #[test]
    fn test_paginated_media_response_serialization() {
        let dto = create_test_media_dto();
        let pagination = PaginationInfo {
            next_cursor: Some("next456".to_string()),
            prev_cursor: None,
            page_size: 1,
            has_next: true,
            has_prev: false,
        };
        let response = PaginatedMediaResponse { data: vec![dto], pagination };

        let json = serde_json::to_string(&response).unwrap();
        let deserialized: PaginatedMediaResponse = serde_json::from_str(&json).unwrap();

        assert_eq!(response.data.len(), deserialized.data.len());
        assert_eq!(response.pagination.page_size, deserialized.pagination.page_size);
        assert_eq!(response.pagination.has_next, deserialized.pagination.has_next);
        assert_eq!(response.pagination.has_prev, deserialized.pagination.has_prev);
    }

    #[test]
    fn test_pagination_info_first_page() {
        let pagination = PaginationInfo {
            next_cursor: Some("next".to_string()),
            prev_cursor: None,
            page_size: 50,
            has_next: true,
            has_prev: false,
        };

        assert!(pagination.has_next);
        assert!(!pagination.has_prev);
        assert!(pagination.prev_cursor.is_none());
        assert!(pagination.next_cursor.is_some());
    }

    #[test]
    fn test_pagination_info_last_page() {
        let pagination = PaginationInfo {
            next_cursor: None,
            prev_cursor: Some("prev".to_string()),
            page_size: 30,
            has_next: false,
            has_prev: true,
        };

        assert!(!pagination.has_next);
        assert!(pagination.has_prev);
        assert!(pagination.next_cursor.is_none());
        assert!(pagination.prev_cursor.is_some());
    }
}
