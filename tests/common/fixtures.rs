use media_management_service::domain::{
    entities::{Media, MediaId, UserId},
    repositories::MediaRepository,
    value_objects::ContentHash,
};
use mockall::mock;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

mock! {
    pub MediaRepo {}

    #[async_trait::async_trait]
    impl MediaRepository for MediaRepo {
        type Error = MockRepositoryError;

        async fn save(&self, media: &Media) -> Result<MediaId, Self::Error>;
        async fn find_by_id(&self, id: MediaId) -> Result<Option<Media>, Self::Error>;
        async fn find_by_content_hash(&self, hash: &ContentHash) -> Result<Option<Media>, Self::Error>;
        async fn find_by_user(&self, user_id: UserId) -> Result<Vec<Media>, Self::Error>;
        async fn update(&self, media: &Media) -> Result<(), Self::Error>;
        async fn delete(&self, id: MediaId) -> Result<bool, Self::Error>;
        async fn exists_by_content_hash(&self, hash: &ContentHash) -> Result<bool, Self::Error>;
    }
}

#[derive(Debug, thiserror::Error)]
pub enum MockRepositoryError {
    #[error("Database connection failed")]
    ConnectionFailed,
    #[error("Media not found")]
    NotFound,
    #[error("Constraint violation: {0}")]
    ConstraintViolation(String),
}

pub struct InMemoryMediaRepository {
    storage: Arc<Mutex<HashMap<MediaId, Media>>>,
}

impl InMemoryMediaRepository {
    pub fn new() -> Self {
        Self {
            storage: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn with_media(media: Vec<Media>) -> Self {
        let storage = media
            .into_iter()
            .map(|m| (m.id, m))
            .collect::<HashMap<_, _>>();

        Self {
            storage: Arc::new(Mutex::new(storage)),
        }
    }
}

impl Default for InMemoryMediaRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl MediaRepository for InMemoryMediaRepository {
    type Error = MockRepositoryError;

    async fn save(&self, media: &Media) -> Result<MediaId, Self::Error> {
        let mut storage = self.storage.lock().unwrap();
        let media_id = media.id;
        storage.insert(media.id, media.clone());
        Ok(media_id)
    }

    async fn find_by_id(&self, id: MediaId) -> Result<Option<Media>, Self::Error> {
        let storage = self.storage.lock().unwrap();
        Ok(storage.get(&id).cloned())
    }

    async fn find_by_content_hash(&self, hash: &ContentHash) -> Result<Option<Media>, Self::Error> {
        let storage = self.storage.lock().unwrap();
        Ok(storage
            .values()
            .find(|media| media.content_hash == *hash)
            .cloned())
    }

    async fn find_by_user(&self, user_id: UserId) -> Result<Vec<Media>, Self::Error> {
        let storage = self.storage.lock().unwrap();
        Ok(storage
            .values()
            .filter(|media| media.uploaded_by == user_id)
            .cloned()
            .collect())
    }

    async fn update(&self, media: &Media) -> Result<(), Self::Error> {
        let mut storage = self.storage.lock().unwrap();
        if storage.contains_key(&media.id) {
            storage.insert(media.id, media.clone());
            Ok(())
        } else {
            Err(MockRepositoryError::NotFound)
        }
    }

    async fn delete(&self, id: MediaId) -> Result<bool, Self::Error> {
        let mut storage = self.storage.lock().unwrap();
        Ok(storage.remove(&id).is_some())
    }

    async fn exists_by_content_hash(&self, hash: &ContentHash) -> Result<bool, Self::Error> {
        let storage = self.storage.lock().unwrap();
        Ok(storage.values().any(|media| media.content_hash == *hash))
    }
}
