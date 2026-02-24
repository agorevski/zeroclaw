use anyhow::Result;
use async_trait::async_trait;

use super::traits::{CommandContext, CommandResult, SlashCommandHandler};

/// Example slash command handler that returns a list of available commands.
pub struct HelpCommand {
    available_commands: Vec<(String, String)>,
}

impl HelpCommand {
    pub fn new(available_commands: Vec<(String, String)>) -> Self {
        Self { available_commands }
    }
}

#[async_trait]
impl SlashCommandHandler for HelpCommand {
    fn command(&self) -> &str {
        "/help"
    }

    fn description(&self) -> &str {
        "Show available commands"
    }

    async fn execute(&self, _args: &str, _context: &CommandContext) -> Result<CommandResult> {
        let mut lines = vec!["Available commands:".to_string()];
        for (cmd, desc) in &self.available_commands {
            lines.push(format!("  {cmd} — {desc}"));
        }
        Ok(CommandResult {
            output: lines.join("\n"),
            consumed: true,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[tokio::test]
    async fn help_command_returns_list() {
        let cmd = HelpCommand::new(vec![
            ("/help".to_string(), "Show help".to_string()),
            ("/compact".to_string(), "Compact session".to_string()),
        ]);
        let ctx = CommandContext {
            sender: "zeroclaw_user".to_string(),
            channel: "test".to_string(),
            session_key: None,
            metadata: HashMap::new(),
        };
        let result = cmd.execute("", &ctx).await.unwrap();
        assert!(result.consumed);
        assert!(result.output.contains("/help"));
        assert!(result.output.contains("/compact"));
    }

    #[test]
    fn help_command_metadata() {
        let cmd = HelpCommand::new(Vec::new());
        assert_eq!(cmd.command(), "/help");
        assert!(!cmd.description().is_empty());
    }
}
