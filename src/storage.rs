use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use tokio::fs;

use crate::error::AppError;
use crate::models::ContentHash;

#[derive(Debug, Clone)]
pub struct Storage {
    base_path: PathBuf,
    temp_path: PathBuf,
}

impl Storage {
    pub async fn new(base_path: &str, temp_path: &str) -> Result<Self, AppError> {
        let base_path = PathBuf::from(base_path);
        let temp_path = PathBuf::from(temp_path);
        fs::create_dir_all(&base_path).await?;
        fs::create_dir_all(&temp_path).await?;
        Ok(Self {
            base_path,
            temp_path,
        })
    }

    pub fn full_path(&self, hash: &ContentHash) -> PathBuf {
        self.base_path.join(hash.cas_path())
    }

    pub async fn store(&self, hash: &ContentHash, bytes: &[u8]) -> Result<String, AppError> {
        let final_path = self.full_path(hash);

        if fs::try_exists(&final_path).await.unwrap_or(false) {
            return Ok(hash.cas_path());
        }

        if let Some(parent) = final_path.parent() {
            fs::create_dir_all(parent).await?;
        }

        let temp_file = self.temp_path.join(format!("{}.tmp", hash.as_str()));
        fs::write(&temp_file, bytes).await?;
        fs::rename(&temp_file, &final_path).await?;

        Ok(hash.cas_path())
    }

    pub async fn retrieve(&self, hash: &ContentHash) -> Result<fs::File, AppError> {
        let path = self.full_path(hash);
        fs::File::open(&path).await.map_err(|e| {
            if e.kind() == ErrorKind::NotFound {
                AppError::NotFound("media")
            } else {
                AppError::Internal(e.to_string())
            }
        })
    }

    pub async fn delete(&self, hash: &ContentHash) -> Result<bool, AppError> {
        let path = self.full_path(hash);

        match fs::remove_file(&path).await {
            Ok(()) => {}
            Err(e) if e.kind() == ErrorKind::NotFound => return Ok(false),
            Err(e) => return Err(AppError::Internal(e.to_string())),
        }

        self.cleanup_empty_dirs(&path).await;
        Ok(true)
    }

    pub async fn exists(&self, hash: &ContentHash) -> bool {
        fs::try_exists(self.full_path(hash)).await.unwrap_or(false)
    }

    pub async fn health_check(&self) -> Result<(), AppError> {
        let probe = self.temp_path.join(".health_check_probe");
        fs::write(&probe, b"ok")
            .await
            .map_err(|e| AppError::ServiceUnavailable(format!("storage write failed: {e}")))?;
        fs::remove_file(&probe)
            .await
            .map_err(|e| AppError::ServiceUnavailable(format!("storage delete failed: {e}")))?;
        Ok(())
    }

