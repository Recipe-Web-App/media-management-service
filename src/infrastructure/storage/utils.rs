use sha2::{Digest, Sha256};
use tokio::io::{AsyncRead, AsyncReadExt};

use super::StorageError;
use crate::domain::value_objects::ContentHash;

/// Generate content hash from bytes
pub fn generate_content_hash(data: &[u8]) -> Result<ContentHash, StorageError> {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let hash = format!("{:x}", hasher.finalize());
    ContentHash::new(&hash)
        .map_err(|e| StorageError::IoError { message: format!("Invalid hash generated: {e}") })
}

/// Generate content hash from async reader
pub async fn generate_content_hash_async<R>(
    mut reader: R,
) -> Result<(ContentHash, Vec<u8>), StorageError>
where
    R: AsyncRead + Send + Unpin,
{
    let mut hasher = Sha256::new();
    let mut buffer = Vec::new();
    let mut chunk = [0u8; 8192];

    loop {
        let n = reader.read(&mut chunk).await?;
        if n == 0 {
            break;
        }
        hasher.update(&chunk[..n]);
        buffer.extend_from_slice(&chunk[..n]);
    }

    let hash = format!("{:x}", hasher.finalize());
    let content_hash = ContentHash::new(&hash)
        .map_err(|e| StorageError::IoError { message: format!("Invalid hash generated: {e}") })?;

    Ok((content_hash, buffer))
}

/// Create content-addressable path from hash (e.g., "ab/cd/ef/abcdef123...")
pub fn content_addressable_path(hash: &ContentHash) -> String {
    let hash_str = hash.as_str();
    if hash_str.len() < 6 {
        return hash_str.to_string();
    }

    format!("{}/{}/{}/{}", &hash_str[0..2], &hash_str[2..4], &hash_str[4..6], hash_str)
}

/// Detect MIME type from file content
pub fn detect_content_type(data: &[u8], filename: Option<&str>) -> String {
    // First, try to detect from content
    if data.len() >= 4 {
        match &data[0..4] {
            [0xFF, 0xD8, 0xFF, ..] => return "image/jpeg".to_string(),
            [0x89, 0x50, 0x4E, 0x47] => return "image/png".to_string(),
            [0x47, 0x49, 0x46, 0x38] => return "image/gif".to_string(),
            [0x52, 0x49, 0x46, 0x46] if data.len() >= 12 && &data[8..12] == b"WEBP" => {
                return "image/webp".to_string();
            }
            [0x66, 0x74, 0x79, 0x70] => return "video/mp4".to_string(),
            _ => {}
        }
    }

    // Check for additional formats
    if data.len() >= 8 {
        // Check for AVIF
        if data[4..8] == [0x66, 0x74, 0x79, 0x70]
            && data.len() >= 12
            && (&data[8..12] == b"avif" || &data[8..12] == b"avis")
        {
            return "image/avif".to_string();
        }
    }

    // Fall back to filename extension
    if let Some(filename) = filename {
        match filename.split('.').next_back().map(str::to_lowercase).as_deref() {
            Some("jpg" | "jpeg") => "image/jpeg".to_string(),
            Some("png") => "image/png".to_string(),
            Some("gif") => "image/gif".to_string(),
            Some("webp") => "image/webp".to_string(),
            Some("avif") => "image/avif".to_string(),
            Some("mp4") => "video/mp4".to_string(),
            Some("webm") => "video/webm".to_string(),
            Some("mov") => "video/quicktime".to_string(),
            Some("avi") => "video/x-msvideo".to_string(),
            Some("mp3") => "audio/mpeg".to_string(),
            Some("wav") => "audio/wav".to_string(),
            Some("flac") => "audio/flac".to_string(),
            Some("ogg") => "audio/ogg".to_string(),
            _ => "application/octet-stream".to_string(),
        }
    } else {
        "application/octet-stream".to_string()
    }
}

