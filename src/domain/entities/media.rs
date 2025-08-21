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
