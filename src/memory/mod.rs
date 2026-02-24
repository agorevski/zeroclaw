pub mod sqlite;
pub mod traits;

pub use sqlite::SqliteMemory;
pub use traits::Memory;
#[allow(unused_imports)]
pub use traits::{MemoryCategory, MemoryEntry};

use crate::config::MemoryConfig;
use anyhow::Result;
use std::path::Path;

/// Return the effective memory backend name (always sqlite after the strip).
pub fn effective_memory_backend_name(memory_backend: &str) -> String {
    memory_backend.trim().to_ascii_lowercase()
}

/// Legacy auto-save key used for model-authored assistant summaries.
/// These entries are treated as untrusted context and should not be re-injected.
pub fn is_assistant_autosave_key(key: &str) -> bool {
    let normalized = key.trim().to_ascii_lowercase();
    normalized == "assistant_resp" || normalized.starts_with("assistant_resp_")
}

/// Factory: create the right memory backend from config
pub fn create_memory(
    _config: &MemoryConfig,
    workspace_dir: &Path,
    _api_key: Option<&str>,
) -> anyhow::Result<Box<dyn Memory>> {
    Ok(Box::new(SqliteMemory::new(workspace_dir)?))
}

/// Factory: create memory with optional storage-provider override.
///
/// Simplified after removing `StorageProviderConfig`.
pub fn create_memory_with_storage(
    _config: &MemoryConfig,
    workspace_dir: &Path,
    _api_key: Option<&str>,
) -> anyhow::Result<Box<dyn Memory>> {
    Ok(Box::new(SqliteMemory::new(workspace_dir)?))
}

/// Factory: create memory with storage and embedding routes.
///
/// Simplified after removing `EmbeddingRouteConfig` and `StorageProviderConfig`.
pub fn create_memory_with_storage_and_routes(
    _config: &MemoryConfig,
    workspace_dir: &Path,
    _api_key: Option<&str>,
) -> anyhow::Result<Box<dyn Memory>> {
    Ok(Box::new(SqliteMemory::new(workspace_dir)?))
}

pub fn create_memory_for_migration(
    _backend: &str,
    workspace_dir: &Path,
) -> anyhow::Result<Box<dyn Memory>> {
    Ok(Box::new(SqliteMemory::new(workspace_dir)?))
}

// ── CLI handler (inlined from deleted cli.rs) ──

/// Handle `zeroclaw memory <subcommand>` CLI commands.
pub async fn handle_memory_command(
    command: crate::MemoryCommands,
    config: &crate::config::Config,
) -> Result<()> {
    let mem = SqliteMemory::new(&config.workspace_dir)?;
    match command {
        crate::MemoryCommands::List {
            category,
            session,
            limit,
            offset,
        } => {
            let cat = category.as_deref().map(parse_category);
            let entries = mem.list(cat.as_ref(), session.as_deref()).await?;
            if entries.is_empty() {
                println!("No memory entries found.");
                return Ok(());
            }
            let total = entries.len();
            let page: Vec<_> = entries.into_iter().skip(offset).take(limit).collect();
            if page.is_empty() {
                println!("No entries at offset {offset} (total: {total}).");
                return Ok(());
            }
            println!(
                "Memory entries ({total} total, showing {}-{}):\n",
                offset + 1,
                offset + page.len(),
            );
            for entry in &page {
                println!("- {} [{}]", entry.key, entry.category);
                let line = entry.content.lines().next().unwrap_or(&entry.content);
                let display = if line.len() <= 80 {
                    line.to_string()
                } else {
                    let truncated: String = line.chars().take(77).collect();
                    format!("{truncated}...")
                };
                println!("    {display}");
            }
            if offset + page.len() < total {
                println!("\n  Use --offset {} to see the next page.", offset + limit);
            }
        }
        crate::MemoryCommands::Get { key } => {
            if let Some(entry) = mem.get(&key).await? {
                println!("Key:       {}", entry.key);
                println!("Category:  {}", entry.category);
                println!("Timestamp: {}", entry.timestamp);
                if let Some(sid) = &entry.session_id {
                    println!("Session:   {sid}");
                }
                println!("\n{}", entry.content);
            } else {
                println!("No memory entry found for key: {key}");
            }
        }
        crate::MemoryCommands::Stats => {
            let healthy = mem.health_check().await;
            let total = mem.count().await.unwrap_or(0);
            println!("Memory Statistics:\n");
            println!("  Backend:  {}", mem.name());
            println!(
                "  Health:   {}",
                if healthy { "healthy" } else { "unhealthy" }
            );
            println!("  Total:    {total}");
        }
        crate::MemoryCommands::Clear {
            key,
            category,
            yes,
        } => {
            if let Some(key) = key {
                if !yes {
                    eprintln!("Use --yes to confirm deletion of key '{key}'.");
                    return Ok(());
                }
                if mem.forget(&key).await? {
                    println!("✓ Deleted key: {key}");
                } else {
                    println!("No memory entry found for key: {key}");
                }
            } else {
                let cat = category.as_deref().map(parse_category);
                let entries = mem.list(cat.as_ref(), None).await?;
                if entries.is_empty() {
                    println!("No entries to clear.");
                    return Ok(());
                }
                let scope = category.as_deref().unwrap_or("all categories");
                println!("Found {} entries in '{scope}'.", entries.len());
                if !yes {
                    eprintln!(
                        "Use --yes to confirm deletion of {} entries.",
                        entries.len()
                    );
                    return Ok(());
                }
                let mut deleted = 0usize;
                for entry in &entries {
                    if mem.forget(&entry.key).await? {
                        deleted += 1;
                    }
                }
                println!("✓ Cleared {deleted}/{} entries.", entries.len());
            }
        }
    }
    Ok(())
}

fn parse_category(s: &str) -> MemoryCategory {
    match s.trim().to_ascii_lowercase().as_str() {
        "core" => MemoryCategory::Core,
        "daily" => MemoryCategory::Daily,
        "conversation" => MemoryCategory::Conversation,
        other => MemoryCategory::Custom(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::MemoryConfig;
    use tempfile::TempDir;

    #[test]
    fn factory_sqlite() {
        let tmp = TempDir::new().unwrap();
        let cfg = MemoryConfig {
            backend: "sqlite".into(),
            ..MemoryConfig::default()
        };
        let mem = create_memory(&cfg, tmp.path(), None).unwrap();
        assert_eq!(mem.name(), "sqlite");
    }

    #[test]
    fn assistant_autosave_key_detection_matches_legacy_patterns() {
        assert!(is_assistant_autosave_key("assistant_resp"));
        assert!(is_assistant_autosave_key("assistant_resp_1234"));
        assert!(is_assistant_autosave_key("ASSISTANT_RESP_abcd"));
        assert!(!is_assistant_autosave_key("assistant_response"));
        assert!(!is_assistant_autosave_key("user_msg_1234"));
    }

    #[test]
    fn effective_backend_always_returns_sqlite() {
        assert_eq!(
            effective_memory_backend_name("sqlite"),
            "sqlite"
        );
    }

    #[test]
    fn migration_factory_creates_sqlite() {
        let tmp = TempDir::new().unwrap();
        let mem = create_memory_for_migration("sqlite", tmp.path()).unwrap();
        assert_eq!(mem.name(), "sqlite");
    }
}
