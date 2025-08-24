use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// Processing status for media files matching database enum
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProcessingStatus {
    /// File has been uploaded and is waiting for processing
    Pending,
    /// File is currently being processed (thumbnails, optimization, etc.)
    Processing,
    /// Processing completed successfully
    Complete,
    /// Processing failed (error details stored separately if needed)
    Failed,
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
        matches!(self, Self::Failed)
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
}

impl std::fmt::Display for ProcessingStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pending => write!(f, "PENDING"),
            Self::Processing => write!(f, "PROCESSING"),
            Self::Complete => write!(f, "COMPLETE"),
            Self::Failed => write!(f, "FAILED"),
        }
    }
}

impl FromStr for ProcessingStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "PENDING" => Ok(Self::Pending),
            "PROCESSING" => Ok(Self::Processing),
            "COMPLETE" => Ok(Self::Complete),
            "FAILED" => Ok(Self::Failed),
            _ => Err(format!("Invalid processing status: {s}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pending_status() {
        let status = ProcessingStatus::Pending;
        assert!(status.is_pending());
        assert!(!status.is_processing());
        assert!(!status.is_complete());
        assert!(!status.is_failed());
        assert_eq!(status.to_string(), "PENDING");
    }

    #[test]
    fn test_processing_status() {
        let status = ProcessingStatus::Processing;
        assert!(!status.is_pending());
        assert!(status.is_processing());
        assert!(!status.is_complete());
        assert!(!status.is_failed());
        assert_eq!(status.to_string(), "PROCESSING");
    }

    #[test]
    fn test_complete_status() {
        let status = ProcessingStatus::Complete;
        assert!(!status.is_pending());
        assert!(!status.is_processing());
        assert!(status.is_complete());
        assert!(!status.is_failed());
        assert_eq!(status.to_string(), "COMPLETE");
    }

    #[test]
    fn test_failed_status() {
        let status = ProcessingStatus::Failed;

        assert!(!status.is_pending());
        assert!(!status.is_processing());
        assert!(!status.is_complete());
        assert!(status.is_failed());
        assert_eq!(status.to_string(), "FAILED");
    }

    #[test]
    fn test_from_str() {
        assert_eq!("PENDING".parse::<ProcessingStatus>().unwrap(), ProcessingStatus::Pending);
        assert_eq!("processing".parse::<ProcessingStatus>().unwrap(), ProcessingStatus::Processing);
        assert_eq!("Complete".parse::<ProcessingStatus>().unwrap(), ProcessingStatus::Complete);
        assert_eq!("FAILED".parse::<ProcessingStatus>().unwrap(), ProcessingStatus::Failed);

        assert!("INVALID".parse::<ProcessingStatus>().is_err());
    }
}
