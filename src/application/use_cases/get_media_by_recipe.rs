use std::sync::Arc;

use crate::{
    domain::{
        entities::{MediaId, RecipeId},
        repositories::MediaRepository,
    },
    presentation::middleware::error::AppError,
};

/// Use case for retrieving media IDs associated with a recipe
pub struct GetMediaByRecipeUseCase<R>
where
    R: MediaRepository,
{
    repository: Arc<R>,
}

impl<R> GetMediaByRecipeUseCase<R>
where
    R: MediaRepository,
{
    /// Create a new get media by recipe use case
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }

    /// Execute the use case to get media IDs for a recipe
    pub async fn execute(&self, recipe_id: RecipeId) -> Result<Vec<MediaId>, AppError> {
        tracing::info!("Getting media IDs for recipe: {}", recipe_id);

        let media_ids = self.repository.find_media_ids_by_recipe(recipe_id).await.map_err(|e| {
            AppError::Internal { message: format!("Failed to query media by recipe: {e}") }
        })?;

        tracing::info!("Found {} media files for recipe: {}", media_ids.len(), recipe_id);

        Ok(media_ids)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::mocks::InMemoryMediaRepository;

    #[tokio::test]
    async fn test_get_media_by_recipe_empty() {
        let repo = InMemoryMediaRepository::new();
        let use_case = GetMediaByRecipeUseCase::new(Arc::new(repo));

        let recipe_id = RecipeId::new(1);
        let result = use_case.execute(recipe_id).await;

        assert!(result.is_ok());
        let media_ids = result.unwrap();
        assert!(media_ids.is_empty());
    }
}
