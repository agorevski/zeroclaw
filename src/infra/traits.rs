use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DaemonStatus {
    Running,
    Stopped,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DaemonPlatform {
    Launchd,
    Systemd,
    WindowsService,
    Manual,
}

#[async_trait]
pub trait Daemon: Send + Sync {
    async fn start(&self) -> anyhow::Result<()>;
    async fn stop(&self) -> anyhow::Result<()>;
    async fn status(&self) -> anyhow::Result<DaemonStatus>;
    fn platform(&self) -> DaemonPlatform;
    fn name(&self) -> &str;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatResult {
    pub healthy: bool,
    pub checked_at: DateTime<Utc>,
    pub details: Option<String>,
}

#[async_trait]
pub trait Heartbeat: Send + Sync {
    async fn tick(&self) -> anyhow::Result<HeartbeatResult>;
    fn interval(&self) -> Duration;
    async fn on_wake(&self) -> anyhow::Result<()>;
    fn name(&self) -> &str;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageEvent {
    pub provider: String,
    pub model: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cost_usd: Option<f64>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum UsagePeriod {
    Hour,
    Day,
    Week,
    Month,
    All,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageSummary {
    pub total_requests: u64,
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub total_cost_usd: f64,
    pub period: UsagePeriod,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageBreakdown {
    pub provider: String,
    pub model: String,
    pub requests: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cost_usd: f64,
}

#[async_trait]
pub trait UsageTracker: Send + Sync {
    async fn record(&self, event: UsageEvent) -> anyhow::Result<()>;
    async fn summary(&self, period: &UsagePeriod) -> anyhow::Result<UsageSummary>;
    async fn breakdown(&self, period: &UsagePeriod) -> anyhow::Result<Vec<UsageBreakdown>>;
    fn name(&self) -> &str;
}
