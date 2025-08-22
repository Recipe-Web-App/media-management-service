use serde::{Deserialize, Serialize};
use std::fmt;

/// Content-addressable hash for file identification and integrity verification
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ContentHash(String);

impl ContentHash {
    /// Create a new content hash from a SHA-256 hex string
    ///
    /// # Errors
    /// Returns an error if the hash is not a valid 64-character hex string
    pub fn new(hash: &str) -> Result<Self, ContentHashError> {
        if hash.len() != 64 {
            return Err(ContentHashError::InvalidLength(hash.len()));
        }

        if !hash.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(ContentHashError::InvalidCharacters);
        }

        Ok(Self(hash.to_lowercase()))
    }

    /// Get the hash as a string slice
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Get the hash as an owned string
    #[must_use]
    pub fn into_string(self) -> String {
        self.0
    }

    /// Get the first 6 characters for directory structure
    #[must_use]
    pub fn prefix(&self) -> &str {
        &self.0[0..6]
    }

    /// Build nested directory path components (ab/cd/ef)
    #[must_use]
    pub fn path_components(&self) -> (String, String, String) {
        (self.0[0..2].to_string(), self.0[2..4].to_string(), self.0[4..6].to_string())
    }
}

impl fmt::Display for ContentHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::str::FromStr for ContentHash {
    type Err = ContentHashError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

/// Errors that can occur when creating a content hash
#[derive(Debug, thiserror::Error)]
pub enum ContentHashError {
    #[error("Invalid hash length: expected 64 characters, got {0}")]
    InvalidLength(usize),
    #[error("Invalid characters: hash must contain only hexadecimal characters")]
    InvalidCharacters,
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn test_valid_hash() {
        let hash_str = "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890";
        let hash = ContentHash::new(hash_str).unwrap();
        assert_eq!(hash.as_str(), hash_str);
    }

    #[test]
    fn test_invalid_length() {
        let hash_str = "abcdef123";
        let result = ContentHash::new(hash_str);
        assert!(matches!(result, Err(ContentHashError::InvalidLength(9))));
    }

    #[test]
    fn test_invalid_characters() {
        let hash_str = "abcdef1234567890abcdef1234567890abcdef1234567890abcdef123456789z";
        let result = ContentHash::new(hash_str);
        assert!(matches!(result, Err(ContentHashError::InvalidCharacters)));
    }

    #[test]
    fn test_path_components() {
        let hash_str = "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890";
        let hash = ContentHash::new(hash_str).unwrap();
        let (p1, p2, p3) = hash.path_components();
        assert_eq!(p1, "ab");
        assert_eq!(p2, "cd");
        assert_eq!(p3, "ef");
    }

    #[test]
    fn test_case_insensitive() {
        let lowercase = "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890";
        let uppercase = "ABCDEF1234567890ABCDEF1234567890ABCDEF1234567890ABCDEF1234567890";

        let hash_lower = ContentHash::new(lowercase).unwrap();
        let hash_upper = ContentHash::new(uppercase).unwrap();

        assert_eq!(hash_lower.as_str(), lowercase);
        assert_eq!(hash_upper.as_str(), lowercase); // Should be converted to lowercase
    }

    #[test]
    fn test_display_trait() {
        let hash_str = "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";
        let hash = ContentHash::new(hash_str).unwrap();
        assert_eq!(format!("{hash}"), hash_str);
    }

    #[test]
    fn test_from_str_trait() {
        let hash_str = "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";
        let hash: ContentHash = hash_str.parse().unwrap();
        assert_eq!(hash.as_str(), hash_str);
    }

    #[test]
    fn test_serialization() {
        let hash_str = "fedcba0987654321fedcba0987654321fedcba0987654321fedcba0987654321";
        let hash = ContentHash::new(hash_str).unwrap();

        let json = serde_json::to_string(&hash).unwrap();
        let deserialized: ContentHash = serde_json::from_str(&json).unwrap();

        assert_eq!(hash, deserialized);
    }

    #[test]
    fn test_prefix() {
        let hash_str = "123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0";
        let hash = ContentHash::new(hash_str).unwrap();
        assert_eq!(hash.prefix(), "123456");
    }

    #[test]
    fn test_into_string() {
        let hash_str = "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789";
        let hash = ContentHash::new(hash_str).unwrap();
        let owned_string = hash.into_string();
        assert_eq!(owned_string, hash_str);
    }

    // Property-based tests using proptest
    proptest! {
        #[test]
        fn test_valid_64_char_hex_strings(
            s in prop::string::string_regex("[0-9a-fA-F]{64}").unwrap()
        ) {
            let result = ContentHash::new(&s);
            prop_assert!(result.is_ok());

            let hash = result.unwrap();
            prop_assert_eq!(hash.as_str().len(), 64);
            prop_assert!(hash.as_str().chars().all(|c| c.is_ascii_hexdigit()));
        }

        #[test]
        fn test_invalid_length_strings(
            s in prop::string::string_regex("[0-9a-fA-F]{1,63}|[0-9a-fA-F]{65,128}").unwrap()
        ) {
            let result = ContentHash::new(&s);
            if s.len() != 64 {
                prop_assert!(result.is_err());
                prop_assert!(matches!(result, Err(ContentHashError::InvalidLength(_))));
            }
        }

        #[test]
        fn test_invalid_characters_property(
            // Generate strings with non-hex characters
            invalid_char in "[g-zG-Z@#$%^&*()_+]",
            prefix in prop::string::string_regex("[0-9a-fA-F]{0,63}").unwrap()
        ) {
            let mut s = prefix;
            s.push_str(&invalid_char.repeat(64 - s.len().min(63)));

            if s.len() == 64 && s.contains(|c: char| !c.is_ascii_hexdigit()) {
                let result = ContentHash::new(&s);
                prop_assert!(result.is_err());
                prop_assert!(matches!(result, Err(ContentHashError::InvalidCharacters)));
            }
        }

        #[test]
        fn test_path_components_property(
            s in prop::string::string_regex("[0-9a-fA-F]{64}").unwrap()
        ) {
            let hash = ContentHash::new(&s).unwrap();
            let (p1, p2, p3) = hash.path_components();

            prop_assert_eq!(p1.len(), 2);
            prop_assert_eq!(p2.len(), 2);
            prop_assert_eq!(p3.len(), 2);
            prop_assert_eq!(format!("{}{}{}", p1, p2, p3), &s.to_lowercase()[0..6]);
        }

        #[test]
        fn test_case_normalization(
            s in prop::string::string_regex("[0-9a-fA-F]{64}").unwrap()
        ) {
            let hash = ContentHash::new(&s).unwrap();
            prop_assert_eq!(hash.as_str(), s.to_lowercase());

            // Test that uppercase input is normalized to lowercase
            let upper_s = s.to_uppercase();
            let hash_upper = ContentHash::new(&upper_s).unwrap();
            prop_assert_eq!(hash_upper.as_str(), s.to_lowercase());
        }
    }

    #[test]
    fn test_error_display() {
        let invalid_length_error = ContentHashError::InvalidLength(32);
        assert_eq!(
            format!("{invalid_length_error}"),
            "Invalid hash length: expected 64 characters, got 32"
        );

        let invalid_chars_error = ContentHashError::InvalidCharacters;
        assert_eq!(
            format!("{invalid_chars_error}"),
            "Invalid characters: hash must contain only hexadecimal characters"
        );
    }
}
