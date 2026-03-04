use std::fmt;
use std::ops::Deref;
use std::str::FromStr;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::AppError;

// ---------------------------------------------------------------------------
// DB row structs
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Media {
    pub media_id: i64,
    pub user_id: Uuid,
    pub content_hash: String,
    pub original_filename: String,
    pub media_type: String,
    pub media_path: String,
    pub file_size: i64,
    pub processing_status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct NewMedia {
    pub user_id: Uuid,
    pub content_hash: String,
    pub original_filename: String,
    pub media_type: String,
    pub media_path: String,
    pub file_size: i64,
    pub processing_status: String,
}

// ---------------------------------------------------------------------------
// Value objects
// ---------------------------------------------------------------------------

/// Validated SHA-256 content hash: exactly 64 lowercase hex characters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentHash(String);

impl ContentHash {
    pub fn new(s: &str) -> Result<Self, AppError> {
        if s.len() != 64 {
            return Err(AppError::BadRequest(format!(
                "content hash must be exactly 64 characters, got {}",
                s.len()
            )));
        }
        if !s.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')) {
            return Err(AppError::BadRequest(
                "content hash must be lowercase hex".to_string(),
            ));
        }
        Ok(Self(s.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Returns the CAS storage path: `"ab/cd/ef/abcdef1234..."`.
    pub fn cas_path(&self) -> String {
        let h = &self.0;
        format!("{}/{}/{}/{h}", &h[0..2], &h[2..4], &h[4..6])
    }
}

impl Deref for ContentHash {
    type Target = str;

    fn deref(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProcessingStatus {
    Pending,
    Processing,
    Complete,
    Failed,
}

impl fmt::Display for ProcessingStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Pending => "pending",
            Self::Processing => "processing",
            Self::Complete => "complete",
            Self::Failed => "failed",
        };
        f.write_str(s)
    }
}

impl FromStr for ProcessingStatus {
    type Err = AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "pending" => Ok(Self::Pending),
            "processing" => Ok(Self::Processing),
            "complete" => Ok(Self::Complete),
            "failed" => Ok(Self::Failed),
            _ => Err(AppError::Internal(format!(
                "unknown processing status: {s}"
            ))),
        }
    }
}

// ---------------------------------------------------------------------------
// API DTOs
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct MediaDto {
    pub id: i64,
    pub content_hash: String,
    pub original_filename: String,
    pub media_type: String,
    pub file_size: i64,
    pub processing_status: String,
    pub uploaded_by: String,
    pub uploaded_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub download_url: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PaginatedMediaResponse {
    pub data: Vec<MediaDto>,
    pub pagination: PaginationInfo,
}

#[derive(Debug, Serialize)]
pub struct PaginationInfo {
    pub next_cursor: Option<String>,
    pub prev_cursor: Option<String>,
    pub page_size: usize,
    pub has_next: bool,
    pub has_prev: bool,
}

#[derive(Debug, Deserialize)]
pub struct InitiateUploadRequest {
    pub filename: String,
    pub content_type: String,
    pub file_size: i64,
}

#[derive(Debug, Serialize)]
pub struct InitiateUploadResponse {
    pub media_id: i64,
    pub upload_url: String,
    pub upload_token: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct UploadStatusResponse {
    pub media_id: i64,
    pub status: String,
    pub error_message: Option<String>,
    pub download_url: Option<String>,
    pub uploaded_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct ListMediaQuery {
    pub cursor: Option<String>,
    pub limit: Option<i64>,
    pub status: Option<String>,
    pub uploaded_by: Option<String>,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    // --- ContentHash ---

    #[test]
    fn content_hash_accepts_valid_hash() {
        let hash = "a".repeat(64);
        assert!(ContentHash::new(&hash).is_ok());
    }

    #[rstest]
    #[case("abc")]
    #[case("")]
    #[case(&"a".repeat(65))]
    fn content_hash_rejects_wrong_length(#[case] input: &str) {
        assert!(matches!(
            ContentHash::new(input),
            Err(AppError::BadRequest(_))
        ));
    }

    #[rstest]
    #[case(&"A".repeat(64))]
    #[case(&"g".repeat(64))]
    #[case(&format!("{}Z", "a".repeat(63)))]
    fn content_hash_rejects_invalid_chars(#[case] input: &str) {
        assert!(matches!(
            ContentHash::new(input),
            Err(AppError::BadRequest(_))
        ));
    }

    #[test]
    fn content_hash_cas_path_format() {
        let hash_str = "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890";
        let hash = ContentHash::new(hash_str).unwrap();
        assert_eq!(hash.cas_path(), format!("ab/cd/ef/{hash_str}"));
    }

    #[test]
    fn content_hash_deref_returns_inner() {
        let hash_str = "f".repeat(64);
        let hash = ContentHash::new(&hash_str).unwrap();
        let s: &str = &hash;
        assert_eq!(s, hash_str);
    }

    #[test]
    fn content_hash_as_str_returns_inner() {
        let hash_str = "0123456789abcdef".repeat(4);
        let hash = ContentHash::new(&hash_str).unwrap();
        assert_eq!(hash.as_str(), hash_str);
    }

    // --- ProcessingStatus ---

    #[rstest]
    #[case("pending", ProcessingStatus::Pending)]
    #[case("processing", ProcessingStatus::Processing)]
    #[case("complete", ProcessingStatus::Complete)]
    #[case("failed", ProcessingStatus::Failed)]
    fn processing_status_from_str(#[case] input: &str, #[case] expected: ProcessingStatus) {
        assert_eq!(ProcessingStatus::from_str(input).unwrap(), expected);
    }

    #[rstest]
    #[case(ProcessingStatus::Pending, "pending")]
    #[case(ProcessingStatus::Processing, "processing")]
    #[case(ProcessingStatus::Complete, "complete")]
    #[case(ProcessingStatus::Failed, "failed")]
    fn processing_status_display(#[case] status: ProcessingStatus, #[case] expected: &str) {
        assert_eq!(status.to_string(), expected);
    }

    #[test]
    fn processing_status_unknown_is_internal_error() {
        assert!(matches!(
            ProcessingStatus::from_str("unknown"),
            Err(AppError::Internal(_))
        ));
    }

    #[rstest]
    #[case(ProcessingStatus::Pending, r#""pending""#)]
    #[case(ProcessingStatus::Complete, r#""complete""#)]
    fn processing_status_serde_serialize(#[case] status: ProcessingStatus, #[case] expected: &str) {
        assert_eq!(serde_json::to_string(&status).unwrap(), expected);
    }

    #[test]
    fn processing_status_serde_deserialize() {
        let status: ProcessingStatus = serde_json::from_str(r#""complete""#).unwrap();
        assert_eq!(status, ProcessingStatus::Complete);
    }
}
