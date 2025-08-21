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
}
