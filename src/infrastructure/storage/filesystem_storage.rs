use async_trait::async_trait;
use std::path::PathBuf;
use tokio::fs;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt, BufReader};

use super::{utils::content_addressable_path, FileMetadata, FileStorage, StorageError};
use crate::domain::value_objects::ContentHash;

/// Filesystem-based storage implementation using content-addressable storage
#[derive(Clone)]
pub struct FilesystemStorage {
    base_path: PathBuf,
}

impl FilesystemStorage {
    /// Create a new filesystem storage instance
    pub fn new(base_path: impl Into<PathBuf>) -> Self {
        Self { base_path: base_path.into() }
    }

    /// Get the full filesystem path for a content hash
    fn full_path(&self, hash: &ContentHash) -> PathBuf {
        let content_path = content_addressable_path(hash);
        self.base_path.join(content_path)
    }

    /// Ensure directory structure exists for a file
    async fn ensure_directory(&self, file_path: &std::path::Path) -> Result<(), StorageError> {
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent).await?;
        }
        Ok(())
    }
}

#[async_trait]
impl FileStorage for FilesystemStorage {
    async fn store<R>(&self, hash: &ContentHash, mut reader: R) -> Result<String, StorageError>
    where
        R: AsyncRead + Send + Unpin,
    {
        let file_path = self.full_path(hash);

        // Check if file already exists (deduplication)
        if file_path.exists() {
            tracing::debug!("File already exists at path: {}", file_path.display());
            return Ok(file_path.to_string_lossy().to_string());
        }

        // Ensure directory structure exists
        self.ensure_directory(&file_path).await?;

        // Create temporary file first, then rename (atomic operation)
        let temp_path = file_path.with_extension("tmp");

        {
            let mut file = fs::File::create(&temp_path).await?;
            let mut buffer = [0u8; 8192];

            loop {
                let n = reader.read(&mut buffer).await?;
                if n == 0 {
                    break;
                }
                file.write_all(&buffer[..n]).await?;
            }

            file.flush().await?;
        }

        // Atomic rename
        fs::rename(&temp_path, &file_path).await?;

        tracing::info!("Stored file at path: {}", file_path.display());
        Ok(file_path.to_string_lossy().to_string())
    }

    async fn retrieve(
        &self,
        hash: &ContentHash,
    ) -> Result<Box<dyn AsyncRead + Send + Unpin>, StorageError> {
        let file_path = self.full_path(hash);

        if !file_path.exists() {
            return Err(StorageError::FileNotFound {
                path: file_path.to_string_lossy().to_string(),
            });
        }

        let file = fs::File::open(&file_path).await?;
        let reader = BufReader::new(file);

        Ok(Box::new(reader))
    }

    async fn exists(&self, hash: &ContentHash) -> Result<bool, StorageError> {
        let file_path = self.full_path(hash);
        Ok(file_path.exists())
    }

    async fn delete(&self, hash: &ContentHash) -> Result<bool, StorageError> {
        let file_path = self.full_path(hash);

        if !file_path.exists() {
            return Ok(false);
        }

        fs::remove_file(&file_path).await?;

        // Try to clean up empty directories (best effort)
        if let Some(parent) = file_path.parent() {
            let () = self.cleanup_empty_directories(parent).await;
        }

        tracing::info!("Deleted file at path: {}", file_path.display());
        Ok(true)
    }

    fn get_path(&self, hash: &ContentHash) -> String {
        self.full_path(hash).to_string_lossy().to_string()
    }

