/// The trait for describing a channel
pub trait ChannelConfig {
    /// human-readable name
    fn name() -> &'static str;
    /// short description
    fn desc() -> &'static str;
}

// Maybe there should be a `&self` as parameter for custom channel/info or what...

pub trait ConfigHandle {
    fn name(&self) -> &'static str;
    fn desc(&self) -> &'static str;
}

// --- Extension traits for OpenClaw architecture parity ---

use async_trait::async_trait;
use std::path::Path;

/// Workspace abstraction matching OpenClaw's workspace system.
///
/// Each agent runs within an isolated workspace with specific files
/// that control behavior, personality, tools, and memory.
#[async_trait]
pub trait Workspace: Send + Sync {
    /// Root path of this workspace.
    fn path(&self) -> &Path;
    /// Agent instructions (AGENTS.md).
    async fn agent_instructions(&self) -> anyhow::Result<Option<String>>;
    /// Agent personality/voice (SOUL.md).
    async fn soul(&self) -> anyhow::Result<Option<String>>;
    /// Custom tool definitions (TOOLS.md).
    async fn tools_config(&self) -> anyhow::Result<Option<String>>;
    /// Agent identity (IDENTITY.md).
    async fn identity(&self) -> anyhow::Result<Option<String>>;
    /// User context and preferences (USER.md).
    async fn user_context(&self) -> anyhow::Result<Option<String>>;
    /// Persistent memory notes (MEMORY.md).
    async fn memory_notes(&self) -> anyhow::Result<Option<String>>;
    /// Periodic task instructions (HEARTBEAT.md).
    async fn heartbeat_config(&self) -> anyhow::Result<Option<String>>;
    /// First-run onboarding (BOOTSTRAP.md).
    async fn bootstrap_content(&self) -> anyhow::Result<Option<String>>;
    /// Return the workspace name.
    fn name(&self) -> &str;
}

/// Config loader abstraction for pluggable config sources.
#[async_trait]
pub trait ConfigLoader: Send + Sync {
    /// Load configuration from the source.
    async fn load(&self) -> anyhow::Result<crate::Config>;
    /// Save configuration back to the source.
    async fn save(&self, config: &crate::Config) -> anyhow::Result<()>;
    /// Return the config file path (if file-based).
    fn config_path(&self) -> Option<&Path>;
    /// Return the loader name.
    fn name(&self) -> &str;
}

/// Config validator for checking configuration consistency.
pub trait ConfigValidator: Send + Sync {
    /// Validate a configuration, returning a list of warnings/errors.
    fn validate(&self, config: &crate::Config) -> Vec<ConfigIssue>;
    /// Return the validator name.
    fn name(&self) -> &str;
}

/// Severity level for configuration issues.
#[derive(Debug, Clone)]
pub enum ConfigIssueSeverity {
    Warning,
    Error,
}

/// A single configuration issue found during validation.
#[derive(Debug, Clone)]
pub struct ConfigIssue {
    pub severity: ConfigIssueSeverity,
    pub field: String,
    pub message: String,
}
