use std::sync::Arc;

use crate::{
    domain::{
        entities::{IngredientId, MediaId, RecipeId},
        repositories::MediaRepository,
    },
    presentation::middleware::error::AppError,
};

/// Use case for retrieving media IDs associated with a recipe ingredient
pub struct GetMediaByIngredientUseCase<R>
where
    R: MediaRepository + ?Sized,
{
    repository: Arc<R>,
}

impl<R> GetMediaByIngredientUseCase<R>
where
    R: MediaRepository + ?Sized,
{
    /// Create a new get media by ingredient use case
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }

    /// Execute the use case to get media IDs for a recipe ingredient
    pub async fn execute(
        &self,
        recipe_id: RecipeId,
        ingredient_id: IngredientId,
    ) -> Result<Vec<MediaId>, AppError> {
        tracing::info!("Getting media IDs for recipe: {} ingredient: {}", recipe_id, ingredient_id);

        let media_ids = self
            .repository
            .find_media_ids_by_recipe_ingredient(recipe_id, ingredient_id)
            .await
            .map_err(|e| AppError::Internal {
                message: format!("Failed to query media by recipe ingredient: {e}"),
            })?;

        tracing::info!(
            "Found {} media files for recipe: {} ingredient: {}",
            media_ids.len(),
            recipe_id,
            ingredient_id
        );

        Ok(media_ids)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::mocks::InMemoryMediaRepository;

    #[tokio::test]
    async fn test_get_media_by_ingredient_empty() {
        let repo = InMemoryMediaRepository::new();
        let use_case = GetMediaByIngredientUseCase::new(Arc::new(repo));

        let recipe_id = RecipeId::new(1);
        let ingredient_id = IngredientId::new(1);
        let result = use_case.execute(recipe_id, ingredient_id).await;

        assert!(result.is_ok());
        let media_ids = result.unwrap();
        assert!(media_ids.is_empty());
    }
}
