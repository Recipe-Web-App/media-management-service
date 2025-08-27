use crate::domain::entities::{IngredientId, Media, MediaId, RecipeId, StepId, UserId};
use crate::domain::value_objects::{ContentHash, ProcessingStatus};
use async_trait::async_trait;

/// Repository trait for media persistence
#[async_trait]
pub trait MediaRepository: Send + Sync {
    type Error: std::error::Error + Send + Sync + 'static;

    /// Save a media entity
    async fn save(&self, media: &Media) -> Result<(), Self::Error>;

    /// Find media by ID
    async fn find_by_id(&self, id: MediaId) -> Result<Option<Media>, Self::Error>;

    /// Find media by content hash
    async fn find_by_content_hash(&self, hash: &ContentHash) -> Result<Option<Media>, Self::Error>;

    /// Find all media uploaded by a specific user
    async fn find_by_user(&self, user_id: UserId) -> Result<Vec<Media>, Self::Error>;

    /// Find media by user with cursor-based pagination
    /// Returns a tuple of (`media_list`, `next_cursor`, `has_more`)
    async fn find_by_user_paginated(
        &self,
        user_id: UserId,
        cursor: Option<String>,
        limit: u32,
        status_filter: Option<ProcessingStatus>,
    ) -> Result<(Vec<Media>, Option<String>, bool), Self::Error>;

    /// Update media entity
    async fn update(&self, media: &Media) -> Result<(), Self::Error>;

    /// Delete media by ID
    async fn delete(&self, id: MediaId) -> Result<bool, Self::Error>;

    /// Check if media exists by content hash
    async fn exists_by_content_hash(&self, hash: &ContentHash) -> Result<bool, Self::Error>;

    /// Find media IDs associated with a recipe
    async fn find_media_ids_by_recipe(
        &self,
        recipe_id: RecipeId,
    ) -> Result<Vec<MediaId>, Self::Error>;

    /// Find media IDs associated with a recipe ingredient
    async fn find_media_ids_by_recipe_ingredient(
        &self,
        recipe_id: RecipeId,
        ingredient_id: IngredientId,
    ) -> Result<Vec<MediaId>, Self::Error>;

    /// Find media IDs associated with a recipe step
    async fn find_media_ids_by_recipe_step(
        &self,
        recipe_id: RecipeId,
        step_id: StepId,
    ) -> Result<Vec<MediaId>, Self::Error>;

    /// Health check for repository connectivity
    ///
    /// Performs a simple check to verify repository is accessible and responsive.
    /// Returns `Ok(())` if repository is accessible, `Err(Self::Error)` otherwise.
    async fn health_check(&self) -> Result<(), Self::Error>;
}

// Mock implementation moved to test utilities
// This avoids complex generic type issues with mockall
