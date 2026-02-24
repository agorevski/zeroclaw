//! Session storage traits and types for agent conversation state.

use async_trait::async_trait;
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Composite key identifying a unique session.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct SessionKey {
    pub agent_id: String,
    pub context: String,
}

/// A tracked conversation session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub key: SessionKey,
    pub created_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub model: Option<String>,
    pub metadata: HashMap<String, String>,
}

/// A single entry in a session transcript.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptEntry {
    pub role: String,
    pub content: String,
    pub timestamp: DateTime<Utc>,
    pub tool_calls: Option<Vec<String>>,
}

/// Filter criteria for listing sessions.
#[derive(Debug, Clone, Default)]
pub struct SessionFilter {
    pub agent_id: Option<String>,
    pub since: Option<DateTime<Utc>>,
    pub limit: Option<usize>,
}

/// Persistent storage for agent conversation sessions and transcripts.
#[async_trait]
pub trait SessionStore: Send + Sync {
    /// Create a new session. Returns the created session.
    async fn create(&self, key: &SessionKey) -> Result<Session>;

    /// Get an existing session by key, if it exists.
    async fn get(&self, key: &SessionKey) -> Result<Option<Session>>;

    /// Update the last activity timestamp for a session.
    async fn update_activity(&self, key: &SessionKey) -> Result<()>;

    /// List sessions matching the given filter.
    async fn list(&self, filter: &SessionFilter) -> Result<Vec<Session>>;

    /// Delete a session and its transcript.
    async fn delete(&self, key: &SessionKey) -> Result<()>;

    /// Append an entry to the session transcript.
    async fn append_transcript(&self, key: &SessionKey, entry: TranscriptEntry) -> Result<()>;

    /// Retrieve transcript entries for a session, optionally limited.
    async fn get_transcript(
        &self,
        key: &SessionKey,
        limit: Option<usize>,
    ) -> Result<Vec<TranscriptEntry>>;

    /// The name of this session store implementation.
    fn name(&self) -> &str;
}
