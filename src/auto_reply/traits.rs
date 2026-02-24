use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedDirective {
    pub name: String,
    pub value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedMessage {
    pub clean_text: String,
    pub directives: Vec<ParsedDirective>,
    pub slash_command: Option<String>,
    pub slash_args: Option<String>,
}

pub trait DirectiveParser: Send + Sync {
    fn parse(&self, text: &str) -> ParsedMessage;
    fn supported_directives(&self) -> Vec<&str>;
    fn name(&self) -> &str;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandResult {
    pub output: String,
    pub consumed: bool,
}

#[derive(Debug, Clone)]
pub struct CommandContext {
    pub sender: String,
    pub channel: String,
    pub session_key: Option<String>,
    pub metadata: HashMap<String, String>,
}

#[async_trait]
pub trait SlashCommandHandler: Send + Sync {
    fn command(&self) -> &str;
    fn description(&self) -> &str;
    async fn execute(&self, args: &str, context: &CommandContext) -> Result<CommandResult>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InboundMessage {
    pub sender: String,
    pub content: String,
    pub channel: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub attachments: Vec<String>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DispatchResult {
    Reply {
        content: String,
        metadata: HashMap<String, String>,
    },
    Deferred {
        reason: String,
    },
    Blocked {
        reason: String,
    },
    CommandHandled {
        output: String,
    },
}

#[derive(Debug, Clone)]
pub struct DispatchContext {
    pub agent_id: String,
    pub session_key: String,
    pub config: HashMap<String, String>,
}

#[async_trait]
pub trait Dispatcher: Send + Sync {
    async fn dispatch(
        &self,
        message: InboundMessage,
        context: &DispatchContext,
    ) -> Result<DispatchResult>;
    fn name(&self) -> &str;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplyMessage {
    pub content: String,
    pub recipient: String,
    pub channel: String,
    pub is_streaming: bool,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct ReplyContext {
    pub session_key: String,
    pub rate_limit_ms: u64,
}

#[async_trait]
pub trait ReplyDispatcher: Send + Sync {
    async fn send(&self, reply: ReplyMessage, context: &ReplyContext) -> Result<()>;
    async fn flush(&self) -> Result<()>;
    fn name(&self) -> &str;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parsed_directive_serialization() {
        let d = ParsedDirective {
            name: "model".to_string(),
            value: Some("gpt-4".to_string()),
        };
        let json = serde_json::to_string(&d).unwrap();
        let back: ParsedDirective = serde_json::from_str(&json).unwrap();
        assert_eq!(back.name, "model");
        assert_eq!(back.value.as_deref(), Some("gpt-4"));
    }

    #[test]
    fn dispatch_result_variants_serialize() {
        let reply = DispatchResult::Reply {
            content: "hello".to_string(),
            metadata: HashMap::new(),
        };
        let json = serde_json::to_string(&reply).unwrap();
        assert!(json.contains("Reply"));

        let blocked = DispatchResult::Blocked {
            reason: "rate limit".to_string(),
        };
        let json = serde_json::to_string(&blocked).unwrap();
        assert!(json.contains("Blocked"));
    }

    #[test]
    fn inbound_message_serialization() {
        let msg = InboundMessage {
            sender: "zeroclaw_user".to_string(),
            content: "hello".to_string(),
            channel: "test".to_string(),
            timestamp: chrono::Utc::now(),
            attachments: Vec::new(),
            metadata: HashMap::new(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("zeroclaw_user"));
    }
}
