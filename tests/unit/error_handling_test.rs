use media_management_service::domain::value_objects::{
    ContentHash, ContentHashError, ProcessingStatus
};
use media_management_service::infrastructure::config::{
    AppConfig, DatabaseConfig, ServerConfig
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_content_hash_error_types() {
        // Test InvalidLength error
        let short_hash = "abc123";
        let result = ContentHash::new(short_hash);
        assert!(result.is_err());

        match result.unwrap_err() {
            ContentHashError::InvalidLength(len) => {
                assert_eq!(len, 6);
                assert_eq!(
                    format!("{}", ContentHashError::InvalidLength(len)),
                    "Invalid hash length: expected 64 characters, got 6"
                );
            }
            _ => panic!("Expected InvalidLength error"),
        }

        // Test InvalidCharacters error
        let invalid_hash = "abcdef1234567890abcdef1234567890abcdef1234567890abcdef123456789z";
        let result = ContentHash::new(invalid_hash);
        assert!(result.is_err());

        match result.unwrap_err() {
            ContentHashError::InvalidCharacters => {
                assert_eq!(
                    format!("{}", ContentHashError::InvalidCharacters),
                    "Invalid characters: hash must contain only hexadecimal characters"
                );
            }
            _ => panic!("Expected InvalidCharacters error"),
        }
    }

    #[test]
    fn test_processing_status_error_cases() {
        // Test various failed status messages
        let error_messages = vec![
            "File corrupted",
            "Unsupported format",
            "Processing timeout",
            "Insufficient memory",
            "",  // Empty error message
        ];

        for msg in error_messages {
            let status = ProcessingStatus::Failed(msg.to_string());
            assert!(status.is_failed());
            assert!(!status.is_complete());
            assert!(!status.is_processing());
            assert!(!status.is_pending());
            assert_eq!(status.error_message(), Some(msg));
        }
    }

    #[test]
    #[should_panic(expected = "Invalid host/port configuration")]
    fn test_server_config_invalid_host() {
        let config = ServerConfig {
            host: "".to_string(), // Invalid empty host
            port: 8080,
            max_upload_size: 1000,
        };
        let _ = config.socket_addr(); // Should panic
    }

    #[test]
    #[should_panic(expected = "Invalid host/port configuration")]
    fn test_server_config_invalid_port() {
        let config = ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 0, // Invalid port
            max_upload_size: 1000,
        };
        let _ = config.socket_addr(); // Should panic
    }

    #[test]
    fn test_database_config_edge_cases() {
        // Test with empty components
        let config = DatabaseConfig {
            url: String::new(),
            max_connections: 1,
            min_connections: 1,
            acquire_timeout_seconds: 1,
            host: String::new(),
            port: 5432,
            database: String::new(),
            schema: String::new(),
            user: String::new(),
            password: String::new(),
        };

        // Should still generate URL, even with empty components
        let url = config.connection_url();
        assert!(url.starts_with("postgres://"));
        assert!(url.contains("@:5432/"));
    }

    #[test]
    fn test_processing_status_display_edge_cases() {
        // Test display with very long error message
        let long_error = "x".repeat(1000);
        let status = ProcessingStatus::Failed(long_error.clone());
        let display_str = format!("{}", status);
        assert!(display_str.starts_with("failed: "));
        assert!(display_str.contains(&long_error));

        // Test display with special characters
        let special_error = "Error: file \"test.jpg\" not found! @#$%^&*()";
        let status = ProcessingStatus::Failed(special_error.to_string());
        let display_str = format!("{}", status);
        assert_eq!(display_str, format!("failed: {}", special_error));
    }

    #[test]
    fn test_content_hash_boundary_conditions() {
        // Test exactly 64 characters
        let valid_hash = "a".repeat(64);
        assert!(ContentHash::new(&valid_hash).is_ok());

        // Test 63 characters (one short)
        let short_hash = "a".repeat(63);
        let result = ContentHash::new(&short_hash);
        assert!(matches!(result, Err(ContentHashError::InvalidLength(63))));

        // Test 65 characters (one too many)
        let long_hash = "a".repeat(65);
        let result = ContentHash::new(&long_hash);
        assert!(matches!(result, Err(ContentHashError::InvalidLength(65))));

        // Test with mixed case and numbers at boundaries
        let boundary_hash = "0123456789ABCDEFabcdef0123456789ABCDEFabcdef0123456789ABCDEFab";
        assert!(ContentHash::new(&boundary_hash).is_ok());
    }

    #[test]
    fn test_error_trait_implementations() {
        let error = ContentHashError::InvalidLength(32);

        // Test that error implements std::error::Error
        let _: &dyn std::error::Error = &error;

        // Test error sources (should be None for these simple errors)
        assert!(error.source().is_none());

        let chars_error = ContentHashError::InvalidCharacters;
        assert!(chars_error.source().is_none());
    }

    #[test]
    fn test_config_validation_edge_cases() {
        // Test configuration with extreme values
        let config = DatabaseConfig {
            url: "postgres://user:pass@host:5432/db".to_string(),
            max_connections: u32::MAX,
            min_connections: 0,
            acquire_timeout_seconds: u64::MAX,
            host: "localhost".to_string(),
            port: 65535, // Maximum valid port
            database: "db".to_string(),
            schema: "schema".to_string(),
            user: "user".to_string(),
            password: "pass".to_string(),
        };

        // Should handle extreme values gracefully
        assert!(!config.connection_url().is_empty());
        assert_eq!(config.max_connections, u32::MAX);
        assert_eq!(config.port, 65535);
    }

    #[test]
    fn test_unicode_and_special_characters_in_errors() {
        // Test content hash with Unicode characters (should fail)
        let unicode_hash = "café".repeat(16); // 64 chars but with non-ASCII
        let result = ContentHash::new(&unicode_hash);
        assert!(matches!(result, Err(ContentHashError::InvalidCharacters)));

        // Test processing status with Unicode error message
        let unicode_error = "Файл не найден"; // "File not found" in Russian
        let status = ProcessingStatus::Failed(unicode_error.to_string());
        assert_eq!(status.error_message(), Some(unicode_error));
        assert!(format!("{}", status).contains(unicode_error));
    }
}
