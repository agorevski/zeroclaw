//! Default in-memory router implementation.

use anyhow::Result;
use async_trait::async_trait;
use parking_lot::Mutex;

use super::traits::{ChatType, MatchedBy, RouteBinding, RouteContext, RouteMatch, Router};

/// A simple in-memory router that resolves by priority-ordered bindings,
/// falling back to a configurable default agent.
pub struct DefaultRouter {
    default_agent_id: String,
    bindings: Mutex<Vec<RouteBinding>>,
}

impl DefaultRouter {
    pub fn new(default_agent_id: &str) -> Self {
        Self {
            default_agent_id: default_agent_id.to_string(),
            bindings: Mutex::new(Vec::new()),
        }
    }

    /// Build a session key from the route context.
    fn build_session_key(agent_id: &str, context: &RouteContext) -> String {
        match context.chat_type {
            ChatType::Direct => format!("{}:{}:{}", agent_id, context.channel, context.sender),
            ChatType::Group | ChatType::Channel => {
                let target = context.recipient.as_deref().unwrap_or("unknown");
                format!("{}:{}:{}", agent_id, context.channel, target)
            }
        }
    }

    /// Find the best matching binding for a given context, ordered by priority (lowest first).
    fn find_match(&self, context: &RouteContext) -> Option<(RouteBinding, MatchedBy)> {
        let bindings = self.bindings.lock();
        let mut candidates: Vec<&RouteBinding> = bindings
            .iter()
            .filter(|b| b.channel == context.channel)
            .collect();
        candidates.sort_by_key(|b| b.priority);

        for binding in candidates {
            // Match by guild
            if let Some(ref guild_id) = context.guild_id {
                if binding.pattern == *guild_id {
                    return Some((binding.clone(), MatchedBy::Guild));
                }
            }
            // Match by account
            if let Some(ref account_id) = context.account_id {
                if binding.pattern == *account_id {
                    return Some((binding.clone(), MatchedBy::Account));
                }
            }
            // Match by sender (peer)
            if binding.pattern == context.sender {
                return Some((binding.clone(), MatchedBy::Peer));
            }
            // Match by channel wildcard
            if binding.pattern == "*" {
                return Some((binding.clone(), MatchedBy::Channel));
            }
        }

        None
    }
}

#[async_trait]
impl Router for DefaultRouter {
    async fn resolve_route(&self, context: &RouteContext) -> Result<RouteMatch> {
        let (agent_id, matched_by) = match self.find_match(context) {
            Some((binding, matched_by)) => (binding.agent_id, matched_by),
            None => (self.default_agent_id.clone(), MatchedBy::Default),
        };

        let session_key = Self::build_session_key(&agent_id, context);

        Ok(RouteMatch {
            agent_id,
            session_key,
            matched_by,
        })
    }

    async fn add_binding(&self, binding: RouteBinding) -> Result<()> {
        let mut bindings = self.bindings.lock();
        // Replace existing binding with the same ID
        bindings.retain(|b| b.id != binding.id);
        bindings.push(binding);
        Ok(())
    }

    async fn remove_binding(&self, binding_id: &str) -> Result<()> {
        let mut bindings = self.bindings.lock();
        bindings.retain(|b| b.id != binding_id);
        Ok(())
    }

    async fn list_bindings(&self) -> Result<Vec<RouteBinding>> {
        let bindings = self.bindings.lock();
        Ok(bindings.clone())
    }

    fn name(&self) -> &str {
        "default"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_context(channel: &str, sender: &str) -> RouteContext {
        RouteContext {
            channel: channel.to_string(),
            sender: sender.to_string(),
            recipient: None,
            chat_type: ChatType::Direct,
            account_id: None,
            guild_id: None,
        }
    }

    #[tokio::test]
    async fn resolve_route_returns_default_when_no_bindings() {
        let router = DefaultRouter::new("default-agent");
        let ctx = test_context("telegram", "zeroclaw_user");

        let result = router.resolve_route(&ctx).await.unwrap();
        assert_eq!(result.agent_id, "default-agent");
        assert!(matches!(result.matched_by, MatchedBy::Default));
    }

    #[tokio::test]
    async fn resolve_route_matches_peer_binding() {
        let router = DefaultRouter::new("default-agent");
        router
            .add_binding(RouteBinding {
                id: "b1".to_string(),
                channel: "telegram".to_string(),
                pattern: "zeroclaw_user".to_string(),
                agent_id: "special-agent".to_string(),
                priority: 10,
            })
            .await
            .unwrap();

        let ctx = test_context("telegram", "zeroclaw_user");
        let result = router.resolve_route(&ctx).await.unwrap();
        assert_eq!(result.agent_id, "special-agent");
        assert!(matches!(result.matched_by, MatchedBy::Peer));
    }

    #[tokio::test]
    async fn add_binding_replaces_existing_by_id() {
        let router = DefaultRouter::new("default-agent");
        router
            .add_binding(RouteBinding {
                id: "b1".to_string(),
                channel: "telegram".to_string(),
                pattern: "old".to_string(),
                agent_id: "agent-a".to_string(),
                priority: 10,
            })
            .await
            .unwrap();
        router
            .add_binding(RouteBinding {
                id: "b1".to_string(),
                channel: "telegram".to_string(),
                pattern: "new".to_string(),
                agent_id: "agent-b".to_string(),
                priority: 5,
            })
            .await
            .unwrap();

        let bindings = router.list_bindings().await.unwrap();
        assert_eq!(bindings.len(), 1);
        assert_eq!(bindings[0].agent_id, "agent-b");
    }

    #[tokio::test]
    async fn remove_binding_by_id() {
        let router = DefaultRouter::new("default-agent");
        router
            .add_binding(RouteBinding {
                id: "b1".to_string(),
                channel: "telegram".to_string(),
                pattern: "*".to_string(),
                agent_id: "agent-a".to_string(),
                priority: 10,
            })
            .await
            .unwrap();

        router.remove_binding("b1").await.unwrap();
        let bindings = router.list_bindings().await.unwrap();
        assert!(bindings.is_empty());
    }

    #[tokio::test]
    async fn resolve_route_prefers_lower_priority() {
        let router = DefaultRouter::new("default-agent");
        router
            .add_binding(RouteBinding {
                id: "b1".to_string(),
                channel: "telegram".to_string(),
                pattern: "*".to_string(),
                agent_id: "low-priority".to_string(),
                priority: 100,
            })
            .await
            .unwrap();
        router
            .add_binding(RouteBinding {
                id: "b2".to_string(),
                channel: "telegram".to_string(),
                pattern: "*".to_string(),
                agent_id: "high-priority".to_string(),
                priority: 1,
            })
            .await
            .unwrap();

        let ctx = test_context("telegram", "zeroclaw_user");
        let result = router.resolve_route(&ctx).await.unwrap();
        assert_eq!(result.agent_id, "high-priority");
    }
}
