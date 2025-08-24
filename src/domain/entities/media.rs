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
    pub media_path: String,
    pub file_size: u64,
    pub processing_status: ProcessingStatus,
    pub uploaded_by: crate::domain::entities::UserId,
    pub uploaded_at: SystemTime,
    pub updated_at: SystemTime,
}

/// Unique identifier for media files (database BIGSERIAL)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MediaId(i64);

impl MediaId {
    #[must_use]
    pub fn new(id: i64) -> Self {
        Self(id)
    }

    #[must_use]
    pub fn as_i64(&self) -> i64 {
        self.0
    }
}

impl std::fmt::Display for MediaId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<i64> for MediaId {
    fn from(id: i64) -> Self {
        Self(id)
    }
}

impl Media {
    /// Create a new media entity (without database ID - will be assigned on save)
    #[must_use]
    pub fn new(
        content_hash: ContentHash,
        original_filename: String,
        media_type: MediaType,
        media_path: String,
        file_size: u64,
        uploaded_by: crate::domain::entities::UserId,
    ) -> Self {
        let now = SystemTime::now();
        Self {
            id: MediaId::new(0), // Database will assign actual ID
            content_hash,
            original_filename,
            media_type,
            media_path,
            file_size,
            processing_status: ProcessingStatus::Pending,
            uploaded_by,
            uploaded_at: now,
            updated_at: now,
        }
    }

    /// Create a media entity with existing database ID
    #[must_use]
    pub fn with_id(
        id: MediaId,
        content_hash: ContentHash,
        original_filename: String,
        media_type: MediaType,
        media_path: String,
        file_size: u64,
        processing_status: ProcessingStatus,
    ) -> MediaBuilder {
        MediaBuilder {
            id,
            content_hash,
            original_filename,
            media_type,
            media_path,
            file_size,
            processing_status,
            uploaded_by: None,
            uploaded_at: None,
            updated_at: None,
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
        matches!(self.processing_status, ProcessingStatus::Failed)
    }
}

/// Builder for creating Media entities with many fields
pub struct MediaBuilder {
    id: MediaId,
    content_hash: ContentHash,
    original_filename: String,
    media_type: MediaType,
    media_path: String,
    file_size: u64,
    processing_status: ProcessingStatus,
    uploaded_by: Option<crate::domain::entities::UserId>,
    uploaded_at: Option<SystemTime>,
    updated_at: Option<SystemTime>,
}

impl MediaBuilder {
    /// Set the user who uploaded the media
    #[must_use]
    pub fn uploaded_by(mut self, user_id: crate::domain::entities::UserId) -> Self {
        self.uploaded_by = Some(user_id);
        self
    }

    /// Set the upload timestamp
    #[must_use]
    pub fn uploaded_at(mut self, timestamp: SystemTime) -> Self {
        self.uploaded_at = Some(timestamp);
        self
    }

    /// Set the last update timestamp
    #[must_use]
    pub fn updated_at(mut self, timestamp: SystemTime) -> Self {
        self.updated_at = Some(timestamp);
        self
    }

    /// Build the final Media entity
    #[must_use]
    pub fn build(self) -> Media {
        let now = SystemTime::now();
        Media {
            id: self.id,
            content_hash: self.content_hash,
            original_filename: self.original_filename,
            media_type: self.media_type,
            media_path: self.media_path,
            file_size: self.file_size,
            processing_status: self.processing_status,
            uploaded_by: self.uploaded_by.unwrap_or_default(),
            uploaded_at: self.uploaded_at.unwrap_or(now),
            updated_at: self.updated_at.unwrap_or(now),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
        let media_type = MediaType::new("image/jpeg");
        let media_path = "ab/cd/ef/abcdef123".to_string();
        let file_size = 1024;
        let user_id = create_test_user_id();

        let media = Media::new(
            content_hash.clone(),
            filename.clone(),
            media_type.clone(),
            media_path.clone(),
            file_size,
            user_id,
        );

        assert_eq!(media.content_hash, content_hash);
        assert_eq!(media.original_filename, filename);
        assert_eq!(media.media_type, media_type);
        assert_eq!(media.media_path, media_path);
        assert_eq!(media.file_size, file_size);
        assert_eq!(media.uploaded_by, user_id);
        assert_eq!(media.processing_status, ProcessingStatus::Pending);
        assert!(media.uploaded_at <= media.updated_at);
    }

    #[test]
    fn test_media_id_operations() {
        let id1 = MediaId::new(1);
        let id2 = MediaId::new(2);

        assert_ne!(id1, id2);
        assert_eq!(id1.as_i64(), 1);
        assert_eq!(id2.as_i64(), 2);

        let id_from_i64 = MediaId::from(42);
        assert_eq!(id_from_i64.as_i64(), 42);

        let id_string = id1.to_string();
        assert_eq!(id_string, "1");
    }

    #[test]
    fn test_processing_status_update() {
        let content_hash = create_test_content_hash();
        let media_type = MediaType::new("video/mp4");
        let user_id = create_test_user_id();

        let mut media = Media::new(
            content_hash,
            "test.mp4".to_string(),
            media_type,
            "ab/cd/ef/test".to_string(),
            2048,
            user_id,
        );

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
        let media_type = MediaType::new("image/png");
        let user_id = create_test_user_id();

        let mut media = Media::new(
            content_hash,
            "test.png".to_string(),
            media_type,
            "ab/cd/ef/test".to_string(),
            512,
            user_id,
        );

        media.set_processing_status(ProcessingStatus::Failed);

        assert!(media.processing_status.is_failed());
        assert!(!media.is_ready());
        assert!(media.has_failed());
    }

    #[test]
    fn test_updated_at_changes_on_status_update() {
        let content_hash = create_test_content_hash();
        let media_type = MediaType::new("image/webp");
        let user_id = create_test_user_id();

        let mut media = Media::new(
            content_hash,
            "test.webp".to_string(),
            media_type,
            "ab/cd/ef/test".to_string(),
            1024,
            user_id,
        );
        let initial_updated_at = media.updated_at;

        std::thread::sleep(std::time::Duration::from_millis(1));
        media.set_processing_status(ProcessingStatus::Complete);

        assert!(media.updated_at > initial_updated_at);
    }
}
