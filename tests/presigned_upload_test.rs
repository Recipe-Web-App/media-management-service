/// Integration tests for presigned upload functionality
///
/// These tests focus on the unit-level functionality since full integration
/// requires complex setup with database and storage backends.
/// The existing unit tests in the lib crate provide comprehensive coverage.

#[tokio::test]
async fn test_presigned_upload_dto_serialization() {
    use media_management_service::{
        application::dto::{InitiateUploadRequest, InitiateUploadResponse},
        domain::{entities::MediaId, value_objects::ProcessingStatus},
    };

    // Test InitiateUploadRequest serialization/deserialization
    let request = InitiateUploadRequest {
        filename: "test.jpg".to_string(),
        content_type: "image/jpeg".to_string(),
        file_size: 1024 * 1024,
    };

    let json = serde_json::to_string(&request).unwrap();
    let deserialized: InitiateUploadRequest = serde_json::from_str(&json).unwrap();

    assert_eq!(request.filename, deserialized.filename);
    assert_eq!(request.content_type, deserialized.content_type);
    assert_eq!(request.file_size, deserialized.file_size);

    // Test InitiateUploadResponse serialization/deserialization
    let response = InitiateUploadResponse {
        media_id: MediaId::new(123),
        upload_url: "http://localhost:3000/api/v1/media-management/media/upload/token123?signature=abc&expires=1234567890".to_string(),
        upload_token: "token123".to_string(),
        expires_at: "2024-01-01T12:00:00Z".to_string(),
        status: ProcessingStatus::Pending,
    };

    let json = serde_json::to_string(&response).unwrap();
    let deserialized: InitiateUploadResponse = serde_json::from_str(&json).unwrap();

    assert_eq!(response.media_id, deserialized.media_id);
    assert_eq!(response.upload_url, deserialized.upload_url);
    assert_eq!(response.upload_token, deserialized.upload_token);
    assert_eq!(response.expires_at, deserialized.expires_at);
    assert_eq!(response.status, deserialized.status);
}

#[tokio::test]
async fn test_presigned_upload_service_configuration() {
    use media_management_service::domain::entities::MediaId;
    use media_management_service::infrastructure::storage::presigned_urls::{
        PresignedUrlConfig, PresignedUrlService,
    };
    use std::time::Duration;

    // Test that the presigned URL service can be configured correctly
    let config = PresignedUrlConfig {
        secret_key: "test-secret-key-for-integration".to_string(),
        base_url: "https://api.example.com".to_string(),
        default_expiration: Duration::from_secs(1800), // 30 minutes
        max_file_size: 100 * 1024 * 1024,              // 100MB
    };

    let service = PresignedUrlService::new(config.clone());
    let media_id = MediaId::new(999);

    let result = service.create_upload_session(
        media_id,
        "integration-test.png",
        "image/png",
        5 * 1024 * 1024, // 5MB
    );

    assert!(result.is_ok());
    let session = result.unwrap();

    // Verify session properties
    assert_eq!(session.media_id, media_id);
    assert_eq!(session.expected_content_type, "image/png");
    assert_eq!(session.max_file_size, 5 * 1024 * 1024);
    assert!(session.upload_url.starts_with(&config.base_url));
    assert!(session.upload_url.contains("signature="));
    assert!(session.upload_token.starts_with("upload_"));
}

// Note: Full integration testing with HTTP requests would require setting up
// the complete application state with database connections and storage backends.
// These tests focus on the core functionality that can be tested in isolation.
// The comprehensive unit tests in src/ provide detailed coverage of all components.
