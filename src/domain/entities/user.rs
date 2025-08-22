use serde::{Deserialize, Serialize};

/// User identifier for tracking media ownership and permissions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UserId(uuid::Uuid);

impl UserId {
    #[must_use]
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4())
    }

    #[must_use]
    pub fn from_uuid(uuid: uuid::Uuid) -> Self {
        Self(uuid)
    }

    #[must_use]
    pub fn as_uuid(&self) -> uuid::Uuid {
        self.0
    }
}

impl Default for UserId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for UserId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_id_creation() {
        let user_id = UserId::new();
        assert!(!user_id.as_uuid().is_nil());
    }

    #[test]
    fn test_user_id_from_uuid() {
        let uuid = uuid::Uuid::new_v4();
        let user_id = UserId::from_uuid(uuid);
        assert_eq!(user_id.as_uuid(), uuid);
    }

    #[test]
    fn test_user_id_default() {
        let user_id = UserId::default();
        assert!(!user_id.as_uuid().is_nil());
    }

    #[test]
    fn test_user_id_equality() {
        let uuid = uuid::Uuid::new_v4();
        let user_id1 = UserId::from_uuid(uuid);
        let user_id2 = UserId::from_uuid(uuid);
        let user_id3 = UserId::new();

        assert_eq!(user_id1, user_id2);
        assert_ne!(user_id1, user_id3);
    }

    #[test]
    fn test_user_id_display() {
        let uuid = uuid::Uuid::new_v4();
        let user_id = UserId::from_uuid(uuid);
        assert_eq!(user_id.to_string(), uuid.to_string());
    }

    #[test]
    fn test_user_id_serialization() {
        let user_id = UserId::new();

        // Test JSON serialization
        let json = serde_json::to_string(&user_id).unwrap();
        let deserialized: UserId = serde_json::from_str(&json).unwrap();
        assert_eq!(user_id, deserialized);
    }

    #[test]
    fn test_user_id_hash() {
        use std::collections::HashMap;

        let user_id1 = UserId::new();
        let user_id2 = UserId::new();

        let mut map = HashMap::new();
        map.insert(user_id1, "user1");
        map.insert(user_id2, "user2");

        assert_eq!(map.len(), 2);
        assert_eq!(map.get(&user_id1), Some(&"user1"));
        assert_eq!(map.get(&user_id2), Some(&"user2"));
    }

    #[test]
    fn test_user_id_clone() {
        let user_id = UserId::new();
        let cloned = user_id;
        assert_eq!(user_id, cloned);
    }

    #[test]
    fn test_user_id_debug() {
        let user_id = UserId::new();
        let debug_str = format!("{user_id:?}");
        assert!(debug_str.contains("UserId"));
    }
}