    async fn metadata(&self, hash: &ContentHash) -> Result<FileMetadata, StorageError> {
        let file_path = self.full_path(hash);

        if !file_path.exists() {
            return Err(StorageError::FileNotFound {
                path: file_path.to_string_lossy().to_string(),
            });
        }

        let metadata = fs::metadata(&file_path).await?;

        Ok(FileMetadata {
            size: metadata.len(),
            content_type: None, // Could be determined by reading file header if needed
            last_modified: metadata.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH),
        })
    }

    async fn health_check(&self) -> Result<(), StorageError> {
        // Check if base directory exists and is accessible
        if !self.base_path.exists() {
            return Err(StorageError::FileNotFound {
                path: self.base_path.to_string_lossy().to_string(),
            });
        }

        // Check if base path is a directory
        let metadata = fs::metadata(&self.base_path).await?;
        if !metadata.is_dir() {
            return Err(StorageError::InvalidPath {
                path: format!("{} is not a directory", self.base_path.display()),
            });
        }

        // Test write permissions by creating a temporary test file
        let test_filename = format!(".health_check_{}", uuid::Uuid::new_v4());
        let test_path = self.base_path.join(&test_filename);

        // Try to create and write to test file
        match fs::File::create(&test_path).await {
            Ok(mut file) => {
                // Try to write test content
                if let Err(e) = file.write_all(b"health_check").await {
                    return Err(StorageError::IoError {
                        message: format!("Write test failed: {e}"),
                    });
                }

                // Try to flush to ensure write succeeds
                if let Err(e) = file.flush().await {
                    return Err(StorageError::IoError {
                        message: format!("Flush test failed: {e}"),
                    });
                }
            }
            Err(e) => {
                return Err(StorageError::IoError {
                    message: format!("Write permission test failed: {e}"),
                });
            }
        }

        // Clean up test file (ignore errors as cleanup is best effort)
        let _ = fs::remove_file(&test_path).await;

        Ok(())
    }
}

