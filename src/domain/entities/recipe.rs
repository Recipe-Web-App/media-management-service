use serde::{Deserialize, Serialize};

/// Unique identifier for recipes (database BIGSERIAL)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RecipeId(i64);

impl RecipeId {
    #[must_use]
    pub fn new(id: i64) -> Self {
        Self(id)
    }

    #[must_use]
    pub fn as_i64(&self) -> i64 {
        self.0
    }
}

impl std::fmt::Display for RecipeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<i64> for RecipeId {
    fn from(id: i64) -> Self {
        Self(id)
    }
}

/// Unique identifier for ingredients (database BIGSERIAL)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct IngredientId(i64);

impl IngredientId {
    #[must_use]
    pub fn new(id: i64) -> Self {
        Self(id)
    }

    #[must_use]
    pub fn as_i64(&self) -> i64 {
        self.0
    }
}

impl std::fmt::Display for IngredientId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<i64> for IngredientId {
    fn from(id: i64) -> Self {
        Self(id)
    }
}

/// Unique identifier for recipe steps (database BIGSERIAL)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StepId(i64);

impl StepId {
    #[must_use]
    pub fn new(id: i64) -> Self {
        Self(id)
    }

    #[must_use]
    pub fn as_i64(&self) -> i64 {
        self.0
    }
}

impl std::fmt::Display for StepId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<i64> for StepId {
    fn from(id: i64) -> Self {
        Self(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recipe_id_operations() {
        let id1 = RecipeId::new(1);
        let id2 = RecipeId::new(2);

        assert_ne!(id1, id2);
        assert_eq!(id1.as_i64(), 1);
        assert_eq!(id2.as_i64(), 2);

        let id_from_i64 = RecipeId::from(42);
        assert_eq!(id_from_i64.as_i64(), 42);

        let id_string = id1.to_string();
        assert_eq!(id_string, "1");
    }

    #[test]
    fn test_ingredient_id_operations() {
        let id1 = IngredientId::new(10);
        let id2 = IngredientId::new(20);

        assert_ne!(id1, id2);
        assert_eq!(id1.as_i64(), 10);
        assert_eq!(id2.as_i64(), 20);

        let id_from_i64 = IngredientId::from(99);
        assert_eq!(id_from_i64.as_i64(), 99);

        let id_string = id1.to_string();
        assert_eq!(id_string, "10");
    }

    #[test]
    fn test_step_id_operations() {
        let id1 = StepId::new(100);
        let id2 = StepId::new(200);

        assert_ne!(id1, id2);
        assert_eq!(id1.as_i64(), 100);
        assert_eq!(id2.as_i64(), 200);

        let id_from_i64 = StepId::from(555);
        assert_eq!(id_from_i64.as_i64(), 555);

        let id_string = id1.to_string();
        assert_eq!(id_string, "100");
    }
}
