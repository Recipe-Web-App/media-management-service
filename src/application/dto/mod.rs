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

/// Request DTO for uploading media
#[derive(Debug, Clone, Deserialize)]
pub struct UploadMediaRequest {
    pub filename: String,
    // File content will be handled separately as a stream
}

/// Response DTO for successful upload
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    fn test_list_media_query_deserialization() {
        let json = r#"{"limit": 10, "offset": 20, "status": "Complete"}"#;
        let query: ListMediaQuery = serde_json::from_str(json).unwrap();

        assert_eq!(query.limit, Some(10));
        assert_eq!(query.offset, Some(20));
        assert_eq!(query.status, Some(ProcessingStatus::Complete));
    }

    #[test]
    fn test_list_media_query_partial() {
        let json = r#"{"limit": 5}"#;
        let query: ListMediaQuery = serde_json::from_str(json).unwrap();

        assert_eq!(query.limit, Some(5));
        assert_eq!(query.offset, None);
        assert_eq!(query.status, None);
    }

    #[test]
    fn test_list_media_query_empty() {
        let json = r"{}";
        let query: ListMediaQuery = serde_json::from_str(json).unwrap();

        assert_eq!(query.limit, None);
        assert_eq!(query.offset, None);
        assert_eq!(query.status, None);
    }

    #[test]
    fn test_list_media_query_with_failed_status() {
        let json = r#"{"status": "Failed"}"#;
        let query: ListMediaQuery = serde_json::from_str(json).unwrap();

        match query.status {
            Some(ProcessingStatus::Failed) => {}
            _ => panic!("Expected Failed status"),
        }
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
}
