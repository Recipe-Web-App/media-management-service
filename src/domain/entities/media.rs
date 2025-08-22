use serde::{Deserialize, Serialize};
use std::time::SystemTime;

use crate::domain::value_objects::{ContentHash, MediaType, ProcessingStatus};

/// Core media entity representing a file in the system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Media {
    pub id: MediaId,
    pub content_hash: ContentHash,
    pub original_filename: String,
    pub media_type: MediaType,
    pub file_size: u64,
    pub processing_status: ProcessingStatus,
    pub uploaded_by: crate::domain::entities::UserId,
    pub uploaded_at: SystemTime,
    pub updated_at: SystemTime,
}

/// Unique identifier for media files
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MediaId(uuid::Uuid);

impl MediaId {
    #[must_use]
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4())
    }

    #[must_use]
    pub fn from_uuid(uuid: uuid::Uuid) -> Self {
        Self(uuid)
    }

    #[must_use]
    pub fn as_uuid(&self) -> uuid::Uuid {
        self.0
    }
}

impl Default for MediaId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for MediaId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Media {
    /// Create a new media entity
    #[must_use]
    pub fn new(
        content_hash: ContentHash,
        original_filename: String,
        media_type: MediaType,
        file_size: u64,
        uploaded_by: crate::domain::entities::UserId,
    ) -> Self {
        let now = SystemTime::now();
        Self {
            id: MediaId::new(),
            content_hash,
            original_filename,
            media_type,
            file_size,
            processing_status: ProcessingStatus::Pending,
            uploaded_by,
            uploaded_at: now,
            updated_at: now,
        }
    }

    /// Update the processing status
    pub fn set_processing_status(&mut self, status: ProcessingStatus) {
        self.processing_status = status;
        self.updated_at = SystemTime::now();
    }

    /// Check if the media file is ready for serving
    #[must_use]
    pub fn is_ready(&self) -> bool {
        matches!(self.processing_status, ProcessingStatus::Complete)
    }

    /// Check if processing failed
    #[must_use]
    pub fn has_failed(&self) -> bool {
        matches!(self.processing_status, ProcessingStatus::Failed(_))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::value_objects::{ImageFormat, VideoFormat};

    fn create_test_content_hash() -> ContentHash {
        ContentHash::new("abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890")
            .unwrap()
    }

    fn create_test_user_id() -> crate::domain::entities::UserId {
        crate::domain::entities::UserId::new()
    }

    #[test]
    fn test_media_creation() {
        let content_hash = create_test_content_hash();
        let filename = "test.jpg".to_string();
        let media_type = MediaType::Image { format: ImageFormat::Jpeg, width: 1920, height: 1080 };
        let file_size = 1024;
        let user_id = create_test_user_id();

        let media = Media::new(
            content_hash.clone(),
            filename.clone(),
            media_type.clone(),
            file_size,
            user_id,
        );

        assert_eq!(media.content_hash, content_hash);
        assert_eq!(media.original_filename, filename);
        assert_eq!(media.media_type, media_type);
        assert_eq!(media.file_size, file_size);
        assert_eq!(media.uploaded_by, user_id);
        assert_eq!(media.processing_status, ProcessingStatus::Pending);
        assert!(media.uploaded_at <= media.updated_at);
    }

    #[test]
    fn test_media_id_operations() {
        let id1 = MediaId::new();
        let id2 = MediaId::new();

        assert_ne!(id1, id2);

        let uuid = uuid::Uuid::new_v4();
        let id_from_uuid = MediaId::from_uuid(uuid);
        assert_eq!(id_from_uuid.as_uuid(), uuid);

        let id_string = id1.to_string();
        assert_eq!(id_string, id1.as_uuid().to_string());
    }

    #[test]
    fn test_processing_status_update() {
        let content_hash = create_test_content_hash();
        let media_type = MediaType::Video {
            format: VideoFormat::Mp4,
            width: 1280,
            height: 720,
            duration_seconds: Some(120),
        };
        let user_id = create_test_user_id();

        let mut media = Media::new(content_hash, "test.mp4".to_string(), media_type, 2048, user_id);

        assert!(media.processing_status.is_pending());
        assert!(!media.is_ready());
        assert!(!media.has_failed());

        media.set_processing_status(ProcessingStatus::Processing);
        assert!(media.processing_status.is_processing());
        assert!(!media.is_ready());
        assert!(!media.has_failed());

        media.set_processing_status(ProcessingStatus::Complete);
        assert!(media.processing_status.is_complete());
        assert!(media.is_ready());
        assert!(!media.has_failed());
    }

    #[test]
    fn test_processing_failure() {
        let content_hash = create_test_content_hash();
        let media_type = MediaType::Image { format: ImageFormat::Png, width: 800, height: 600 };
        let user_id = create_test_user_id();

        let mut media = Media::new(content_hash, "test.png".to_string(), media_type, 512, user_id);

        let error_message = "Corrupted file header";
        media.set_processing_status(ProcessingStatus::Failed(error_message.to_string()));

        assert!(media.processing_status.is_failed());
        assert!(!media.is_ready());
        assert!(media.has_failed());
        assert_eq!(media.processing_status.error_message(), Some(error_message));
    }

    #[test]
    fn test_updated_at_changes_on_status_update() {
        let content_hash = create_test_content_hash();
        let media_type = MediaType::Image { format: ImageFormat::WebP, width: 1024, height: 768 };
        let user_id = create_test_user_id();

        let mut media =
            Media::new(content_hash, "test.webp".to_string(), media_type, 1024, user_id);
        let initial_updated_at = media.updated_at;

        std::thread::sleep(std::time::Duration::from_millis(1));
        media.set_processing_status(ProcessingStatus::Complete);

        assert!(media.updated_at > initial_updated_at);
    }
}
