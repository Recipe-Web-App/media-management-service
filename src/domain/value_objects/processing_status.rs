use serde::{Deserialize, Serialize};

/// Processing status for media files
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProcessingStatus {
    /// File has been uploaded and is waiting for processing
    Pending,
    /// File is currently being processed (thumbnails, optimization, etc.)
    Processing,
    /// Processing completed successfully
    Complete,
    /// Processing failed with an error message
    Failed(String),
}

impl ProcessingStatus {
    /// Check if the status indicates processing is complete
    #[must_use]
    pub fn is_complete(&self) -> bool {
        matches!(self, Self::Complete)
    }

    /// Check if the status indicates processing failed
    #[must_use]
    pub fn is_failed(&self) -> bool {
        matches!(self, Self::Failed(_))
    }

    /// Check if the status indicates processing is in progress
    #[must_use]
    pub fn is_processing(&self) -> bool {
        matches!(self, Self::Processing)
    }

    /// Check if the status indicates processing is pending
    #[must_use]
    pub fn is_pending(&self) -> bool {
        matches!(self, Self::Pending)
    }

    /// Get the error message if processing failed
    #[must_use]
    pub fn error_message(&self) -> Option<&str> {
        match self {
            Self::Failed(msg) => Some(msg),
            _ => None,
        }
    }
}

impl std::fmt::Display for ProcessingStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Processing => write!(f, "processing"),
            Self::Complete => write!(f, "complete"),
            Self::Failed(msg) => write!(f, "failed: {msg}"),
        }
    }
}
