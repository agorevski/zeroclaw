use async_trait::async_trait;
use std::path::{Path, PathBuf};

use super::traits::{MediaEntry, MediaId, MediaMetadata, MediaStore};

/// Local filesystem media store. Files are stored under a configurable
/// base directory using UUID-based filenames to avoid collisions.
pub struct LocalMediaStore {
    base_dir: PathBuf,
}

impl LocalMediaStore {
    pub fn new(base_dir: &Path) -> Self {
        Self {
            base_dir: base_dir.to_path_buf(),
        }
    }
}

#[async_trait]
impl MediaStore for LocalMediaStore {
    async fn store(&self, data: &[u8], metadata: MediaMetadata) -> anyhow::Result<MediaEntry> {
        tokio::fs::create_dir_all(&self.base_dir).await?;

        let id = uuid::Uuid::new_v4().to_string();
        let extension = metadata
            .filename
            .as_deref()
            .and_then(|f| Path::new(f).extension())
            .and_then(|e| e.to_str())
            .unwrap_or("bin");
        let filename = format!("{id}.{extension}");
        let path = self.base_dir.join(&filename);

        tokio::fs::write(&path, data).await?;

        Ok(MediaEntry {
            id: MediaId(id),
            path,
            metadata,
            created_at: chrono::Utc::now(),
        })
    }

    async fn get(&self, id: &MediaId) -> anyhow::Result<Option<MediaEntry>> {
        let mut entries = tokio::fs::read_dir(&self.base_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let file_name = entry.file_name().to_string_lossy().to_string();
            if file_name.starts_with(&id.0) {
                let path = entry.path();
                let fs_meta = entry.metadata().await?;
                return Ok(Some(MediaEntry {
                    id: MediaId(id.0.clone()),
                    path,
                    metadata: MediaMetadata {
                        filename: Some(file_name),
                        mime_type: None,
                        size_bytes: Some(fs_meta.len()),
                        source_url: None,
                    },
                    created_at: chrono::Utc::now(),
                }));
            }
        }
        Ok(None)
    }

    async fn delete(&self, id: &MediaId) -> anyhow::Result<()> {
        let mut entries = tokio::fs::read_dir(&self.base_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let file_name = entry.file_name().to_string_lossy().to_string();
            if file_name.starts_with(&id.0) {
                tokio::fs::remove_file(entry.path()).await?;
                return Ok(());
            }
        }
        Ok(())
    }

    async fn list(&self) -> anyhow::Result<Vec<MediaEntry>> {
        if !self.base_dir.exists() {
            return Ok(Vec::new());
        }
        let mut results = Vec::new();
        let mut entries = tokio::fs::read_dir(&self.base_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let file_name = entry.file_name().to_string_lossy().to_string();
            let path = entry.path();
            let fs_meta = entry.metadata().await?;
            // Extract the UUID portion before the first dot
            let id = file_name
                .split('.')
                .next()
                .unwrap_or(&file_name)
                .to_string();
            results.push(MediaEntry {
                id: MediaId(id),
                path,
                metadata: MediaMetadata {
                    filename: Some(file_name),
                    mime_type: None,
                    size_bytes: Some(fs_meta.len()),
                    source_url: None,
                },
                created_at: chrono::Utc::now(),
            });
        }
        Ok(results)
    }

    fn name(&self) -> &str {
        "local"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn store_and_get_roundtrip() {
        let tmp = TempDir::new().unwrap();
        let store = LocalMediaStore::new(tmp.path());
        let meta = MediaMetadata {
            filename: Some("test.txt".into()),
            mime_type: Some("text/plain".into()),
            size_bytes: Some(5),
            source_url: None,
        };
        let entry = store.store(b"hello", meta).await.unwrap();
        assert!(entry.path.exists());

        let found = store.get(&entry.id).await.unwrap();
        assert!(found.is_some());
    }

    #[tokio::test]
    async fn delete_removes_file() {
        let tmp = TempDir::new().unwrap();
        let store = LocalMediaStore::new(tmp.path());
        let meta = MediaMetadata {
            filename: Some("rm.bin".into()),
            mime_type: None,
            size_bytes: None,
            source_url: None,
        };
        let entry = store.store(b"data", meta).await.unwrap();
        assert!(entry.path.exists());

        store.delete(&entry.id).await.unwrap();
        assert!(!entry.path.exists());
    }

    #[tokio::test]
    async fn list_returns_stored_entries() {
        let tmp = TempDir::new().unwrap();
        let store = LocalMediaStore::new(tmp.path());
        let meta = MediaMetadata {
            filename: Some("a.bin".into()),
            mime_type: None,
            size_bytes: None,
            source_url: None,
        };
        store.store(b"one", meta.clone()).await.unwrap();
        store.store(b"two", meta).await.unwrap();

        let entries = store.list().await.unwrap();
        assert_eq!(entries.len(), 2);
    }
}
