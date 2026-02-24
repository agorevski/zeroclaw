use async_trait::async_trait;
use std::time::Duration;

use super::traits::{Heartbeat, HeartbeatResult};

/// Simple heartbeat that always reports healthy with a configurable interval.
pub struct DefaultHeartbeat {
    interval: Duration,
}

impl DefaultHeartbeat {
    pub fn new(interval: Duration) -> Self {
        Self { interval }
    }
}

#[async_trait]
impl Heartbeat for DefaultHeartbeat {
    async fn tick(&self) -> anyhow::Result<HeartbeatResult> {
        Ok(HeartbeatResult {
            healthy: true,
            checked_at: chrono::Utc::now(),
            details: None,
        })
    }

    fn interval(&self) -> Duration {
        self.interval
    }

    async fn on_wake(&self) -> anyhow::Result<()> {
        Ok(())
    }

    fn name(&self) -> &str {
        "default"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn tick_reports_healthy() {
        let hb = DefaultHeartbeat::new(Duration::from_secs(30));
        let result = hb.tick().await.unwrap();
        assert!(result.healthy);
    }

    #[test]
    fn interval_matches_config() {
        let hb = DefaultHeartbeat::new(Duration::from_secs(60));
        assert_eq!(hb.interval(), Duration::from_secs(60));
    }
}
