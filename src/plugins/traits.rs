use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Hook types covering the full agent lifecycle.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum HookEventType {
    AgentStart,
    AgentEnd,
    ModelSelect,
    PromptBuild,
    ToolCallBefore,
    ToolCallAfter,
    SessionStart,
    SessionEnd,
    GatewayConnect,
    GatewayDisconnect,
    LlmRequest,
    LlmResponse,
    Compaction,
    MessageInbound,
    MessageOutbound,
}

/// Payload delivered to hooks when a lifecycle event fires.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookEvent {
    pub event_type: HookEventType,
    pub data: HashMap<String, serde_json::Value>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Action returned by a hook to control pipeline flow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HookAction {
    Continue,
    Modify(HashMap<String, serde_json::Value>),
    Cancel { reason: String },
}

/// A named, priority-ordered lifecycle hook.
#[async_trait]
pub trait Hook: Send + Sync {
    fn name(&self) -> &str;
    fn event_types(&self) -> Vec<HookEventType>;
    fn priority(&self) -> i32 {
        0
    }
    async fn execute(&self, event: &HookEvent) -> Result<HookAction>;
}

/// Descriptor for a CLI command contributed by a plugin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginCommand {
    pub name: String,
    pub description: String,
    pub usage: Option<String>,
}

/// Runtime context supplied to a plugin during load.
#[derive(Debug, Clone)]
pub struct PluginContext {
    pub workspace_dir: PathBuf,
    pub config: HashMap<String, serde_json::Value>,
}

/// Extension point for packaging tools, hooks, and commands into a
/// loadable unit. Default implementations return empty collections so
/// plugins only need to implement what they provide.
#[async_trait]
pub trait Plugin: Send + Sync {
    fn name(&self) -> &str;
    fn version(&self) -> &str;

    async fn on_load(&mut self, context: &PluginContext) -> Result<()> {
        let _ = context;
        Ok(())
    }

    async fn on_unload(&mut self) -> Result<()> {
        Ok(())
    }

    fn tools(&self) -> Vec<Box<dyn crate::tools::Tool>> {
        vec![]
    }

    fn hooks(&self) -> Vec<Box<dyn Hook>> {
        vec![]
    }

    fn commands(&self) -> Vec<PluginCommand> {
        vec![]
    }
}

/// Manager responsible for loading, unloading, and dispatching across
/// the set of active plugins.
#[async_trait]
pub trait PluginManager: Send + Sync {
    async fn load_plugin(&self, path: &std::path::Path) -> Result<()>;
    async fn unload_plugin(&self, name: &str) -> Result<()>;
    fn list_plugins(&self) -> Vec<&str>;
    fn get_all_tools(&self) -> Vec<Box<dyn crate::tools::Tool>>;
    fn get_all_hooks(&self, event_type: &HookEventType) -> Vec<&dyn Hook>;
    async fn dispatch_hook(&self, event: &HookEvent) -> Result<HookAction>;
    fn name(&self) -> &str;
}
