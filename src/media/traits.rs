use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaId(pub String);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaMetadata {
    pub filename: Option<String>,
    pub mime_type: Option<String>,
    pub size_bytes: Option<u64>,
    pub source_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaEntry {
    pub id: MediaId,
    pub path: PathBuf,
    pub metadata: MediaMetadata,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone)]
pub struct FetchOptions {
    pub max_size_bytes: u64,
    pub timeout: std::time::Duration,
    pub allowed_mime_types: Option<Vec<String>>,
}

#[derive(Debug, Clone)]
pub struct FetchedMedia {
    pub data: Vec<u8>,
    pub mime_type: String,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaToken {
    pub source: String,
    pub is_url: bool,
}

#[async_trait]
pub trait MediaStore: Send + Sync {
    async fn store(&self, data: &[u8], metadata: MediaMetadata) -> anyhow::Result<MediaEntry>;
    async fn get(&self, id: &MediaId) -> anyhow::Result<Option<MediaEntry>>;
    async fn delete(&self, id: &MediaId) -> anyhow::Result<()>;
    async fn list(&self) -> anyhow::Result<Vec<MediaEntry>>;
    fn name(&self) -> &str;
}

#[async_trait]
pub trait MediaFetcher: Send + Sync {
    async fn fetch(&self, url: &str, options: &FetchOptions) -> anyhow::Result<FetchedMedia>;
    fn name(&self) -> &str;
}

pub trait MediaParser: Send + Sync {
    fn parse_tokens(&self, text: &str) -> Vec<MediaToken>;
    fn name(&self) -> &str;
}
