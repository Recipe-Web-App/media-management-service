use async_trait::async_trait;
use tokio::io::AsyncRead;

mod filesystem_storage;
pub mod presigned_urls;
pub mod utils;

pub use filesystem_storage::FilesystemStorage;
pub use presigned_urls::{
    PresignedUrlConfig, PresignedUrlError, PresignedUrlService, UploadSession,
};
pub use utils::*;

use crate::domain::value_objects::ContentHash;

/// Error types for storage operations
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("File not found: {path}")]
    FileNotFound { path: String },

    #[error("IO error: {message}")]
    IoError { message: String },

    #[error("Invalid path: {path}")]
    InvalidPath { path: String },

    #[error("Storage full or quota exceeded")]
    StorageFull,

    #[error("Hash mismatch: expected {expected}, got {actual}")]
    HashMismatch { expected: String, actual: String },
}

impl From<std::io::Error> for StorageError {
    fn from(error: std::io::Error) -> Self {
        StorageError::IoError { message: error.to_string() }
    }
}

/// Trait for file storage operations
#[async_trait]
pub trait FileStorage: Send + Sync {
    /// Store a file with its content hash
    async fn store<R>(&self, hash: &ContentHash, reader: R) -> Result<String, StorageError>
    where
        R: AsyncRead + Send + Unpin;

    /// Retrieve a file by its content hash
    async fn retrieve(
        &self,
        hash: &ContentHash,
    ) -> Result<Box<dyn AsyncRead + Send + Unpin>, StorageError>;

    /// Check if a file exists
    async fn exists(&self, hash: &ContentHash) -> Result<bool, StorageError>;

    /// Delete a file by its content hash
    async fn delete(&self, hash: &ContentHash) -> Result<bool, StorageError>;

    /// Get the file path for a given hash (for serving via filesystem)
    fn get_path(&self, hash: &ContentHash) -> String;

    /// Get metadata about a stored file
    async fn metadata(&self, hash: &ContentHash) -> Result<FileMetadata, StorageError>;

    /// Check storage system health
    ///
    /// Validates that the storage system is accessible and operational.
    /// This includes checking directory access, write permissions, and basic functionality.
    ///
    /// # Returns
    /// - `Ok(())` if storage is healthy and fully operational
    /// - `Err(StorageError)` if storage has issues that prevent normal operation
    ///
    /// # Timeout
    /// Implementation should complete within 2 seconds to avoid hanging health checks
    async fn health_check(&self) -> Result<(), StorageError>;
}

/// File metadata information
#[derive(Debug, Clone)]
pub struct FileMetadata {
    pub size: u64,
    pub content_type: Option<String>,
    pub last_modified: std::time::SystemTime,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::SystemTime;

    #[test]
    fn test_storage_error_creation() {
        let file_not_found = StorageError::FileNotFound { path: "test/path".to_string() };
        assert!(file_not_found.to_string().contains("test/path"));

        let io_error = StorageError::IoError { message: "Test IO error".to_string() };
        assert!(io_error.to_string().contains("Test IO error"));

        let invalid_path = StorageError::InvalidPath { path: "/invalid/path".to_string() };
        assert!(invalid_path.to_string().contains("/invalid/path"));

        let storage_full = StorageError::StorageFull;
        assert!(storage_full.to_string().contains("Storage full"));

        let hash_mismatch = StorageError::HashMismatch {
            expected: "abc123".to_string(),
            actual: "def456".to_string(),
        };
        assert!(hash_mismatch.to_string().contains("abc123"));
        assert!(hash_mismatch.to_string().contains("def456"));
    }

    #[test]
    fn test_storage_error_from_io_error() {
        let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "File not found");
        let storage_error = StorageError::from(io_error);

        match storage_error {
            StorageError::IoError { message } => {
                assert!(message.contains("File not found"));
            }
            _ => panic!("Expected IoError variant"),
        }
    }

    #[test]
    fn test_file_metadata_creation() {
        let now = SystemTime::now();
        let metadata = FileMetadata {
            size: 1024,
            content_type: Some("image/jpeg".to_string()),
            last_modified: now,
        };

        assert_eq!(metadata.size, 1024);
        assert_eq!(metadata.content_type, Some("image/jpeg".to_string()));
        assert_eq!(metadata.last_modified, now);
    }

    #[test]
    fn test_file_metadata_without_content_type() {
        let now = SystemTime::now();
        let metadata = FileMetadata { size: 2048, content_type: None, last_modified: now };

        assert_eq!(metadata.size, 2048);
        assert_eq!(metadata.content_type, None);
        assert_eq!(metadata.last_modified, now);
    }

    #[test]
    fn test_file_metadata_clone() {
        let now = SystemTime::now();
        let metadata = FileMetadata {
            size: 512,
            content_type: Some("text/plain".to_string()),
            last_modified: now,
        };

        let cloned_metadata = metadata.clone();
        assert_eq!(metadata.size, cloned_metadata.size);
        assert_eq!(metadata.content_type, cloned_metadata.content_type);
        assert_eq!(metadata.last_modified, cloned_metadata.last_modified);
    }

    #[test]
    fn test_storage_error_debug() {
        let error = StorageError::FileNotFound { path: "debug/test".to_string() };
        let debug_str = format!("{error:?}");
        assert!(debug_str.contains("FileNotFound"));
        assert!(debug_str.contains("debug/test"));
    }

    #[test]
    fn test_file_metadata_debug() {
        let now = SystemTime::now();
        let metadata = FileMetadata {
            size: 1024,
            content_type: Some("image/png".to_string()),
            last_modified: now,
        };

        let debug_str = format!("{metadata:?}");
        assert!(debug_str.contains("FileMetadata"));
        assert!(debug_str.contains("1024"));
        assert!(debug_str.contains("image/png"));
    }

    // Note: FileStorage trait tests would require mock implementations
    // These are better suited for integration tests with concrete implementations
    // The trait itself is tested through its implementations (FilesystemStorage)
}