impl FilesystemStorage {
    /// Clean up empty directories (best effort, ignore errors)
    async fn cleanup_empty_directories(&self, mut dir_path: &std::path::Path) {
        // Only clean up directories within our base path
        while dir_path != self.base_path && dir_path.starts_with(&self.base_path) {
            match fs::read_dir(dir_path).await {
                Ok(mut entries) => {
                    // Check if directory is empty
                    if entries.next_entry().await.unwrap_or(None).is_some() {
                        break; // Directory not empty
                    }

                    // Try to remove empty directory
                    if fs::remove_dir(dir_path).await.is_err() {
                        break; // Failed to remove, stop
                    }

                    tracing::debug!("Cleaned up empty directory: {}", dir_path.display());
                }
                Err(_) => break, // Can't read directory, stop
            }

            // Move to parent directory
            if let Some(parent) = dir_path.parent() {
                dir_path = parent;
            } else {
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use tempfile::TempDir;

    fn create_test_hash() -> ContentHash {
        ContentHash::new("abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890")
            .unwrap()
    }

    #[tokio::test]
    async fn test_filesystem_storage_store_and_retrieve() {
        let temp_dir = TempDir::new().unwrap();
        let storage = FilesystemStorage::new(temp_dir.path());
        let hash = create_test_hash();

        let data = b"test file content";
        let reader = Cursor::new(data);

        // Store file
        let stored_path = storage.store(&hash, reader).await.unwrap();
        assert!(stored_path.contains("ab/cd/ef"));

        // Verify file exists
        let exists = storage.exists(&hash).await.unwrap();
        assert!(exists);

        // Retrieve file
        let mut retrieved_reader = storage.retrieve(&hash).await.unwrap();
        let mut retrieved_data = Vec::new();
        retrieved_reader.read_to_end(&mut retrieved_data).await.unwrap();

        assert_eq!(retrieved_data, data.to_vec());
    }

    #[tokio::test]
    async fn test_filesystem_storage_metadata() {
        let temp_dir = TempDir::new().unwrap();
        let storage = FilesystemStorage::new(temp_dir.path());
        let hash = create_test_hash();

        let data = b"test file content for metadata";
        let reader = Cursor::new(data);

        // Store file
        storage.store(&hash, reader).await.unwrap();

        // Get metadata
        let metadata = storage.metadata(&hash).await.unwrap();
        assert_eq!(metadata.size, data.len() as u64);
        assert!(metadata.last_modified > std::time::SystemTime::UNIX_EPOCH);
    }

    #[tokio::test]
    async fn test_filesystem_storage_delete() {
        let temp_dir = TempDir::new().unwrap();
        let storage = FilesystemStorage::new(temp_dir.path());
        let hash = create_test_hash();

        let data = b"test file content";
        let reader = Cursor::new(data);

        // Store file
        storage.store(&hash, reader).await.unwrap();
        assert!(storage.exists(&hash).await.unwrap());

        // Delete file
        let deleted = storage.delete(&hash).await.unwrap();
        assert!(deleted);
        assert!(!storage.exists(&hash).await.unwrap());

        // Delete non-existent file
        let deleted_again = storage.delete(&hash).await.unwrap();
        assert!(!deleted_again);
    }

    #[tokio::test]
    async fn test_filesystem_storage_deduplication() {
        let temp_dir = TempDir::new().unwrap();
        let storage = FilesystemStorage::new(temp_dir.path());
        let hash = create_test_hash();

        let data = b"test file content";

        // Store file first time
        let reader1 = Cursor::new(data);
        let path1 = storage.store(&hash, reader1).await.unwrap();

        // Store same file again (should deduplicate)
        let reader2 = Cursor::new(data);
        let path2 = storage.store(&hash, reader2).await.unwrap();

        assert_eq!(path1, path2);
        assert!(storage.exists(&hash).await.unwrap());
    }

    #[tokio::test]
    async fn test_filesystem_storage_error_cases() {
        let temp_dir = TempDir::new().unwrap();
        let storage = FilesystemStorage::new(temp_dir.path());
        let hash = create_test_hash();

        // Try to retrieve non-existent file
        let result = storage.retrieve(&hash).await;
        assert!(matches!(result, Err(StorageError::FileNotFound { .. })));

        // Try to get metadata for non-existent file
        let result = storage.metadata(&hash).await;
        assert!(matches!(result, Err(StorageError::FileNotFound { .. })));

        // Non-existent file should not exist
        let exists = storage.exists(&hash).await.unwrap();
        assert!(!exists);
    }

    #[test]
    fn test_filesystem_storage_get_path() {
        let temp_dir = TempDir::new().unwrap();
        let storage = FilesystemStorage::new(temp_dir.path());
        let hash = create_test_hash();

        let path = storage.get_path(&hash);
        assert!(path.contains("ab/cd/ef"));
        assert!(path.contains("abcdef1234567890"));
    }

    #[tokio::test]
    async fn test_health_check_healthy_storage() {
        let temp_dir = TempDir::new().unwrap();
        let storage = FilesystemStorage::new(temp_dir.path());

        // Health check should pass for valid, writable directory
        let result = storage.health_check().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_health_check_missing_directory() {
        // Use a path that doesn't exist
        let non_existent_path = "/this/path/should/not/exist/test";
        let storage = FilesystemStorage::new(non_existent_path);

        let result = storage.health_check().await;
        assert!(result.is_err());
        match result.unwrap_err() {
            StorageError::FileNotFound { path } => {
                assert!(path.contains("this/path/should/not/exist"));
            }
            _ => panic!("Expected FileNotFound error"),
        }
    }

    #[tokio::test]
    async fn test_health_check_file_instead_of_directory() {
        let temp_dir = TempDir::new().unwrap();

        // Create a regular file
        let file_path = temp_dir.path().join("test_file");
        tokio::fs::write(&file_path, b"test content").await.unwrap();

        // Try to use the file as a storage directory
        let storage = FilesystemStorage::new(&file_path);

        let result = storage.health_check().await;
        assert!(result.is_err());
        match result.unwrap_err() {
            StorageError::InvalidPath { path } => {
                assert!(path.contains("is not a directory"));
            }
            _ => panic!("Expected InvalidPath error"),
        }
    }

    #[tokio::test]
    async fn test_health_check_write_permissions() {
        let temp_dir = TempDir::new().unwrap();
        let storage = FilesystemStorage::new(temp_dir.path());

        // Health check should create and clean up a test file
        let result = storage.health_check().await;
        assert!(result.is_ok());

        // Verify no health check files are left behind
        let mut entries = tokio::fs::read_dir(temp_dir.path()).await.unwrap();
        let mut found_health_files = false;
        while let Some(entry) = entries.next_entry().await.unwrap() {
            if entry.file_name().to_string_lossy().starts_with(".health_check_") {
                found_health_files = true;
                break;
            }
        }
        assert!(!found_health_files, "Health check should clean up test files");
    }

    #[tokio::test]
    async fn test_health_check_concurrent_access() {
        let temp_dir = TempDir::new().unwrap();
        let storage = FilesystemStorage::new(temp_dir.path());

        // Run multiple health checks concurrently
        let mut handles = vec![];
        for _ in 0..5 {
            let storage_clone = storage.clone();
            let handle = tokio::spawn(async move { storage_clone.health_check().await });
            handles.push(handle);
        }

        // All should succeed
        for handle in handles {
            let result = handle.await.unwrap();
            assert!(result.is_ok());
        }
    }
}
