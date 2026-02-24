//! Routing traits and types for resolving which agent handles a conversation.

use async_trait::async_trait;
use anyhow::Result;
use serde::{Deserialize, Serialize};

/// The type of chat context for routing decisions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChatType {
    Direct,
    Group,
    Channel,
}

/// How a route was matched to an agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MatchedBy {
    Peer,
    Guild,
    Account,
    Channel,
    Default,
}

/// Context provided to the router for resolving a route.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteContext {
    pub channel: String,
    pub sender: String,
    pub recipient: Option<String>,
    pub chat_type: ChatType,
    pub account_id: Option<String>,
    pub guild_id: Option<String>,
}

/// The result of a successful route resolution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteMatch {
    pub agent_id: String,
    pub session_key: String,
    pub matched_by: MatchedBy,
}

/// A binding that maps a channel pattern to an agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteBinding {
    pub id: String,
    pub channel: String,
    pub pattern: String,
    pub agent_id: String,
    pub priority: u32,
}

/// Routes incoming messages to the appropriate agent based on bindings.
#[async_trait]
pub trait Router: Send + Sync {
    /// Resolve which agent should handle this conversation context.
    async fn resolve_route(&self, context: &RouteContext) -> Result<RouteMatch>;

    /// Register a new route binding.
    async fn add_binding(&self, binding: RouteBinding) -> Result<()>;

    /// Remove a route binding by ID.
    async fn remove_binding(&self, binding_id: &str) -> Result<()>;

    /// List all registered bindings.
    async fn list_bindings(&self) -> Result<Vec<RouteBinding>>;

    /// The name of this router implementation.
    fn name(&self) -> &str;
}