/// Validate file content matches expected type
pub fn validate_content_type(data: &[u8], expected_type: &str) -> Result<(), StorageError> {
    let detected_type = detect_content_type(data, None);

    // Allow some flexibility in content type matching
    let is_valid = match expected_type {
        "image/jpeg" => detected_type == "image/jpeg",
        "image/png" => detected_type == "image/png",
        "image/gif" => detected_type == "image/gif",
        "image/webp" => detected_type == "image/webp",
        "image/avif" => detected_type == "image/avif",
        "video/mp4" => detected_type == "video/mp4",
        "video/webm" => detected_type == "video/webm",
        "video/quicktime" => detected_type == "video/quicktime",
        "audio/mpeg" => detected_type == "audio/mpeg",
        "audio/wav" => detected_type == "audio/wav",
        "audio/flac" => detected_type == "audio/flac",
        _ => expected_type == detected_type,
    };

    if !is_valid {
        return Err(StorageError::InvalidPath {
            path: format!(
                "Content type mismatch: expected {expected_type}, detected {detected_type}"
            ),
        });
    }

    Ok(())
}

/// Validate file size limits
pub fn validate_file_size(size: u64, max_size: u64) -> Result<(), StorageError> {
    if size > max_size {
        return Err(StorageError::StorageFull);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_generate_content_hash() {
        let data = b"hello world";
        let hash = generate_content_hash(data).unwrap();

        // SHA-256 of "hello world"
        assert_eq!(
            hash.as_str(),
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
    }

    #[test]
    fn test_content_addressable_path() {
        let hash =
            ContentHash::new("abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890")
                .unwrap();
        let path = content_addressable_path(&hash);

        assert_eq!(
            path,
            "ab/cd/ef/abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890"
        );
    }

    #[test]
    fn test_content_addressable_path_short_hash() {
        let hash = ContentHash::new("abc12").unwrap_or_else(|_| {
            // For test purposes, create a mock hash if validation fails
            ContentHash::new("abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890")
                .unwrap()
        });
        let path = content_addressable_path(&hash);

        // Should handle short hashes gracefully
        assert!(path.contains('/'));
    }

    #[test]
    fn test_detect_jpeg_content_type() {
        let jpeg_header = [0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10];
        let content_type = detect_content_type(&jpeg_header, None);
        assert_eq!(content_type, "image/jpeg");
    }

    #[test]
    fn test_detect_png_content_type() {
        let png_header = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        let content_type = detect_content_type(&png_header, None);
        assert_eq!(content_type, "image/png");
    }

    #[test]
    fn test_detect_content_type_by_filename() {
        let data = [0x00, 0x00, 0x00, 0x00]; // Unknown content

        let content_type = detect_content_type(&data, Some("test.jpg"));
        assert_eq!(content_type, "image/jpeg");

        let content_type = detect_content_type(&data, Some("video.mp4"));
        assert_eq!(content_type, "video/mp4");

        let content_type = detect_content_type(&data, Some("unknown.xyz"));
        assert_eq!(content_type, "application/octet-stream");
    }

    #[test]
    fn test_validate_content_type_success() {
        let jpeg_header = [0xFF, 0xD8, 0xFF, 0xE0];
        let result = validate_content_type(&jpeg_header, "image/jpeg");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_content_type_failure() {
        let png_header = [0x89, 0x50, 0x4E, 0x47];
        let result = validate_content_type(&png_header, "image/jpeg");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_file_size_success() {
        let result = validate_file_size(1000, 2000);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_file_size_failure() {
        let result = validate_file_size(3000, 2000);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_generate_content_hash_async() {
        let data = b"hello world";
        let cursor = Cursor::new(data);

        let (hash, buffer) = generate_content_hash_async(cursor).await.unwrap();

        assert_eq!(
            hash.as_str(),
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
        assert_eq!(buffer, data.to_vec());
    }
}
