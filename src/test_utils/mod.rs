#[cfg(test)]
pub mod mocks {
    use async_trait::async_trait;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    use crate::domain::{
        entities::{IngredientId, Media, MediaId, RecipeId, StepId, UserId},
        repositories::MediaRepository,
        value_objects::{ContentHash, ProcessingStatus},
    };
    use crate::presentation::middleware::error::AppError;

    /// Type alias for recipe ingredient media mapping
    type RecipeIngredientMediaMap = HashMap<(RecipeId, IngredientId), Vec<MediaId>>;

    /// Type alias for recipe step media mapping
    type RecipeStepMediaMap = HashMap<(RecipeId, StepId), Vec<MediaId>>;

    /// Simple in-memory mock repository for testing
    #[derive(Clone, Default)]
    pub struct InMemoryMediaRepository {
        storage: Arc<Mutex<HashMap<MediaId, Media>>>,
        next_id: Arc<Mutex<i64>>,
        recipe_media: Arc<Mutex<HashMap<RecipeId, Vec<MediaId>>>>,
        recipe_ingredient_media: Arc<Mutex<RecipeIngredientMediaMap>>,
        recipe_step_media: Arc<Mutex<RecipeStepMediaMap>>,
    }

    impl InMemoryMediaRepository {
        pub fn new() -> Self {
            Self {
                storage: Arc::new(Mutex::new(HashMap::new())),
                next_id: Arc::new(Mutex::new(1)),
                recipe_media: Arc::new(Mutex::new(HashMap::new())),
                recipe_ingredient_media: Arc::new(Mutex::new(HashMap::new())),
                recipe_step_media: Arc::new(Mutex::new(HashMap::new())),
            }
        }

        /// # Panics
        /// Panics if the internal mutex is poisoned
        #[must_use]
        pub fn with_media(self, media: Media) -> Self {
            {
                let mut storage = self.storage.lock().unwrap();
                storage.insert(media.id, media);
            }
            self
        }

        /// Set up recipe ingredient media associations for testing
        /// # Panics
        /// Panics if the internal mutex is poisoned
        #[must_use]
        pub fn with_recipe_ingredient_media(
            self,
            recipe_id: RecipeId,
            ingredient_id: IngredientId,
            media_ids: Vec<MediaId>,
        ) -> Self {
            {
                let mut ingredient_media = self.recipe_ingredient_media.lock().unwrap();
                ingredient_media.insert((recipe_id, ingredient_id), media_ids);
            }
            self
        }

        /// Set up recipe media associations for testing
        /// # Panics
        /// Panics if the internal mutex is poisoned
        #[must_use]
        pub fn with_recipe_media(self, recipe_id: RecipeId, media_ids: Vec<MediaId>) -> Self {
            {
                let mut recipe_media = self.recipe_media.lock().unwrap();
                recipe_media.insert(recipe_id, media_ids);
            }
            self
        }

        /// Set up recipe step media associations for testing
        /// # Panics
        /// Panics if the internal mutex is poisoned
        #[must_use]
        pub fn with_recipe_step_media(
            self,
            recipe_id: RecipeId,
            step_id: StepId,
            media_ids: Vec<MediaId>,
        ) -> Self {
            {
                let mut step_media = self.recipe_step_media.lock().unwrap();
                step_media.insert((recipe_id, step_id), media_ids);
            }
            self
        }
    }

    #[async_trait]
    impl MediaRepository for InMemoryMediaRepository {
        type Error = AppError;

        async fn save(&self, media: &Media) -> Result<MediaId, Self::Error> {
            let mut storage = self.storage.lock().unwrap();
            let mut next_id = self.next_id.lock().unwrap();

            let mut media_to_save = media.clone();
            if media_to_save.id.as_i64() == 0 {
                media_to_save.id = MediaId::new(*next_id);
                *next_id += 1;
            }

            let assigned_id = media_to_save.id;
            storage.insert(media_to_save.id, media_to_save);
            Ok(assigned_id)
        }

        async fn find_by_id(&self, id: MediaId) -> Result<Option<Media>, Self::Error> {
            let storage = self.storage.lock().unwrap();
            Ok(storage.get(&id).cloned())
        }

        async fn find_by_content_hash(
            &self,
            hash: &ContentHash,
        ) -> Result<Option<Media>, Self::Error> {
            let storage = self.storage.lock().unwrap();
            Ok(storage.values().find(|m| &m.content_hash == hash).cloned())
        }

