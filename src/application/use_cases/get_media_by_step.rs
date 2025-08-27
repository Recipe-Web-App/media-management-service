use std::sync::Arc;

use crate::{
    domain::{
        entities::{MediaId, RecipeId, StepId},
        repositories::MediaRepository,
    },
    presentation::middleware::error::AppError,
};

/// Use case for retrieving media IDs associated with a recipe step
pub struct GetMediaByStepUseCase<R>
where
    R: MediaRepository + ?Sized,
{
    repository: Arc<R>,
}

impl<R> GetMediaByStepUseCase<R>
where
    R: MediaRepository + ?Sized,
{
    /// Create a new get media by step use case
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }

    /// Execute the use case to get media IDs for a recipe step
    pub async fn execute(
        &self,
        recipe_id: RecipeId,
        step_id: StepId,
    ) -> Result<Vec<MediaId>, AppError> {
        tracing::info!("Getting media IDs for recipe: {} step: {}", recipe_id, step_id);

        let media_ids =
            self.repository.find_media_ids_by_recipe_step(recipe_id, step_id).await.map_err(
                |e| AppError::Internal {
                    message: format!("Failed to query media by recipe step: {e}"),
                },
            )?;

        tracing::info!(
            "Found {} media files for recipe: {} step: {}",
            media_ids.len(),
            recipe_id,
            step_id
        );

        Ok(media_ids)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::mocks::InMemoryMediaRepository;

    #[tokio::test]
    async fn test_get_media_by_step_empty() {
        let repo = InMemoryMediaRepository::new();
        let use_case = GetMediaByStepUseCase::new(Arc::new(repo));

        let recipe_id = RecipeId::new(1);
        let step_id = StepId::new(1);
        let result = use_case.execute(recipe_id, step_id).await;

        assert!(result.is_ok());
        let media_ids = result.unwrap();
        assert!(media_ids.is_empty());
    }
}
