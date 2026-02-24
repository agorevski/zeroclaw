//! File-based workspace implementation.
//!
//! Reads workspace markdown files from a directory on disk.

use super::traits::Workspace;
use async_trait::async_trait;
use std::path::{Path, PathBuf};

/// Maximum content size per workspace file (20 KB).
const MAX_CONTENT_BYTES: usize = 20 * 1024;

/// File-based workspace that reads `.md` files from a directory.
pub struct FileWorkspace {
    root: PathBuf,
}

impl FileWorkspace {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }
}

/// Read a file if it exists, truncating to `MAX_CONTENT_BYTES`.
async fn read_workspace_file(path: &Path) -> anyhow::Result<Option<String>> {
    match tokio::fs::read_to_string(path).await {
        Ok(content) => {
            if content.len() > MAX_CONTENT_BYTES {
                // Truncate at a char boundary.
                let mut end = MAX_CONTENT_BYTES;
                while end > 0 && !content.is_char_boundary(end) {
                    end -= 1;
                }
                Ok(Some(content[..end].to_string()))
            } else {
                Ok(Some(content))
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.into()),
    }
}

#[async_trait]
impl Workspace for FileWorkspace {
    fn path(&self) -> &Path {
        &self.root
    }

    async fn agent_instructions(&self) -> anyhow::Result<Option<String>> {
        read_workspace_file(&self.root.join("AGENTS.md")).await
    }

    async fn soul(&self) -> anyhow::Result<Option<String>> {
        read_workspace_file(&self.root.join("SOUL.md")).await
    }

    async fn tools_config(&self) -> anyhow::Result<Option<String>> {
        read_workspace_file(&self.root.join("TOOLS.md")).await
    }

    async fn identity(&self) -> anyhow::Result<Option<String>> {
        read_workspace_file(&self.root.join("IDENTITY.md")).await
    }

    async fn user_context(&self) -> anyhow::Result<Option<String>> {
        read_workspace_file(&self.root.join("USER.md")).await
    }

    async fn memory_notes(&self) -> anyhow::Result<Option<String>> {
        read_workspace_file(&self.root.join("MEMORY.md")).await
    }

    async fn heartbeat_config(&self) -> anyhow::Result<Option<String>> {
        read_workspace_file(&self.root.join("HEARTBEAT.md")).await
    }

    async fn bootstrap_content(&self) -> anyhow::Result<Option<String>> {
        read_workspace_file(&self.root.join("BOOTSTRAP.md")).await
    }

    fn name(&self) -> &str {
        "file"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn file_workspace_returns_none_for_missing_files() {
        let tmp = TempDir::new().unwrap();
        let ws = FileWorkspace::new(tmp.path().to_path_buf());

        assert!(ws.agent_instructions().await.unwrap().is_none());
        assert!(ws.soul().await.unwrap().is_none());
        assert!(ws.tools_config().await.unwrap().is_none());
        assert!(ws.identity().await.unwrap().is_none());
        assert!(ws.user_context().await.unwrap().is_none());
        assert!(ws.memory_notes().await.unwrap().is_none());
        assert!(ws.heartbeat_config().await.unwrap().is_none());
        assert!(ws.bootstrap_content().await.unwrap().is_none());
    }

    #[tokio::test]
    async fn file_workspace_reads_existing_file() {
        let tmp = TempDir::new().unwrap();
        let content = "# Agent Instructions\nBe helpful.";
        std::fs::write(tmp.path().join("AGENTS.md"), content).unwrap();

        let ws = FileWorkspace::new(tmp.path().to_path_buf());
        let result = ws.agent_instructions().await.unwrap();
        assert_eq!(result.as_deref(), Some(content));
    }

    #[tokio::test]
    async fn file_workspace_truncates_large_files() {
        let tmp = TempDir::new().unwrap();
        let content = "a".repeat(30_000);
        std::fs::write(tmp.path().join("SOUL.md"), &content).unwrap();

        let ws = FileWorkspace::new(tmp.path().to_path_buf());
        let result = ws.soul().await.unwrap().unwrap();
        assert!(result.len() <= MAX_CONTENT_BYTES);
    }

    #[test]
    fn file_workspace_name() {
        let ws = FileWorkspace::new(PathBuf::from("/tmp/test"));
        assert_eq!(ws.name(), "file");
    }

    #[test]
    fn file_workspace_path() {
        let ws = FileWorkspace::new(PathBuf::from("/tmp/test"));
        assert_eq!(ws.path(), Path::new("/tmp/test"));
    }
}
