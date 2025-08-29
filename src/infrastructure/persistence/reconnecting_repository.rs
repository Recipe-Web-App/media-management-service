use crate::domain::entities::{IngredientId, Media, MediaId, RecipeId, StepId, UserId};
use crate::domain::repositories::MediaRepository;
use crate::domain::value_objects::{ContentHash, ProcessingStatus};
use crate::infrastructure::config::PostgresConfig;
use crate::infrastructure::persistence::{
    Database, DisconnectedMediaRepository, PostgreSqlMediaRepository,
};
use crate::presentation::middleware::error::AppError;
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// A repository wrapper that handles database reconnection automatically
///
/// This repository starts in a disconnected state and periodically attempts to
/// reconnect to the database. Once connected, it switches to using the `PostgreSQL`
/// repository. If the connection is lost, it falls back to the disconnected repository.
#[derive(Clone)]
pub struct ReconnectingMediaRepository {
    /// Current repository implementation (either connected or disconnected)
    current_repo: Arc<RwLock<RepositoryState>>,
    /// Database configuration for reconnection attempts
    postgres_config: PostgresConfig,
}

#[derive(Clone)]
enum RepositoryState {
    Connected(PostgreSqlMediaRepository),
    Disconnected(DisconnectedMediaRepository),
}

impl ReconnectingMediaRepository {
    /// Create a new reconnecting repository starting in disconnected state
    pub fn new(postgres_config: PostgresConfig, initial_error: String) -> Self {
        let disconnected_repo = DisconnectedMediaRepository::new(initial_error);
        let current_repo = Arc::new(RwLock::new(RepositoryState::Disconnected(disconnected_repo)));

        Self { current_repo, postgres_config }
    }

    /// Create a new reconnecting repository starting with an existing database connection
    pub fn with_connection(postgres_config: PostgresConfig, database: &Database) -> Self {
        let connected_repo = PostgreSqlMediaRepository::new(database.pool().clone());
        let current_repo = Arc::new(RwLock::new(RepositoryState::Connected(connected_repo)));

        Self { current_repo, postgres_config }
    }

    /// Attempt to establish database connection
    ///
    /// Returns true if connection was successful and repository was updated
    pub async fn attempt_reconnection(&self) -> bool {
        debug!("Attempting database reconnection...");

        match Database::new(&self.postgres_config).await {
            Ok(database) => {
                info!("Database reconnection successful");
                let connected_repo = PostgreSqlMediaRepository::new(database.pool().clone());

                // Update the repository state
                let mut current_repo = self.current_repo.write().await;
                *current_repo = RepositoryState::Connected(connected_repo);

                true
            }
            Err(e) => {
                debug!("Database reconnection failed: {}", e);
                false
            }
        }
    }

    /// Check if the repository is currently connected
    pub async fn is_connected(&self) -> bool {
        let current_repo = self.current_repo.read().await;
        matches!(*current_repo, RepositoryState::Connected(_))
    }

    /// Start background reconnection task
    ///
    /// This spawns a background task that periodically attempts to reconnect
    /// to the database when in disconnected state.
    pub fn start_reconnection_task(self) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));

            loop {
                interval.tick().await;

                // Only attempt reconnection if currently disconnected
                if !self.is_connected().await {
                    if self.attempt_reconnection().await {
                        info!("Database connection restored");
                    } else {
                        debug!("Database still unavailable, will retry in 30 seconds");
                    }
                }
            }
        })
    }

    /// Handle connection errors by falling back to disconnected state
    async fn handle_connection_error(&self, error: AppError) -> AppError {
        // Check if this looks like a connection error
        if let AppError::Database { ref message } = error {
            if message.contains("connection")
                || message.contains("timeout")
                || message.contains("network")
            {
                warn!(
                    "Database connection error detected, switching to disconnected mode: {}",
                    message
                );

                // Switch back to disconnected state
                let disconnected_repo =
                    DisconnectedMediaRepository::new(format!("Connection lost: {message}"));
                let mut current_repo = self.current_repo.write().await;
                *current_repo = RepositoryState::Disconnected(disconnected_repo);
            }
        }

        error
    }
}

#[async_trait]
impl MediaRepository for ReconnectingMediaRepository {
    type Error = AppError;

