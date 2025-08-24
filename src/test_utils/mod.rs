#[cfg(test)]
pub mod mocks {
    use async_trait::async_trait;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    use crate::domain::{
        entities::{Media, MediaId, UserId},
        repositories::MediaRepository,
        value_objects::ContentHash,
    };
    use crate::presentation::middleware::error::AppError;

    /// Simple in-memory mock repository for testing
    #[derive(Clone, Default)]
    pub struct InMemoryMediaRepository {
        storage: Arc<Mutex<HashMap<MediaId, Media>>>,
        next_id: Arc<Mutex<i64>>,
    }

    impl InMemoryMediaRepository {
        pub fn new() -> Self {
            Self { storage: Arc::new(Mutex::new(HashMap::new())), next_id: Arc::new(Mutex::new(1)) }
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
    }

    #[async_trait]
    impl MediaRepository for InMemoryMediaRepository {
        type Error = AppError;

        async fn save(&self, media: &Media) -> Result<(), Self::Error> {
            let mut storage = self.storage.lock().unwrap();
            let mut next_id = self.next_id.lock().unwrap();

            let mut media_to_save = media.clone();
            if media_to_save.id.as_i64() == 0 {
                media_to_save.id = MediaId::new(*next_id);
                *next_id += 1;
            }

            storage.insert(media_to_save.id, media_to_save);
            Ok(())
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
    }
}
