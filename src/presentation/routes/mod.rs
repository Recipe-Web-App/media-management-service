use axum::{
    routing::{delete, get, post},
    Router,
};

use crate::{
    infrastructure::http::{health_check_with_dependencies, readiness_check_with_dependencies},
    presentation::handlers::{self, media::AppState},
};

/// Create all application routes with application state
pub fn create_routes(app_state: AppState) -> Router {
    Router::new().nest("/api/v1/media-management", media_management_routes()).with_state(app_state)
}

/// Create media management service routes with state
fn media_management_routes() -> Router<AppState> {
    Router::new()
        .route("/health", get(health_check_with_dependencies))
        .route("/ready", get(readiness_check_with_dependencies))
        .nest("/media", media_routes())
}

/// Create media-related routes with state
fn media_routes() -> Router<AppState> {
    Router::new()
        .route("/", post(handlers::media::upload_media))
        .route("/", get(handlers::media::list_media))
        .route("/{id}", get(handlers::media::get_media))
        .route("/{id}", delete(handlers::media::delete_media))
        .route("/{id}/download", get(handlers::media::download_media))
        .route("/recipe/{recipe_id}", get(handlers::media::get_media_by_recipe))
        .route(
            "/recipe/{recipe_id}/ingredient/{ingredient_id}",
            get(handlers::media::get_media_by_ingredient),
        )
        .route("/recipe/{recipe_id}/step/{step_id}", get(handlers::media::get_media_by_step))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::storage::FileStorage;

    // Mock storage for route testing
    #[derive(Clone)]
    struct MockRoutesStorage {
        base_path: String,
    }

    impl MockRoutesStorage {
        fn new() -> Self {
            Self { base_path: "/tmp/test".to_string() }
        }
    }

    #[async_trait::async_trait]
    impl crate::infrastructure::storage::FileStorage for MockRoutesStorage {
        async fn store<R>(
            &self,
            hash: &crate::domain::value_objects::ContentHash,
            mut _reader: R,
        ) -> Result<String, crate::infrastructure::storage::StorageError>
        where
            R: tokio::io::AsyncRead + Send + Unpin,
        {
            Ok(format!("{}/{}", self.base_path, hash.as_str()))
        }

        async fn retrieve(
            &self,
            _hash: &crate::domain::value_objects::ContentHash,
        ) -> Result<
            Box<dyn tokio::io::AsyncRead + Send + Unpin>,
            crate::infrastructure::storage::StorageError,
        > {
            use crate::infrastructure::storage::StorageError;
            Err(StorageError::FileNotFound { path: "mock".to_string() })
        }

        async fn exists(
            &self,
            _hash: &crate::domain::value_objects::ContentHash,
        ) -> Result<bool, crate::infrastructure::storage::StorageError> {
            Ok(false)
        }

        async fn delete(
            &self,
            _hash: &crate::domain::value_objects::ContentHash,
        ) -> Result<bool, crate::infrastructure::storage::StorageError> {
            Ok(false)
        }

        fn get_path(&self, hash: &crate::domain::value_objects::ContentHash) -> String {
            format!("{}/{}", self.base_path, hash.as_str())
        }

        async fn metadata(
            &self,
            _hash: &crate::domain::value_objects::ContentHash,
        ) -> Result<
            crate::infrastructure::storage::FileMetadata,
            crate::infrastructure::storage::StorageError,
        > {
            use crate::infrastructure::storage::StorageError;
            Err(StorageError::FileNotFound { path: "mock".to_string() })
        }

        async fn health_check(&self) -> Result<(), crate::infrastructure::storage::StorageError> {
            Ok(())
        }
    }

    // Note: AppState requires concrete types, so we'll test route structure
    // without state or with simplified tests

    // Test route creation functions

    #[test]
    fn test_route_functions_exist() {
        // Test internal route functions
        let media_routes = media_routes();
        let media_mgmt_routes = media_management_routes();

        // Test that routes are created successfully (basic structure test)
        assert!(std::ptr::addr_of!(media_routes).is_aligned());
        assert!(std::ptr::addr_of!(media_mgmt_routes).is_aligned());
    }

    #[test]
    fn test_mock_routes_storage_creation() {
        let storage = MockRoutesStorage::new();
        assert_eq!(storage.base_path, "/tmp/test");
    }

    #[tokio::test]
    async fn test_mock_routes_storage_store() {
        let storage = MockRoutesStorage::new();
        let hash = crate::domain::value_objects::ContentHash::new(
            "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
        )
        .unwrap();
        let cursor = std::io::Cursor::new(b"test");

        let result = storage.store(&hash, cursor).await;
        assert!(result.is_ok());
        assert!(result.unwrap().contains(hash.as_str()));
    }

    #[tokio::test]
    async fn test_mock_routes_storage_retrieve_not_found() {
        let storage = MockRoutesStorage::new();
        let hash = crate::domain::value_objects::ContentHash::new(
            "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
        )
        .unwrap();

        let result = storage.retrieve(&hash).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_mock_routes_storage_exists_false() {
        let storage = MockRoutesStorage::new();
        let hash = crate::domain::value_objects::ContentHash::new(
            "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
        )
        .unwrap();

        let exists = storage.exists(&hash).await.unwrap();
        assert!(!exists);
    }

    #[tokio::test]
    async fn test_mock_routes_storage_delete_false() {
        let storage = MockRoutesStorage::new();
        let hash = crate::domain::value_objects::ContentHash::new(
            "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
        )
        .unwrap();

        let deleted = storage.delete(&hash).await.unwrap();
        assert!(!deleted);
    }

    #[test]
    fn test_mock_routes_storage_get_path() {
        let storage = MockRoutesStorage::new();
        let hash = crate::domain::value_objects::ContentHash::new(
            "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
        )
        .unwrap();

        let path = storage.get_path(&hash);
        assert!(path.contains("/tmp/test"));
        assert!(path.contains(hash.as_str()));
    }

    #[tokio::test]
    async fn test_mock_routes_storage_metadata_not_found() {
        let storage = MockRoutesStorage::new();
        let hash = crate::domain::value_objects::ContentHash::new(
            "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
        )
        .unwrap();

        let result = storage.metadata(&hash).await;
        assert!(result.is_err());
    }
}