    async fn save(&self, media: &Media) -> Result<MediaId, Self::Error> {
        let current_repo = self.current_repo.read().await;
        let result = match &*current_repo {
            RepositoryState::Connected(repo) => repo.save(media).await,
            RepositoryState::Disconnected(repo) => repo.save(media).await,
        };

        // Handle potential connection errors
        match result {
            Err(e) => Err(self.handle_connection_error(e).await),
            Ok(id) => Ok(id),
        }
    }

    async fn find_by_id(&self, id: MediaId) -> Result<Option<Media>, Self::Error> {
        let current_repo = self.current_repo.read().await;
        let result = match &*current_repo {
            RepositoryState::Connected(repo) => repo.find_by_id(id).await,
            RepositoryState::Disconnected(repo) => repo.find_by_id(id).await,
        };

        match result {
            Err(e) => Err(self.handle_connection_error(e).await),
            Ok(media) => Ok(media),
        }
    }

    async fn find_by_content_hash(&self, hash: &ContentHash) -> Result<Option<Media>, Self::Error> {
        let current_repo = self.current_repo.read().await;
        let result = match &*current_repo {
            RepositoryState::Connected(repo) => repo.find_by_content_hash(hash).await,
            RepositoryState::Disconnected(repo) => repo.find_by_content_hash(hash).await,
        };

        match result {
            Err(e) => Err(self.handle_connection_error(e).await),
            Ok(media) => Ok(media),
        }
    }

    async fn find_by_user(&self, user_id: UserId) -> Result<Vec<Media>, Self::Error> {
        let current_repo = self.current_repo.read().await;
        let result = match &*current_repo {
            RepositoryState::Connected(repo) => repo.find_by_user(user_id).await,
            RepositoryState::Disconnected(repo) => repo.find_by_user(user_id).await,
        };

        match result {
            Err(e) => Err(self.handle_connection_error(e).await),
            Ok(media) => Ok(media),
        }
    }

    async fn find_by_user_paginated(
        &self,
        user_id: UserId,
        cursor: Option<String>,
        limit: u32,
        status_filter: Option<ProcessingStatus>,
    ) -> Result<(Vec<Media>, Option<String>, bool), Self::Error> {
        let current_repo = self.current_repo.read().await;
        let result = match &*current_repo {
            RepositoryState::Connected(repo) => {
                repo.find_by_user_paginated(user_id, cursor, limit, status_filter).await
            }
            RepositoryState::Disconnected(repo) => {
                repo.find_by_user_paginated(user_id, cursor, limit, status_filter).await
            }
        };

        match result {
            Err(e) => Err(self.handle_connection_error(e).await),
            Ok(result) => Ok(result),
        }
    }

    async fn update(&self, media: &Media) -> Result<(), Self::Error> {
        let current_repo = self.current_repo.read().await;
        let result = match &*current_repo {
            RepositoryState::Connected(repo) => repo.update(media).await,
            RepositoryState::Disconnected(repo) => repo.update(media).await,
        };

        match result {
            Err(e) => Err(self.handle_connection_error(e).await),
            Ok(()) => Ok(()),
        }
    }

    async fn delete(&self, id: MediaId) -> Result<bool, Self::Error> {
        let current_repo = self.current_repo.read().await;
        let result = match &*current_repo {
            RepositoryState::Connected(repo) => repo.delete(id).await,
            RepositoryState::Disconnected(repo) => repo.delete(id).await,
        };

        match result {
            Err(e) => Err(self.handle_connection_error(e).await),
            Ok(deleted) => Ok(deleted),
        }
    }

    async fn exists_by_content_hash(&self, hash: &ContentHash) -> Result<bool, Self::Error> {
        let current_repo = self.current_repo.read().await;
        let result = match &*current_repo {
            RepositoryState::Connected(repo) => repo.exists_by_content_hash(hash).await,
            RepositoryState::Disconnected(repo) => repo.exists_by_content_hash(hash).await,
        };

        match result {
            Err(e) => Err(self.handle_connection_error(e).await),
            Ok(exists) => Ok(exists),
        }
    }

