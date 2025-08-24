use async_trait::async_trait;
use tokio::io::AsyncRead;

mod filesystem_storage;
pub mod utils;

pub use filesystem_storage::FilesystemStorage;
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
}

/// File metadata information
#[derive(Debug, Clone)]
pub struct FileMetadata {
    pub size: u64,
    pub content_type: Option<String>,
    pub last_modified: std::time::SystemTime,
}

// Mock removed due to complexity with generic types
// Integration tests with real storage are more appropriate for this trait
