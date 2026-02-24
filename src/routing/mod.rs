//! Message routing — resolves which agent handles a given conversation.

pub mod default;
pub mod traits;

pub use default::DefaultRouter;
pub use traits::{ChatType, MatchedBy, RouteBinding, RouteContext, RouteMatch, Router};

/// Create a default in-memory router with the given fallback agent ID.
pub fn create_router(default_agent_id: &str) -> Box<dyn Router> {
    Box::new(DefaultRouter::new(default_agent_id))
}