        async fn find_by_user(&self, user_id: UserId) -> Result<Vec<Media>, Self::Error> {
            let storage = self.storage.lock().unwrap();
            let media: Vec<Media> =
                storage.values().filter(|m| m.uploaded_by == user_id).cloned().collect();
            Ok(media)
        }

        async fn find_by_user_paginated(
            &self,
            user_id: UserId,
            cursor: Option<String>,
            limit: u32,
            status_filter: Option<ProcessingStatus>,
        ) -> Result<(Vec<Media>, Option<String>, bool), Self::Error> {
            let storage = self.storage.lock().unwrap();

            // Filter by user and optional status
            let mut media: Vec<Media> = storage
                .values()
                .filter(|m| m.uploaded_by == user_id)
                .filter(|m| {
                    if let Some(ref status) = status_filter {
                        &m.processing_status == status
                    } else {
                        true
                    }
                })
                .cloned()
                .collect();

            // Sort by ID for consistent pagination
            media.sort_by_key(|m| m.id.as_i64());

            // Apply cursor-based filtering
            let start_index = if let Some(cursor_str) = cursor {
                // Decode cursor (simple base64 of media_id)
                use base64::{engine::general_purpose::STANDARD, Engine as _};
                if let Ok(decoded) = STANDARD.decode(&cursor_str) {
                    if let Ok(cursor_data) = String::from_utf8(decoded) {
                        if let Ok(cursor_id) = cursor_data.parse::<i64>() {
                            // Find the first media with ID greater than cursor
                            media
                                .iter()
                                .position(|m| m.id.as_i64() > cursor_id)
                                .unwrap_or(media.len())
                        } else {
                            0
                        }
                    } else {
                        0
                    }
                } else {
                    0
                }
            } else {
                0
            };

            // Take the page slice
            let limit = limit.clamp(1, 100) as usize;
            let end_index = (start_index + limit).min(media.len());
            let has_more = end_index < media.len();

            let page_media = media[start_index..end_index].to_vec();

            // Generate next cursor if there are more items
            let next_cursor = if has_more && !page_media.is_empty() {
                use base64::{engine::general_purpose::STANDARD, Engine as _};
                let last_media_id = page_media.last().unwrap().id.as_i64();
                Some(STANDARD.encode(last_media_id.to_string().as_bytes()))
            } else {
                None
            };

            Ok((page_media, next_cursor, has_more))
        }

        async fn update(&self, media: &Media) -> Result<(), Self::Error> {
            let mut storage = self.storage.lock().unwrap();
            storage.insert(media.id, media.clone());
            Ok(())
        }

        async fn delete(&self, id: MediaId) -> Result<bool, Self::Error> {
            let mut storage = self.storage.lock().unwrap();
            Ok(storage.remove(&id).is_some())
        }

        async fn exists_by_content_hash(&self, hash: &ContentHash) -> Result<bool, Self::Error> {
            let storage = self.storage.lock().unwrap();
            Ok(storage.values().any(|m| &m.content_hash == hash))
        }

        async fn find_media_ids_by_recipe(
            &self,
            recipe_id: RecipeId,
        ) -> Result<Vec<MediaId>, Self::Error> {
            let recipe_media = self.recipe_media.lock().unwrap();
            Ok(recipe_media.get(&recipe_id).cloned().unwrap_or_default())
        }

        async fn find_media_ids_by_recipe_ingredient(
            &self,
            recipe_id: RecipeId,
            ingredient_id: IngredientId,
        ) -> Result<Vec<MediaId>, Self::Error> {
            let ingredient_media = self.recipe_ingredient_media.lock().unwrap();
            Ok(ingredient_media.get(&(recipe_id, ingredient_id)).cloned().unwrap_or_default())
        }

        async fn find_media_ids_by_recipe_step(
            &self,
            recipe_id: RecipeId,
            step_id: StepId,
        ) -> Result<Vec<MediaId>, Self::Error> {
            let step_media = self.recipe_step_media.lock().unwrap();
            Ok(step_media.get(&(recipe_id, step_id)).cloned().unwrap_or_default())
        }

        async fn health_check(&self) -> Result<(), Self::Error> {
            // In-memory repository is always healthy
            Ok(())
        }
    }
}