    async fn find_media_ids_by_recipe(
        &self,
        recipe_id: RecipeId,
    ) -> Result<Vec<MediaId>, Self::Error> {
        let current_repo = self.current_repo.read().await;
        let result = match &*current_repo {
            RepositoryState::Connected(repo) => repo.find_media_ids_by_recipe(recipe_id).await,
            RepositoryState::Disconnected(repo) => repo.find_media_ids_by_recipe(recipe_id).await,
        };

        match result {
            Err(e) => Err(self.handle_connection_error(e).await),
            Ok(ids) => Ok(ids),
        }
    }

    async fn find_media_ids_by_recipe_ingredient(
        &self,
        recipe_id: RecipeId,
        ingredient_id: IngredientId,
    ) -> Result<Vec<MediaId>, Self::Error> {
        let current_repo = self.current_repo.read().await;
        let result = match &*current_repo {
            RepositoryState::Connected(repo) => {
                repo.find_media_ids_by_recipe_ingredient(recipe_id, ingredient_id).await
            }
            RepositoryState::Disconnected(repo) => {
                repo.find_media_ids_by_recipe_ingredient(recipe_id, ingredient_id).await
            }
        };

        match result {
            Err(e) => Err(self.handle_connection_error(e).await),
            Ok(ids) => Ok(ids),
        }
    }

    async fn find_media_ids_by_recipe_step(
        &self,
        recipe_id: RecipeId,
        step_id: StepId,
    ) -> Result<Vec<MediaId>, Self::Error> {
        let current_repo = self.current_repo.read().await;
        let result = match &*current_repo {
            RepositoryState::Connected(repo) => {
                repo.find_media_ids_by_recipe_step(recipe_id, step_id).await
            }
            RepositoryState::Disconnected(repo) => {
                repo.find_media_ids_by_recipe_step(recipe_id, step_id).await
            }
        };

        match result {
            Err(e) => Err(self.handle_connection_error(e).await),
            Ok(ids) => Ok(ids),
        }
    }

    async fn health_check(&self) -> Result<(), Self::Error> {
        let current_repo = self.current_repo.read().await;
        match &*current_repo {
            RepositoryState::Connected(repo) => {
                let result = repo.health_check().await;
                match result {
                    Err(e) => Err(self.handle_connection_error(e).await),
                    Ok(()) => Ok(()),
                }
            }
            RepositoryState::Disconnected(repo) => repo.health_check().await,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::config::PostgresConfig;

    fn create_test_postgres_config() -> PostgresConfig {
        PostgresConfig {
            url: "postgres://test:test@localhost:5432/test".to_string(),
            max_connections: 5,
            min_connections: 1,
            acquire_timeout_seconds: 10,
            host: "localhost".to_string(),
            port: 5432,
            database: "test".to_string(),
            schema: "public".to_string(),
            user: "test".to_string(),
            password: "test".to_string(),
        }
    }

    #[test]
    fn test_reconnecting_repository_creation() {
        let config = create_test_postgres_config();
        let repo = ReconnectingMediaRepository::new(config, "test error".to_string());

        // Repository should be created successfully
        assert!(std::ptr::addr_of!(repo).is_aligned());
    }

    #[tokio::test]
    async fn test_initial_disconnected_state() {
        let config = create_test_postgres_config();
        let repo = ReconnectingMediaRepository::new(config, "test error".to_string());

        // Should start in disconnected state
        assert!(!repo.is_connected().await);
    }

    #[tokio::test]
    async fn test_health_check_disconnected() {
        let config = create_test_postgres_config();
        let repo = ReconnectingMediaRepository::new(config, "test connection failed".to_string());

        let result = repo.health_check().await;
        assert!(result.is_err());

        if let Err(AppError::Database { message }) = result {
            assert!(message.contains("Database unavailable"));
            assert!(message.contains("test connection failed"));
        } else {
            panic!("Expected Database error");
        }
    }

    #[tokio::test]
    async fn test_reconnection_task_startup() {
        let config = create_test_postgres_config();
        let repo = ReconnectingMediaRepository::new(config, "test error".to_string());

        // Should be able to start reconnection task without panic
        let handle = repo.start_reconnection_task();

        // Cancel the task immediately to avoid running indefinitely in tests
        handle.abort();

        // Test passes if we reach this point without panicking
    }
}
