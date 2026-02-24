//! Session management — tracks agent conversation state and transcripts.

pub mod in_memory;
pub mod traits;

pub use in_memory::InMemorySessionStore;
pub use traits::{Session, SessionFilter, SessionKey, SessionStore, TranscriptEntry};

/// Create a default in-memory session store.
pub fn create_session_store() -> Box<dyn SessionStore> {
    Box::new(InMemorySessionStore::new())
}
