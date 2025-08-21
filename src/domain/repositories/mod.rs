use crate::domain::entities::{Media, MediaId, UserId};
use crate::domain::value_objects::ContentHash;
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

    /// Update media entity
    async fn update(&self, media: &Media) -> Result<(), Self::Error>;

    /// Delete media by ID
    async fn delete(&self, id: MediaId) -> Result<bool, Self::Error>;

    /// Check if media exists by content hash
    async fn exists_by_content_hash(&self, hash: &ContentHash) -> Result<bool, Self::Error>;
}
