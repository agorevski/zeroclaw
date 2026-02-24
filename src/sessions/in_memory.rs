//! In-memory session store implementation.

use anyhow::{bail, Result};
use async_trait::async_trait;
use chrono::Utc;
use parking_lot::Mutex;
use std::collections::HashMap;

use super::traits::{Session, SessionFilter, SessionKey, SessionStore, TranscriptEntry};

/// An in-memory session store backed by a mutex-protected hash map.
pub struct InMemorySessionStore {
    sessions: Mutex<HashMap<SessionKey, Session>>,
    transcripts: Mutex<HashMap<SessionKey, Vec<TranscriptEntry>>>,
}

impl InMemorySessionStore {
    pub fn new() -> Self {
        Self {
            sessions: Mutex::new(HashMap::new()),
            transcripts: Mutex::new(HashMap::new()),
        }
    }
}

impl Default for InMemorySessionStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SessionStore for InMemorySessionStore {
    async fn create(&self, key: &SessionKey) -> Result<Session> {
        let now = Utc::now();
        let session = Session {
            key: key.clone(),
            created_at: now,
            last_activity: now,
            model: None,
            metadata: HashMap::new(),
        };

        let mut sessions = self.sessions.lock();
        sessions.insert(key.clone(), session.clone());
        Ok(session)
    }

    async fn get(&self, key: &SessionKey) -> Result<Option<Session>> {
        let sessions = self.sessions.lock();
        Ok(sessions.get(key).cloned())
    }

    async fn update_activity(&self, key: &SessionKey) -> Result<()> {
        let mut sessions = self.sessions.lock();
        match sessions.get_mut(key) {
            Some(session) => {
                session.last_activity = Utc::now();
                Ok(())
            }
            None => bail!("session not found: {}:{}", key.agent_id, key.context),
        }
    }

    async fn list(&self, filter: &SessionFilter) -> Result<Vec<Session>> {
        let sessions = self.sessions.lock();
        let mut results: Vec<Session> = sessions
            .values()
            .filter(|s| {
                if let Some(ref agent_id) = filter.agent_id {
                    if s.key.agent_id != *agent_id {
                        return false;
                    }
                }
                if let Some(ref since) = filter.since {
                    if s.last_activity < *since {
                        return false;
                    }
                }
                true
            })
            .cloned()
            .collect();

        results.sort_by(|a, b| b.last_activity.cmp(&a.last_activity));

        if let Some(limit) = filter.limit {
            results.truncate(limit);
        }

        Ok(results)
    }

    async fn delete(&self, key: &SessionKey) -> Result<()> {
        let mut sessions = self.sessions.lock();
        sessions.remove(key);
        drop(sessions);

        let mut transcripts = self.transcripts.lock();
        transcripts.remove(key);
        Ok(())
    }

    async fn append_transcript(&self, key: &SessionKey, entry: TranscriptEntry) -> Result<()> {
        let mut transcripts = self.transcripts.lock();
        transcripts
            .entry(key.clone())
            .or_default()
            .push(entry);
        Ok(())
    }

    async fn get_transcript(
        &self,
        key: &SessionKey,
        limit: Option<usize>,
    ) -> Result<Vec<TranscriptEntry>> {
        let transcripts = self.transcripts.lock();
        let entries = match transcripts.get(key) {
            Some(entries) => entries.clone(),
            None => return Ok(Vec::new()),
        };

        match limit {
            Some(n) => {
                let start = entries.len().saturating_sub(n);
                Ok(entries[start..].to_vec())
            }
            None => Ok(entries),
        }
    }

    fn name(&self) -> &str {
        "in_memory"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_key() -> SessionKey {
        SessionKey {
            agent_id: "test-agent".to_string(),
            context: "test-context".to_string(),
        }
    }

    #[tokio::test]
    async fn create_and_get_session() {
        let store = InMemorySessionStore::new();
        let key = test_key();

        let created = store.create(&key).await.unwrap();
        assert_eq!(created.key.agent_id, "test-agent");

        let fetched = store.get(&key).await.unwrap();
        assert!(fetched.is_some());
        assert_eq!(fetched.unwrap().key.context, "test-context");
    }

    #[tokio::test]
    async fn get_returns_none_for_missing_session() {
        let store = InMemorySessionStore::new();
        let key = test_key();

        let result = store.get(&key).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn update_activity_updates_timestamp() {
        let store = InMemorySessionStore::new();
        let key = test_key();
        let created = store.create(&key).await.unwrap();

        store.update_activity(&key).await.unwrap();
        let updated = store.get(&key).await.unwrap().unwrap();
        assert!(updated.last_activity >= created.last_activity);
    }

    #[tokio::test]
    async fn update_activity_fails_for_missing_session() {
        let store = InMemorySessionStore::new();
        let key = test_key();

        let result = store.update_activity(&key).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn delete_removes_session_and_transcript() {
        let store = InMemorySessionStore::new();
        let key = test_key();
        store.create(&key).await.unwrap();
        store
            .append_transcript(
                &key,
                TranscriptEntry {
                    role: "user".to_string(),
                    content: "hello".to_string(),
                    timestamp: Utc::now(),
                    tool_calls: None,
                },
            )
            .await
            .unwrap();

        store.delete(&key).await.unwrap();
        assert!(store.get(&key).await.unwrap().is_none());
        assert!(store.get_transcript(&key, None).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn list_filters_by_agent_id() {
        let store = InMemorySessionStore::new();
        store
            .create(&SessionKey {
                agent_id: "agent-a".to_string(),
                context: "ctx-1".to_string(),
            })
            .await
            .unwrap();
        store
            .create(&SessionKey {
                agent_id: "agent-b".to_string(),
                context: "ctx-2".to_string(),
            })
            .await
            .unwrap();

        let filter = SessionFilter {
            agent_id: Some("agent-a".to_string()),
            ..Default::default()
        };
        let results = store.list(&filter).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].key.agent_id, "agent-a");
    }

    #[tokio::test]
    async fn transcript_append_and_retrieve() {
        let store = InMemorySessionStore::new();
        let key = test_key();

        for i in 0..5 {
            store
                .append_transcript(
                    &key,
                    TranscriptEntry {
                        role: "user".to_string(),
                        content: format!("message {}", i),
                        timestamp: Utc::now(),
                        tool_calls: None,
                    },
                )
                .await
                .unwrap();
        }

        let all = store.get_transcript(&key, None).await.unwrap();
        assert_eq!(all.len(), 5);

        // Limit returns the most recent entries
        let last_two = store.get_transcript(&key, Some(2)).await.unwrap();
        assert_eq!(last_two.len(), 2);
        assert_eq!(last_two[0].content, "message 3");
        assert_eq!(last_two[1].content, "message 4");
    }
}