    async fn cleanup_empty_dirs(&self, file_path: &Path) {
        let mut dir = file_path.parent();
        for _ in 0..3 {
            let Some(d) = dir else { break };
            if d == self.base_path {
                break;
            }
            match fs::remove_dir(d).await {
                Ok(()) => {}
                Err(_) => break,
            }
            dir = d.parent();
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use tokio::io::AsyncReadExt;

    async fn make_storage(dir: &TempDir) -> Storage {
        let base = dir.path().join("media");
        let temp = dir.path().join("temp");
        Storage::new(base.to_str().unwrap(), temp.to_str().unwrap())
            .await
            .unwrap()
    }

    fn test_hash() -> ContentHash {
        ContentHash::new(&"a".repeat(64)).unwrap()
    }

    #[tokio::test]
    async fn new_creates_directories() {
        let dir = TempDir::new().unwrap();
        let base = dir.path().join("media");
        let temp = dir.path().join("temp");
        Storage::new(base.to_str().unwrap(), temp.to_str().unwrap())
            .await
            .unwrap();
        assert!(base.is_dir());
        assert!(temp.is_dir());
    }

    #[tokio::test]
    async fn new_is_idempotent() {
        let dir = TempDir::new().unwrap();
        let base = dir.path().join("media");
        let temp = dir.path().join("temp");
        let b = base.to_str().unwrap();
        let t = temp.to_str().unwrap();
        Storage::new(b, t).await.unwrap();
        Storage::new(b, t).await.unwrap();
    }

    #[tokio::test]
    async fn full_path_joins_base_and_cas_path() {
        let dir = TempDir::new().unwrap();
        let storage = make_storage(&dir).await;
        let hash = test_hash();
        let path = storage.full_path(&hash);
        assert!(path.starts_with(&storage.base_path));
        assert!(path.ends_with(hash.cas_path().as_str()));
    }

    #[tokio::test]
    async fn store_writes_correct_content() {
        let dir = TempDir::new().unwrap();
        let storage = make_storage(&dir).await;
        let hash = test_hash();
        storage.store(&hash, b"hello").await.unwrap();
        let content = std::fs::read(storage.full_path(&hash)).unwrap();
        assert_eq!(content, b"hello");
    }

    #[tokio::test]
    async fn store_returns_cas_path() {
        let dir = TempDir::new().unwrap();
        let storage = make_storage(&dir).await;
        let hash = test_hash();
        let path = storage.store(&hash, b"data").await.unwrap();
        assert_eq!(path, hash.cas_path());
    }

    #[tokio::test]
    async fn store_creates_intermediate_dirs() {
        let dir = TempDir::new().unwrap();
        let storage = make_storage(&dir).await;
        let hash = test_hash();
        storage.store(&hash, b"data").await.unwrap();
        let parent = storage.full_path(&hash).parent().unwrap().to_path_buf();
        assert!(parent.is_dir());
    }

    #[tokio::test]
    async fn store_is_idempotent() {
        let dir = TempDir::new().unwrap();
        let storage = make_storage(&dir).await;
        let hash = test_hash();
        let p1 = storage.store(&hash, b"data").await.unwrap();
        let p2 = storage.store(&hash, b"data").await.unwrap();
        assert_eq!(p1, p2);
    }

    #[tokio::test]
    async fn retrieve_returns_stored_content() {
        let dir = TempDir::new().unwrap();
        let storage = make_storage(&dir).await;
        let hash = test_hash();
        storage.store(&hash, b"payload").await.unwrap();
        let mut file = storage.retrieve(&hash).await.unwrap();
        let mut buf = Vec::new();
        file.read_to_end(&mut buf).await.unwrap();
        assert_eq!(buf, b"payload");
    }

    #[tokio::test]
    async fn retrieve_not_found_for_missing() {
        let dir = TempDir::new().unwrap();
        let storage = make_storage(&dir).await;
        let result = storage.retrieve(&test_hash()).await;
        assert!(matches!(result, Err(AppError::NotFound("media"))));
    }

    #[tokio::test]
    async fn exists_true_after_store() {
        let dir = TempDir::new().unwrap();
        let storage = make_storage(&dir).await;
        let hash = test_hash();
        storage.store(&hash, b"data").await.unwrap();
        assert!(storage.exists(&hash).await);
    }

    #[tokio::test]
    async fn exists_false_for_missing() {
        let dir = TempDir::new().unwrap();
        let storage = make_storage(&dir).await;
        assert!(!storage.exists(&test_hash()).await);
    }

    #[tokio::test]
    async fn delete_returns_true_for_existing() {
        let dir = TempDir::new().unwrap();
        let storage = make_storage(&dir).await;
        let hash = test_hash();
        storage.store(&hash, b"data").await.unwrap();
        assert!(storage.delete(&hash).await.unwrap());
    }

    #[tokio::test]
    async fn delete_returns_false_for_nonexistent() {
        let dir = TempDir::new().unwrap();
        let storage = make_storage(&dir).await;
        assert!(!storage.delete(&test_hash()).await.unwrap());
    }

    #[tokio::test]
    async fn delete_removes_file() {
        let dir = TempDir::new().unwrap();
        let storage = make_storage(&dir).await;
        let hash = test_hash();
        storage.store(&hash, b"data").await.unwrap();
        storage.delete(&hash).await.unwrap();
        assert!(!storage.exists(&hash).await);
    }

    #[tokio::test]
    async fn delete_cleans_empty_parent_dirs() {
        let dir = TempDir::new().unwrap();
        let storage = make_storage(&dir).await;
        let hash = test_hash();
        storage.store(&hash, b"data").await.unwrap();
        let leaf_dir = storage.full_path(&hash).parent().unwrap().to_path_buf();
        storage.delete(&hash).await.unwrap();
        assert!(!leaf_dir.exists());
    }

    #[tokio::test]
    async fn delete_preserves_nonempty_parent() {
        let dir = TempDir::new().unwrap();
        let storage = make_storage(&dir).await;

        // Two hashes sharing the first 2 chars: "aa/bb/..." and "aa/cc/..."
        let hash1 = ContentHash::new(&format!("aabb{}", "0".repeat(60))).unwrap();
        let hash2 = ContentHash::new(&format!("aacc{}", "0".repeat(60))).unwrap();

        storage.store(&hash1, b"one").await.unwrap();
        storage.store(&hash2, b"two").await.unwrap();

        let shared_parent = storage.base_path.join("aa");
        storage.delete(&hash1).await.unwrap();

        assert!(shared_parent.is_dir());
        assert!(storage.exists(&hash2).await);
    }

    #[tokio::test]
    async fn health_check_succeeds() {
        let dir = TempDir::new().unwrap();
        let storage = make_storage(&dir).await;
        storage.health_check().await.unwrap();
        let probe = storage.temp_path.join(".health_check_probe");
        assert!(!probe.exists());
    }
}
