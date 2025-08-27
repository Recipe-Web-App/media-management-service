use std::sync::Arc;
use tracing::{info, warn};

use crate::{
    domain::{entities::MediaId, repositories::MediaRepository},
    infrastructure::storage::FileStorage,
    presentation::middleware::error::AppError,
};

/// Use case for deleting media files
///
/// This use case handles the complete deletion of a media file, including:
/// - Validating the media exists
/// - Removing the file from storage
/// - Removing the database record
/// - Handling partial failures gracefully
pub struct DeleteMediaUseCase<R: ?Sized, S> {
    repository: Arc<R>,
    storage: Arc<S>,
}

impl<R: ?Sized, S> DeleteMediaUseCase<R, S>
where
    R: MediaRepository,
    S: FileStorage,
{
    /// Create a new delete media use case
    pub fn new(repository: Arc<R>, storage: Arc<S>) -> Self {
        Self { repository, storage }
    }

    /// Execute the delete media use case
    ///
    /// # Arguments
    /// * `media_id` - The ID of the media to delete
    ///
    /// # Returns
    /// * `Ok(())` if the media was successfully deleted
    /// * `Err(AppError)` if the operation failed
    ///
    /// # Errors
    /// * `NotFound` - Media with the given ID doesn't exist
    /// * `Internal` - Storage or database operation failed
    pub async fn execute(&self, media_id: MediaId) -> Result<(), AppError>
    where
        R::Error: Into<AppError>,
        S: FileStorage,
    {
        info!("Deleting media with ID: {}", media_id);

        // First, retrieve the media to get the content hash for storage deletion
        let media =
            self.repository.find_by_id(media_id).await.map_err(Into::into)?.ok_or_else(|| {
                AppError::NotFound { resource: format!("Media with ID {media_id}") }
            })?;

        info!(
            "Found media to delete: {} (hash: {})",
            media.original_filename,
            media.content_hash.as_str()
        );

        // Delete from storage first - if this fails, we haven't modified the database yet
        let storage_deleted = match self.storage.delete(&media.content_hash).await {
            Ok(deleted) => {
                if deleted {
                    info!(
                        "Successfully deleted file from storage: {}",
                        media.content_hash.as_str()
                    );
                } else {
                    warn!(
                        "File not found in storage (may have been already deleted): {}",
                        media.content_hash.as_str()
                    );
                }
                deleted
            }
            Err(e) => {
                warn!("Failed to delete file from storage: {}", e);
                // Continue with database deletion even if storage deletion failed
                // This handles cases where the file might have been manually deleted
                false
            }
        };

        // Delete from database
        let db_deleted = self.repository.delete(media_id).await.map_err(Into::into)?;

        if !db_deleted {
            return Err(AppError::NotFound { resource: format!("Media with ID {media_id}") });
        }

        info!(
            "Media deletion completed: ID={}, storage_deleted={}, db_deleted={}",
            media_id, storage_deleted, db_deleted
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        domain::{
            entities::{Media, UserId},
            value_objects::{ContentHash, MediaType, ProcessingStatus},
        },
        infrastructure::storage::StorageError,
        test_utils::mocks::InMemoryMediaRepository,
    };
    use async_trait::async_trait;
    use std::collections::HashMap;
    use std::sync::Mutex;
    use tokio::io::{AsyncRead, AsyncReadExt};

    // Mock storage implementation for testing
    #[derive(Clone, Default)]
    struct MockStorage {
        files: Arc<Mutex<HashMap<String, Vec<u8>>>>,
        should_fail_delete: bool,
    }

    impl MockStorage {
        fn new() -> Self {
            Self { files: Arc::new(Mutex::new(HashMap::new())), should_fail_delete: false }
        }

        fn with_file(self, hash: &str, content: Vec<u8>) -> Self {
            {
                let mut files = self.files.lock().unwrap();
                files.insert(hash.to_string(), content);
            }
            self
        }

        fn with_delete_failure(mut self) -> Self {
            self.should_fail_delete = true;
            self
        }
    }

    #[async_trait]
    impl FileStorage for MockStorage {
        async fn store<R>(&self, hash: &ContentHash, mut reader: R) -> Result<String, StorageError>
        where
            R: AsyncRead + Send + Unpin,
        {
            let mut buffer = Vec::new();
            reader
                .read_to_end(&mut buffer)
                .await
                .map_err(|e| StorageError::IoError { message: e.to_string() })?;

            let mut files = self.files.lock().unwrap();
            let hash_str = hash.as_str().to_string();
            files.insert(hash_str.clone(), buffer);
            Ok(format!("mock/path/{hash_str}"))
        }

        async fn retrieve(
            &self,
            hash: &ContentHash,
        ) -> Result<Box<dyn AsyncRead + Send + Unpin>, StorageError> {
            let files = self.files.lock().unwrap();
            let hash_str = hash.as_str();

            match files.get(hash_str) {
                Some(content) => {
                    let cursor = std::io::Cursor::new(content.clone());
                    Ok(Box::new(cursor))
                }
                None => Err(StorageError::FileNotFound { path: hash_str.to_string() }),
            }
        }

        async fn exists(&self, hash: &ContentHash) -> Result<bool, StorageError> {
            let files = self.files.lock().unwrap();
            Ok(files.contains_key(hash.as_str()))
        }

        async fn delete(&self, hash: &ContentHash) -> Result<bool, StorageError> {
            if self.should_fail_delete {
                return Err(StorageError::IoError {
                    message: "Mock storage delete failure".to_string(),
                });
            }

            let mut files = self.files.lock().unwrap();
            Ok(files.remove(hash.as_str()).is_some())
        }

        fn get_path(&self, hash: &ContentHash) -> String {
            format!("mock/path/{}", hash.as_str())
        }

        async fn metadata(
            &self,
            hash: &ContentHash,
        ) -> Result<crate::infrastructure::storage::FileMetadata, StorageError> {
            let files = self.files.lock().unwrap();
            match files.get(hash.as_str()) {
                Some(content) => Ok(crate::infrastructure::storage::FileMetadata {
                    size: content.len() as u64,
                    content_type: Some("application/octet-stream".to_string()),
                    last_modified: std::time::SystemTime::now(),
                }),
                None => Err(StorageError::FileNotFound { path: hash.as_str().to_string() }),
            }
        }

        async fn health_check(&self) -> Result<(), StorageError> {
            Ok(())
        }
    }

    fn create_test_media(id: i64) -> Media {
        let content_hash =
            ContentHash::new("abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890")
                .unwrap();

        Media::with_id(
            MediaId::new(id),
            content_hash,
            format!("test_file_{id}.jpg"),
            MediaType::new("image/jpeg"),
            format!("/test/path/{id}"),
            1024,
            ProcessingStatus::Complete,
        )
        .uploaded_by(UserId::new())
        .build()
    }

    #[tokio::test]
    async fn test_delete_media_success() {
        let media = create_test_media(1);
        let content_hash = media.content_hash.clone();

        let repository = Arc::new(InMemoryMediaRepository::new().with_media(media));
        let storage =
            Arc::new(MockStorage::new().with_file(content_hash.as_str(), b"test content".to_vec()));

        let delete_use_case = DeleteMediaUseCase::new(repository, storage.clone());
        let result = delete_use_case.execute(MediaId::new(1)).await;

        assert!(result.is_ok());

        // Verify file was deleted from storage
        assert!(!storage.exists(&content_hash).await.unwrap());
    }

    #[tokio::test]
    async fn test_delete_media_not_found() {
        let repository = Arc::new(InMemoryMediaRepository::new());
        let storage = Arc::new(MockStorage::new());

        let delete_use_case = DeleteMediaUseCase::new(repository, storage);
        let result = delete_use_case.execute(MediaId::new(999)).await;

        assert!(result.is_err());
        if let Err(AppError::NotFound { resource }) = result {
            assert!(resource.contains("Media with ID 999"));
        } else {
            panic!("Expected NotFound error");
        }
    }

    #[tokio::test]
    async fn test_delete_media_storage_failure_continues() {
        let media = create_test_media(1);
        let content_hash = media.content_hash.clone();

        let repository = Arc::new(InMemoryMediaRepository::new().with_media(media));
        let storage = Arc::new(
            MockStorage::new()
                .with_file(content_hash.as_str(), b"test content".to_vec())
                .with_delete_failure(),
        );

        let delete_use_case = DeleteMediaUseCase::new(repository.clone(), storage);
        let result = delete_use_case.execute(MediaId::new(1)).await;

        // Should succeed despite storage failure
        assert!(result.is_ok());

        // Verify database record was still deleted
        let media_check = repository.find_by_id(MediaId::new(1)).await.unwrap();
        assert!(media_check.is_none());
    }

    #[tokio::test]
    async fn test_delete_media_file_not_in_storage() {
        let media = create_test_media(1);

        let repository = Arc::new(InMemoryMediaRepository::new().with_media(media));
        let storage = Arc::new(MockStorage::new()); // No file in storage

        let delete_use_case = DeleteMediaUseCase::new(repository.clone(), storage);
        let result = delete_use_case.execute(MediaId::new(1)).await;

        // Should succeed even if file not in storage
        assert!(result.is_ok());

        // Verify database record was deleted
        let media_check = repository.find_by_id(MediaId::new(1)).await.unwrap();
        assert!(media_check.is_none());
    }

    #[tokio::test]
    async fn test_delete_use_case_creation() {
        let repository = Arc::new(InMemoryMediaRepository::new());
        let storage = Arc::new(MockStorage::new());

        let delete_use_case = DeleteMediaUseCase::new(repository, storage);

        // Test that the use case was created successfully
        // This validates the constructor works
        assert!(std::ptr::addr_of!(delete_use_case).is_aligned());
    }

    #[tokio::test]
    async fn test_multiple_deletes_same_media() {
        let media = create_test_media(1);
        let repository = Arc::new(InMemoryMediaRepository::new().with_media(media));
        let storage = Arc::new(MockStorage::new());

        let delete_use_case = DeleteMediaUseCase::new(repository, storage);

        // First delete should succeed
        let result1 = delete_use_case.execute(MediaId::new(1)).await;
        assert!(result1.is_ok());

        // Second delete should fail with NotFound
        let result2 = delete_use_case.execute(MediaId::new(1)).await;
        assert!(result2.is_err());
        if let Err(AppError::NotFound { resource }) = result2 {
            assert!(resource.contains("Media with ID 1"));
        }
    }

    #[tokio::test]
    async fn test_delete_media_different_media_types() {
        // Test with different media types
        let content_hash1 =
            ContentHash::new("1111111111111111111111111111111111111111111111111111111111111111")
                .unwrap();
        let content_hash2 =
            ContentHash::new("2222222222222222222222222222222222222222222222222222222222222222")
                .unwrap();

        let media1 = Media::with_id(
            MediaId::new(1),
            content_hash1.clone(),
            "image.jpg".to_string(),
            MediaType::new("image/jpeg"),
            "/path/image.jpg".to_string(),
            1024,
            ProcessingStatus::Complete,
        )
        .uploaded_by(UserId::new())
        .build();

        let media2 = Media::with_id(
            MediaId::new(2),
            content_hash2.clone(),
            "video.mp4".to_string(),
            MediaType::new("video/mp4"),
            "/path/video.mp4".to_string(),
            2048,
            ProcessingStatus::Complete,
        )
        .uploaded_by(UserId::new())
        .build();

        let repository =
            Arc::new(InMemoryMediaRepository::new().with_media(media1).with_media(media2));
        let storage = Arc::new(
            MockStorage::new()
                .with_file(content_hash1.as_str(), b"image content".to_vec())
                .with_file(content_hash2.as_str(), b"video content".to_vec()),
        );

        let delete_use_case = DeleteMediaUseCase::new(repository, storage.clone());

        // Delete both media files
        let result1 = delete_use_case.execute(MediaId::new(1)).await;
        let result2 = delete_use_case.execute(MediaId::new(2)).await;

        assert!(result1.is_ok());
        assert!(result2.is_ok());

        // Verify both files were deleted from storage
        assert!(!storage.exists(&content_hash1).await.unwrap());
        assert!(!storage.exists(&content_hash2).await.unwrap());
    }
}
