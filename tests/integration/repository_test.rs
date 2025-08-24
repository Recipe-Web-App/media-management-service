use media_management_service::domain::{
    entities::{Media, MediaId, UserId},
    repositories::MediaRepository,
    value_objects::{ContentHash, MediaType, ProcessingStatus},
};

mod common;
use common::{builders::MediaBuilder, fixtures::InMemoryMediaRepository};

#[tokio::test]
async fn test_save_and_find_media() {
    let repo = InMemoryMediaRepository::new();
    let media = MediaBuilder::new()
        .with_filename("test.jpg")
        .with_file_size(1024)
        .build();

    repo.save(&media).await.unwrap();

    let found = repo.find_by_id(media.id).await.unwrap();
    assert!(found.is_some());
    let found_media = found.unwrap();
    assert_eq!(found_media.id, media.id);
    assert_eq!(found_media.original_filename, "test.jpg");
    assert_eq!(found_media.file_size, 1024);
}

#[tokio::test]
async fn test_find_by_content_hash() {
    let repo = InMemoryMediaRepository::new();
    let hash = ContentHash::new("abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890").unwrap();
    let media = MediaBuilder::new()
        .with_content_hash(hash.clone())
        .with_filename("test.png")
        .build();

    repo.save(&media).await.unwrap();

    let found = repo.find_by_content_hash(&hash).await.unwrap();
    assert!(found.is_some());
    assert_eq!(found.unwrap().content_hash, hash);
}

#[tokio::test]
async fn test_find_by_user() {
    let repo = InMemoryMediaRepository::new();
    let user_id = UserId::new();

    let media1 = MediaBuilder::new()
        .with_filename("user_file1.jpg")
        .build();
    let media2 = MediaBuilder::new()
        .with_filename("user_file2.png")
        .build();

    repo.save(&media1).await.unwrap();
    repo.save(&media2).await.unwrap();

    let user_media = repo.find_by_user(media1.uploaded_by).await.unwrap();
    assert_eq!(user_media.len(), 1);
    assert_eq!(user_media[0].original_filename, "user_file1.jpg");
}

#[tokio::test]
async fn test_update_media() {
    let repo = InMemoryMediaRepository::new();
    let mut media = MediaBuilder::new()
        .with_filename("original.jpg")
        .build();

    repo.save(&media).await.unwrap();

    media.set_processing_status(ProcessingStatus::Complete);
    repo.update(&media).await.unwrap();

    let updated = repo.find_by_id(media.id).await.unwrap().unwrap();
    assert!(updated.is_ready());
}

#[tokio::test]
async fn test_delete_media() {
    let repo = InMemoryMediaRepository::new();
    let media = MediaBuilder::new().build();

    repo.save(&media).await.unwrap();
    assert!(repo.find_by_id(media.id).await.unwrap().is_some());

    let deleted = repo.delete(media.id).await.unwrap();
    assert!(deleted);
    assert!(repo.find_by_id(media.id).await.unwrap().is_none());
}

#[tokio::test]
async fn test_exists_by_content_hash() {
    let repo = InMemoryMediaRepository::new();
    let hash = ContentHash::new("1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef").unwrap();
    let media = MediaBuilder::new()
        .with_content_hash(hash.clone())
        .build();

    assert!(!repo.exists_by_content_hash(&hash).await.unwrap());

    repo.save(&media).await.unwrap();

    assert!(repo.exists_by_content_hash(&hash).await.unwrap());
}
