use media_management_service::domain::{
    entities::{Media, MediaId, UserId},
    value_objects::{ContentHash, MediaType, ProcessingStatus}
};
use std::time::SystemTime;

pub struct MediaBuilder {
    id: Option<MediaId>,
    content_hash: Option<ContentHash>,
    original_filename: Option<String>,
    media_type: Option<MediaType>,
    file_size: Option<u64>,
    processing_status: Option<ProcessingStatus>,
    uploaded_by: Option<UserId>,
    uploaded_at: Option<SystemTime>,
    updated_at: Option<SystemTime>,
}

impl MediaBuilder {
    pub fn new() -> Self {
        Self {
            id: None,
            content_hash: None,
            original_filename: None,
            media_type: None,
            file_size: None,
            processing_status: None,
            uploaded_by: None,
            uploaded_at: None,
            updated_at: None,
        }
    }

    pub fn with_id(mut self, id: MediaId) -> Self {
        self.id = Some(id);
        self
    }

    pub fn with_content_hash(mut self, hash: ContentHash) -> Self {
        self.content_hash = Some(hash);
        self
    }

    pub fn with_filename(mut self, filename: &str) -> Self {
        self.original_filename = Some(filename.to_string());
        self
    }

    pub fn with_media_type(mut self, media_type: MediaType) -> Self {
        self.media_type = Some(media_type);
        self
    }

    pub fn with_file_size(mut self, size: u64) -> Self {
        self.file_size = Some(size);
        self
    }

    pub fn build(self) -> Media {
        let content_hash = self.content_hash.unwrap_or_else(||
            ContentHash::new("abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890").unwrap()
        );
        let original_filename = self.original_filename.unwrap_or_else(|| "test.jpg".to_string());
        let media_type = self.media_type.unwrap_or(MediaType::Image);
        let file_size = self.file_size.unwrap_or(1024);
        let uploaded_by = self.uploaded_by.unwrap_or_else(UserId::new);

        Media::new(content_hash, original_filename, media_type, file_size, uploaded_by)
    }
}

impl Default for MediaBuilder {
    fn default() -> Self {
        Self::new()
    }
}
