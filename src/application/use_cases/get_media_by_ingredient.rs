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

    #[tokio::test]
    async fn test_get_media_by_ingredient_zero_ids() {
        let repo = InMemoryMediaRepository::new();
        let use_case = GetMediaByIngredientUseCase::new(Arc::new(repo));

        // Test with zero recipe ID
        let recipe_id = RecipeId::new(0);
        let ingredient_id = IngredientId::new(1);
        let result = use_case.execute(recipe_id, ingredient_id).await;

        assert!(result.is_ok());
        let media_ids = result.unwrap();
        assert!(media_ids.is_empty());
    }

    #[tokio::test]
    async fn test_get_media_by_ingredient_zero_ingredient_id() {
        let repo = InMemoryMediaRepository::new();
        let use_case = GetMediaByIngredientUseCase::new(Arc::new(repo));

        // Test with zero ingredient ID
        let recipe_id = RecipeId::new(1);
        let ingredient_id = IngredientId::new(0);
        let result = use_case.execute(recipe_id, ingredient_id).await;

        assert!(result.is_ok());
        let media_ids = result.unwrap();
        assert!(media_ids.is_empty());
    }

    #[tokio::test]
    async fn test_get_media_by_ingredient_large_ids() {
        let repo = InMemoryMediaRepository::new();
        let use_case = GetMediaByIngredientUseCase::new(Arc::new(repo));

        // Test with large IDs
        let recipe_id = RecipeId::new(999_999_999);
        let ingredient_id = IngredientId::new(888_888_888);
        let result = use_case.execute(recipe_id, ingredient_id).await;

        assert!(result.is_ok());
        let media_ids = result.unwrap();
        assert!(media_ids.is_empty());
    }

    #[tokio::test]
    async fn test_get_media_by_ingredient_repository_error() {
        // Create a mock repository that always returns an error
        use crate::domain::entities::*;
        use crate::domain::repositories::MediaRepository;
        use async_trait::async_trait;

        struct ErrorMediaRepository;

        #[async_trait]
        impl MediaRepository for ErrorMediaRepository {
            type Error = AppError;

            async fn save(
                &self,
                _media: &crate::domain::entities::Media,
            ) -> Result<(), Self::Error> {
                Err(AppError::Internal { message: "Database error".to_string() })
            }

            async fn find_by_id(
                &self,
                _id: MediaId,
            ) -> Result<Option<crate::domain::entities::Media>, Self::Error> {
                Err(AppError::Internal { message: "Database error".to_string() })
            }

            async fn find_by_content_hash(
                &self,
                _hash: &crate::domain::value_objects::ContentHash,
            ) -> Result<Option<crate::domain::entities::Media>, Self::Error> {
                Err(AppError::Internal { message: "Database error".to_string() })
            }

            async fn find_by_user(
                &self,
                _user_id: UserId,
            ) -> Result<Vec<crate::domain::entities::Media>, Self::Error> {
                Err(AppError::Internal { message: "Database error".to_string() })
            }

            async fn update(
                &self,
                _media: &crate::domain::entities::Media,
            ) -> Result<(), Self::Error> {
                Err(AppError::Internal { message: "Database error".to_string() })
            }

            async fn delete(&self, _id: MediaId) -> Result<bool, Self::Error> {
                Err(AppError::Internal { message: "Database error".to_string() })
            }

            async fn exists_by_content_hash(
                &self,
                _hash: &crate::domain::value_objects::ContentHash,
            ) -> Result<bool, Self::Error> {
                Err(AppError::Internal { message: "Database error".to_string() })
            }

            async fn find_media_ids_by_recipe(
                &self,
                _recipe_id: RecipeId,
            ) -> Result<Vec<MediaId>, Self::Error> {
                Err(AppError::Internal { message: "Database error".to_string() })
            }

            async fn find_media_ids_by_recipe_ingredient(
                &self,
                _recipe_id: RecipeId,
                _ingredient_id: IngredientId,
            ) -> Result<Vec<MediaId>, Self::Error> {
                Err(AppError::Internal { message: "Database connection failed".to_string() })
            }

            async fn find_media_ids_by_recipe_step(
                &self,
                _recipe_id: RecipeId,
                _step_id: StepId,
            ) -> Result<Vec<MediaId>, Self::Error> {
                Err(AppError::Internal { message: "Database error".to_string() })
            }

            async fn health_check(&self) -> Result<(), Self::Error> {
                Err(AppError::Internal { message: "Database error".to_string() })
            }
        }

        let repo = ErrorMediaRepository;
        let use_case = GetMediaByIngredientUseCase::new(Arc::new(repo));

        let recipe_id = RecipeId::new(1);
        let ingredient_id = IngredientId::new(1);
        let result = use_case.execute(recipe_id, ingredient_id).await;

        assert!(result.is_err());
        let error = result.unwrap_err();
        match error {
            AppError::Internal { message } => {
                assert!(message.contains("Failed to query media by recipe ingredient"));
                assert!(message.contains("Database connection failed"));
            }
            _ => panic!("Expected Internal error, got: {error:?}"),
        }
    }

    #[tokio::test]
    async fn test_get_media_by_ingredient_with_data() {
        use crate::domain::entities::*;
        use crate::domain::value_objects::*;

        // Create repository with some media data
        let media1 = crate::domain::entities::Media::with_id(
            MediaId::new(1),
            ContentHash::new("abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890")
                .unwrap(),
            "test1.jpg".to_string(),
            MediaType::new("image/jpeg"),
            "/test/path1".to_string(),
            1024,
            ProcessingStatus::Complete,
        )
        .uploaded_by(UserId::new())
        .build();

        let media2 = crate::domain::entities::Media::with_id(
            MediaId::new(2),
            ContentHash::new("1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef")
                .unwrap(),
            "test2.jpg".to_string(),
            MediaType::new("image/jpeg"),
            "/test/path2".to_string(),
            2048,
            ProcessingStatus::Complete,
        )
        .uploaded_by(UserId::new())
        .build();

        let repo = InMemoryMediaRepository::new()
            .with_media(media1)
            .with_media(media2)
            .with_recipe_ingredient_media(
                RecipeId::new(1),
                IngredientId::new(1),
                vec![MediaId::new(1), MediaId::new(2)],
            );

        let use_case = GetMediaByIngredientUseCase::new(Arc::new(repo));

        let recipe_id = RecipeId::new(1);
        let ingredient_id = IngredientId::new(1);
        let result = use_case.execute(recipe_id, ingredient_id).await;

        assert!(result.is_ok());
        let media_ids = result.unwrap();
        assert_eq!(media_ids.len(), 2);
        assert!(media_ids.contains(&MediaId::new(1)));
        assert!(media_ids.contains(&MediaId::new(2)));
    }

    #[tokio::test]
    async fn test_get_media_by_ingredient_different_ingredients() {
        use crate::domain::entities::*;
        use crate::domain::value_objects::*;

        // Create repository with media for different ingredients
        let media1 = crate::domain::entities::Media::with_id(
            MediaId::new(1),
            ContentHash::new("abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890")
                .unwrap(),
            "test1.jpg".to_string(),
            MediaType::new("image/jpeg"),
            "/test/path1".to_string(),
            1024,
            ProcessingStatus::Complete,
        )
        .uploaded_by(UserId::new())
        .build();

        let repo = InMemoryMediaRepository::new()
            .with_media(media1)
            .with_recipe_ingredient_media(
                RecipeId::new(1),
                IngredientId::new(1),
                vec![MediaId::new(1)],
            )
            .with_recipe_ingredient_media(RecipeId::new(1), IngredientId::new(2), vec![]); // Different ingredient, no media

        let use_case = GetMediaByIngredientUseCase::new(Arc::new(repo));

        // Test ingredient 1 (has media)
        let result1 = use_case.execute(RecipeId::new(1), IngredientId::new(1)).await;
        assert!(result1.is_ok());
        let media_ids1 = result1.unwrap();
        assert_eq!(media_ids1.len(), 1);
        assert!(media_ids1.contains(&MediaId::new(1)));

        // Test ingredient 2 (no media)
        let result2 = use_case.execute(RecipeId::new(1), IngredientId::new(2)).await;
        assert!(result2.is_ok());
        let media_ids2 = result2.unwrap();
        assert!(media_ids2.is_empty());
    }

    #[tokio::test]
    async fn test_get_media_by_ingredient_logging() {
        // This test verifies that logging calls don't cause panics
        let repo = InMemoryMediaRepository::new();
        let use_case = GetMediaByIngredientUseCase::new(Arc::new(repo));

        let recipe_id = RecipeId::new(42);
        let ingredient_id = IngredientId::new(123);
        let result = use_case.execute(recipe_id, ingredient_id).await;

        // The test primarily ensures that tracing calls in the use case don't panic
        // and that the operation completes successfully
        assert!(result.is_ok());
    }
}
