use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;

use super::traits::{
    CommandContext, DirectiveParser, DispatchContext, DispatchResult, Dispatcher, InboundMessage,
    SlashCommandHandler,
};

/// Default dispatcher that parses directives/commands and routes messages.
pub struct DefaultDispatcher {
    parser: Box<dyn DirectiveParser>,
    commands: Vec<Box<dyn SlashCommandHandler>>,
}

impl DefaultDispatcher {
    pub fn new(
        parser: Box<dyn DirectiveParser>,
        commands: Vec<Box<dyn SlashCommandHandler>>,
    ) -> Self {
        Self { parser, commands }
    }
}

#[async_trait]
impl Dispatcher for DefaultDispatcher {
    async fn dispatch(
        &self,
        message: InboundMessage,
        context: &DispatchContext,
    ) -> Result<DispatchResult> {
        let parsed = self.parser.parse(&message.content);

        // Handle slash commands
        if let Some(ref slash_cmd) = parsed.slash_command {
            for handler in &self.commands {
                if handler.command() == slash_cmd {
                    let cmd_context = CommandContext {
                        sender: message.sender.clone(),
                        channel: message.channel.clone(),
                        session_key: Some(context.session_key.clone()),
                        metadata: message.metadata.clone(),
                    };
                    let args = parsed.slash_args.as_deref().unwrap_or("");
                    let result = handler.execute(args, &cmd_context).await?;
                    if result.consumed {
                        return Ok(DispatchResult::CommandHandled {
                            output: result.output,
                        });
                    }
                }
            }
        }

        // Forward clean text for agent processing
        let mut metadata = HashMap::new();
        for directive in &parsed.directives {
            if let Some(ref value) = directive.value {
                metadata.insert(directive.name.clone(), value.clone());
            }
        }

        Ok(DispatchResult::Reply {
            content: parsed.clean_text,
            metadata,
        })
    }

    fn name(&self) -> &str {
        "default"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auto_reply::commands::HelpCommand;
    use crate::auto_reply::directives::DefaultDirectiveParser;

    fn test_context() -> DispatchContext {
        DispatchContext {
            agent_id: "zeroclaw_agent".to_string(),
            session_key: "test_session".to_string(),
            config: HashMap::new(),
        }
    }

    fn test_message(content: &str) -> InboundMessage {
        InboundMessage {
            sender: "zeroclaw_user".to_string(),
            content: content.to_string(),
            channel: "test".to_string(),
            timestamp: chrono::Utc::now(),
            attachments: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    #[tokio::test]
    async fn dispatch_plain_message() {
        let parser = Box::new(DefaultDirectiveParser);
        let dispatcher = DefaultDispatcher::new(parser, Vec::new());
        let result = dispatcher
            .dispatch(test_message("hello world"), &test_context())
            .await
            .unwrap();

        match result {
            DispatchResult::Reply { content, .. } => assert_eq!(content, "hello world"),
            other => panic!("Expected Reply, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn dispatch_slash_command() {
        let parser = Box::new(DefaultDirectiveParser);
        let help = HelpCommand::new(vec![("/help".to_string(), "Show help".to_string())]);
        let commands: Vec<Box<dyn SlashCommandHandler>> = vec![Box::new(help)];
        let dispatcher = DefaultDispatcher::new(parser, commands);

        let result = dispatcher
            .dispatch(test_message("/help"), &test_context())
            .await
            .unwrap();

        match result {
            DispatchResult::CommandHandled { output } => {
                assert!(output.contains("Available commands"));
            }
            other => panic!("Expected CommandHandled, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn dispatch_with_directive_extracts_metadata() {
        let parser = Box::new(DefaultDirectiveParser);
        let dispatcher = DefaultDispatcher::new(parser, Vec::new());
        let result = dispatcher
            .dispatch(test_message("@model(gpt-4) do the thing"), &test_context())
            .await
            .unwrap();

        match result {
            DispatchResult::Reply { metadata, .. } => {
                assert_eq!(metadata.get("model").map(|s| s.as_str()), Some("gpt-4"));
            }
            other => panic!("Expected Reply, got {:?}", other),
        }
    }
}
