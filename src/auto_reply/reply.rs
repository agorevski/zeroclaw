use anyhow::Result;
use async_trait::async_trait;

use super::traits::{ReplyContext, ReplyDispatcher, ReplyMessage};

/// Simple pass-through reply dispatcher that logs replies.
pub struct DefaultReplyDispatcher;

#[async_trait]
impl ReplyDispatcher for DefaultReplyDispatcher {
    async fn send(&self, reply: ReplyMessage, _context: &ReplyContext) -> Result<()> {
        tracing::info!(
            recipient = %reply.recipient,
            channel = %reply.channel,
            streaming = reply.is_streaming,
            "dispatching reply"
        );
        Ok(())
    }

    async fn flush(&self) -> Result<()> {
        Ok(())
    }

    fn name(&self) -> &str {
        "default"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[tokio::test]
    async fn send_reply_succeeds() {
        let dispatcher = DefaultReplyDispatcher;
        let reply = ReplyMessage {
            content: "test reply".to_string(),
            recipient: "zeroclaw_user".to_string(),
            channel: "test".to_string(),
            is_streaming: false,
            metadata: HashMap::new(),
        };
        let ctx = ReplyContext {
            session_key: "test_session".to_string(),
            rate_limit_ms: 0,
        };
        assert!(dispatcher.send(reply, &ctx).await.is_ok());
    }

    #[tokio::test]
    async fn flush_is_noop() {
        let dispatcher = DefaultReplyDispatcher;
        assert!(dispatcher.flush().await.is_ok());
    }
}
