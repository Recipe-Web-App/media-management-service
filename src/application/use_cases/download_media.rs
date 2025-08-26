use std::sync::Arc;
use tokio::io::{AsyncRead, AsyncReadExt};

use crate::{
    domain::{
        entities::{Media, MediaId},
        repositories::MediaRepository,
    },
    infrastructure::storage::{FileStorage, StorageError},
    presentation::middleware::error::AppError,
};

/// Use case for downloading media files
pub struct DownloadMediaUseCase<R, S>
where
    R: MediaRepository,
    S: FileStorage,
{
    repository: Arc<R>,
    storage: Arc<S>,
}

/// Download response containing file data and metadata
#[derive(Debug)]
pub struct DownloadResponse {
    pub content: Vec<u8>,
    pub content_type: String,
    pub filename: String,
    pub file_size: u64,
}

impl<R, S> DownloadMediaUseCase<R, S>
where
    R: MediaRepository,
    S: FileStorage,
{
    /// Create a new download media use case
    pub fn new(repository: Arc<R>, storage: Arc<S>) -> Self {
        Self { repository, storage }
    }

    /// Execute the download media use case
    pub async fn execute(&self, media_id: MediaId) -> Result<DownloadResponse, AppError> {
        tracing::info!("Downloading media with ID: {}", media_id);

        // Get media metadata from database
        let media =
            self.repository.find_by_id(media_id).await.map_err(|e| AppError::Internal {
                message: format!("Failed to query media: {e}"),
            })?;

        let Some(media) = media else {
            tracing::warn!("Media not found with ID: {}", media_id);
            return Err(AppError::NotFound { resource: format!("Media with ID {media_id}") });
        };

        // Check if media processing is complete
        if !media.is_ready() {
            tracing::warn!(
                "Media not ready for download: {} (status: {:?})",
                media_id,
                media.processing_status
            );
            return Err(AppError::BadRequest {
                message: format!(
                    "Media is not ready for download. Status: {:?}",
                    media.processing_status
                ),
            });
        }

        tracing::info!("Retrieving file from storage: {}", media.content_hash.as_str());

        // Get file from storage
        let mut file_reader =
            self.storage.retrieve(&media.content_hash).await.map_err(|e| match e {
                StorageError::FileNotFound { .. } => {
                    AppError::NotFound { resource: format!("File content for media {media_id}") }
                }
                _ => AppError::Internal { message: format!("Storage error: {e}") },
            })?;

        // Read file content
        let mut content = Vec::new();
        file_reader.read_to_end(&mut content).await.map_err(|e| AppError::Internal {
            message: format!("Failed to read file content: {e}"),
        })?;

        tracing::info!(
            "Successfully downloaded media: {} ({} bytes)",
            media.original_filename,
            content.len()
        );

        Ok(DownloadResponse {
            content,
            content_type: media.media_type.mime_type().to_string(),
            filename: media.original_filename,
            file_size: media.file_size,
        })
    }

    /// Execute download and return streaming reader (for large files)
    /// This method returns the reader directly without loading the entire file into memory
    pub async fn execute_stream(
        &self,
        media_id: MediaId,
    ) -> Result<(Box<dyn AsyncRead + Send + Unpin>, Media), AppError> {
        tracing::info!("Streaming media with ID: {}", media_id);

        // Get media metadata from database
        let media =
            self.repository.find_by_id(media_id).await.map_err(|e| AppError::Internal {
                message: format!("Failed to query media: {e}"),
            })?;

        let Some(media) = media else {
            tracing::warn!("Media not found with ID: {}", media_id);
            return Err(AppError::NotFound { resource: format!("Media with ID {media_id}") });
        };

        // Check if media processing is complete
        if !media.is_ready() {
            tracing::warn!(
                "Media not ready for streaming: {} (status: {:?})",
                media_id,
                media.processing_status
            );
            return Err(AppError::BadRequest {
                message: format!(
                    "Media is not ready for download. Status: {:?}",
                    media.processing_status
                ),
            });
        }

        tracing::info!("Streaming file from storage: {}", media.content_hash.as_str());

        // Get file reader from storage
        let file_reader =
            self.storage.retrieve(&media.content_hash).await.map_err(|e| match e {
                StorageError::FileNotFound { .. } => {
                    AppError::NotFound { resource: format!("File content for media {media_id}") }
                }
                _ => AppError::Internal { message: format!("Storage error: {e}") },
            })?;

        tracing::info!("Successfully initiated streaming for media: {}", media.original_filename);

        Ok((file_reader, media))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        domain::{
            entities::{MediaId, UserId},
            value_objects::{ContentHash, MediaType, ProcessingStatus},
        },
        infrastructure::storage::{FileStorage, StorageError},
        test_utils::mocks::InMemoryMediaRepository,
    };
    use async_trait::async_trait;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};
    use tokio::io::{AsyncRead, AsyncReadExt};

    // Mock storage for download testing
    #[derive(Clone, Default)]
    pub struct MockDownloadStorage {
        files: Arc<Mutex<HashMap<String, Vec<u8>>>>,
        simulate_error: Arc<Mutex<Option<StorageError>>>,
    }

    impl MockDownloadStorage {
        pub fn new() -> Self {
            Self {
                files: Arc::new(Mutex::new(HashMap::new())),
                simulate_error: Arc::new(Mutex::new(None)),
            }
        }

        pub fn with_file(self, hash: &str, content: Vec<u8>) -> Self {
            {
                let mut files = self.files.lock().unwrap();
                files.insert(hash.to_string(), content);
            }
            self
        }

        pub fn set_error(&self, error: StorageError) {
            let mut sim_error = self.simulate_error.lock().unwrap();
            *sim_error = Some(error);
        }
    }

    #[async_trait]
    impl FileStorage for MockDownloadStorage {
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
            files.insert(hash.as_str().to_string(), buffer);
            Ok(format!("mock/path/{}", hash.as_str()))
        }

        async fn retrieve(
            &self,
            hash: &ContentHash,
        ) -> Result<Box<dyn AsyncRead + Send + Unpin>, StorageError> {
            // Check if we should simulate an error
            {
                let sim_error = self.simulate_error.lock().unwrap();
                if let Some(ref error) = *sim_error {
                    return match error {
                        StorageError::FileNotFound { path } => {
                            Err(StorageError::FileNotFound { path: path.clone() })
                        }
                        StorageError::IoError { message } => {
                            Err(StorageError::IoError { message: message.clone() })
                        }
                        _ => Err(StorageError::IoError { message: "Simulated error".to_string() }),
                    };
                }
            }

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
    }

    fn create_test_media(id: MediaId, status: ProcessingStatus) -> Media {
        let content_hash =
            ContentHash::new("abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890")
                .unwrap();

        Media::with_id(
            id,
            content_hash,
            "test.jpg".to_string(),
            MediaType::new("image/jpeg"),
            "/path/to/test.jpg".to_string(),
            1024,
            status,
        )
        .uploaded_by(UserId::new())
        .build()
    }

    #[tokio::test]
    async fn test_download_media_success() {
        let test_content = b"test image content".to_vec();
        let content_hash =
            ContentHash::new("abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890")
                .unwrap();

        let media = create_test_media(MediaId::new(1), ProcessingStatus::Complete);
        let repository = Arc::new(InMemoryMediaRepository::new().with_media(media));
        let storage = Arc::new(
            MockDownloadStorage::new().with_file(content_hash.as_str(), test_content.clone()),
        );

        let use_case = DownloadMediaUseCase::new(repository, storage);
        let result = use_case.execute(MediaId::new(1)).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.content, test_content);
        assert_eq!(response.filename, "test.jpg");
        assert_eq!(response.content_type, "image/jpeg");
        assert_eq!(response.file_size, 1024);
    }

    #[tokio::test]
    async fn test_download_media_not_found() {
        let repository = Arc::new(InMemoryMediaRepository::new());
        let storage = Arc::new(MockDownloadStorage::new());

        let use_case = DownloadMediaUseCase::new(repository, storage);
        let result = use_case.execute(MediaId::new(999)).await;

        assert!(result.is_err());
        match result.err().unwrap() {
            AppError::NotFound { resource } => assert!(resource.contains("999")),
            _ => panic!("Expected NotFound error"),
        }
    }

    #[tokio::test]
    async fn test_download_media_not_ready() {
        let media = create_test_media(MediaId::new(1), ProcessingStatus::Pending);
        let repository = Arc::new(InMemoryMediaRepository::new().with_media(media));
        let storage = Arc::new(MockDownloadStorage::new());

        let use_case = DownloadMediaUseCase::new(repository, storage);
        let result = use_case.execute(MediaId::new(1)).await;

        assert!(result.is_err());
        match result.err().unwrap() {
            AppError::BadRequest { message } => assert!(message.contains("not ready")),
            _ => panic!("Expected BadRequest error"),
        }
    }

    #[tokio::test]
    async fn test_download_media_file_not_found_in_storage() {
        let media = create_test_media(MediaId::new(1), ProcessingStatus::Complete);
        let repository = Arc::new(InMemoryMediaRepository::new().with_media(media));
        let storage = Arc::new(MockDownloadStorage::new());

        let use_case = DownloadMediaUseCase::new(repository, storage);
        let result = use_case.execute(MediaId::new(1)).await;

        assert!(result.is_err());
        match result.err().unwrap() {
            AppError::NotFound { resource } => assert!(resource.contains("File content")),
            _ => panic!("Expected NotFound error"),
        }
    }

    #[tokio::test]
    async fn test_download_media_storage_io_error() {
        let media = create_test_media(MediaId::new(1), ProcessingStatus::Complete);
        let repository = Arc::new(InMemoryMediaRepository::new().with_media(media));
        let storage = Arc::new(MockDownloadStorage::new());
        storage.set_error(StorageError::IoError { message: "Disk error".to_string() });

        let use_case = DownloadMediaUseCase::new(repository, storage);
        let result = use_case.execute(MediaId::new(1)).await;

        assert!(result.is_err());
        match result.err().unwrap() {
            AppError::Internal { message } => assert!(message.contains("Storage error")),
            _ => panic!("Expected Internal error"),
        }
    }

    #[tokio::test]
    async fn test_download_stream_success() {
        let test_content = b"streaming content".to_vec();
        let content_hash =
            ContentHash::new("abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890")
                .unwrap();

        let media = create_test_media(MediaId::new(1), ProcessingStatus::Complete);
        let repository = Arc::new(InMemoryMediaRepository::new().with_media(media.clone()));
        let storage = Arc::new(
            MockDownloadStorage::new().with_file(content_hash.as_str(), test_content.clone()),
        );

        let use_case = DownloadMediaUseCase::new(repository, storage);
        let result = use_case.execute_stream(MediaId::new(1)).await;

        assert!(result.is_ok());
        let (mut reader, returned_media) = result.unwrap();
        assert_eq!(returned_media.id, media.id);

        let mut buffer = Vec::new();
        reader.read_to_end(&mut buffer).await.unwrap();
        assert_eq!(buffer, test_content);
    }

    #[tokio::test]
    async fn test_download_stream_not_ready() {
        let media = create_test_media(MediaId::new(1), ProcessingStatus::Pending);
        let repository = Arc::new(InMemoryMediaRepository::new().with_media(media));
        let storage = Arc::new(MockDownloadStorage::new());

        let use_case = DownloadMediaUseCase::new(repository, storage);
        let result = use_case.execute_stream(MediaId::new(1)).await;

        assert!(result.is_err());
        match result.err().unwrap() {
            AppError::BadRequest { message } => assert!(message.contains("not ready")),
            _ => panic!("Expected BadRequest error"),
        }
    }

    #[tokio::test]
    async fn test_download_response_creation() {
        let content = b"test content".to_vec();
        let response = DownloadResponse {
            content: content.clone(),
            content_type: "text/plain".to_string(),
            filename: "test.txt".to_string(),
            file_size: content.len() as u64,
        };

        assert_eq!(response.content, content);
        assert_eq!(response.content_type, "text/plain");
        assert_eq!(response.filename, "test.txt");
        assert_eq!(response.file_size, content.len() as u64);
    }

    #[tokio::test]
    async fn test_use_case_creation() {
        let repository = Arc::new(InMemoryMediaRepository::new());
        let storage = Arc::new(MockDownloadStorage::new());

        let use_case = DownloadMediaUseCase::new(repository.clone(), storage.clone());
        // Test that use case is created successfully - we can't inspect internal fields
        // but we can test that it doesn't panic during creation
        assert!(std::ptr::addr_of!(use_case).is_aligned());
    }

    #[test]
    fn test_media_ready_status_variations() {
        let media_completed = create_test_media(MediaId::new(1), ProcessingStatus::Complete);
        let media_pending = create_test_media(MediaId::new(2), ProcessingStatus::Pending);
        let media_processing = create_test_media(MediaId::new(3), ProcessingStatus::Processing);
        let media_failed = create_test_media(MediaId::new(4), ProcessingStatus::Failed);

        assert!(media_completed.is_ready());
        assert!(!media_pending.is_ready());
        assert!(!media_processing.is_ready());
        assert!(!media_failed.is_ready());
    }
}
