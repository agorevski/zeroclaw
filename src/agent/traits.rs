//! Agent orchestration traits for pluggable agent behavior.

use async_trait::async_trait;
use anyhow::Result;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

/// Agent orchestrator interface.
///
/// This is the top-level "brain" interface. Implement this trait to provide
/// alternative agent orchestration strategies (e.g., ReAct, chain-of-thought,
/// tree-of-thought, etc.)
#[async_trait]
pub trait AgentOrchestrator: Send + Sync {
    /// Run the agent with a user message and return the final response.
    async fn run(&self, input: &AgentInput) -> Result<AgentOutput>;
    /// Run the agent in streaming mode (if supported).
    async fn run_streaming(&self, input: &AgentInput) -> Result<AgentOutput> {
        self.run(input).await
    }
    fn name(&self) -> &str;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInput {
    pub message: String,
    pub session_key: Option<String>,
    pub context: HashMap<String, String>,
    pub attachments: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentOutput {
    pub response: String,
    pub tool_calls_made: u32,
    pub tokens_used: Option<u64>,
    pub model: Option<String>,
    pub metadata: HashMap<String, String>,
}

/// Prompt builder interface for composable system prompts.
///
/// Matches OpenClaw's modular prompt system with sections for identity,
/// tooling, safety, workspace, runtime, and datetime.
pub trait PromptBuilder: Send + Sync {
    /// Build the complete system prompt from registered sections.
    fn build(&self) -> String;
    /// Add a named section to the prompt.
    fn add_section(&mut self, section: Box<dyn PromptSectionTrait>);
    /// List registered section names.
    fn section_names(&self) -> Vec<&str>;
    fn name(&self) -> &str;
}

/// A single section of the system prompt (identity, tools, safety, etc.)
pub trait PromptSectionTrait: Send + Sync {
    fn section_name(&self) -> &str;
    fn render(&self) -> String;
    fn priority(&self) -> i32 { 0 }
}

/// Query classifier for routing queries to specialized handling.
pub trait QueryClassifier: Send + Sync {
    /// Classify a user query.
    fn classify(&self, query: &str) -> QueryClassification;
    fn name(&self) -> &str;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum QueryClassification {
    General,
    CodeGeneration,
    Analysis,
    Search,
    Action,
    Clarification,
    Custom(String),
}

/// No-op query classifier that always returns General.
#[derive(Debug, Clone, Default)]
pub struct NoopQueryClassifier;

impl QueryClassifier for NoopQueryClassifier {
    fn classify(&self, _query: &str) -> QueryClassification {
        QueryClassification::General
    }
    fn name(&self) -> &str { "noop" }
}

/// Context compactor for managing conversation history overflow.
///
/// Matches OpenClaw's compaction system that summarizes older messages
/// to reduce token count when context window is exceeded.
#[async_trait]
pub trait ContextCompactor: Send + Sync {
    /// Compact conversation history, returning summarized version.
    async fn compact(&self, messages: &[CompactMessage], max_tokens: usize) -> Result<Vec<CompactMessage>>;
    fn name(&self) -> &str;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactMessage {
    pub role: String,
    pub content: String,
    pub is_summary: bool,
}
