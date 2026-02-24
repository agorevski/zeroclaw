use async_trait::async_trait;

use super::traits::{Daemon, DaemonPlatform, DaemonStatus};

/// Stub daemon for manual process management. All operations are no-ops
/// and status always reports `Stopped`.
pub struct ManualDaemon;

#[async_trait]
impl Daemon for ManualDaemon {
    async fn start(&self) -> anyhow::Result<()> {
        Ok(())
    }

    async fn stop(&self) -> anyhow::Result<()> {
        Ok(())
    }

    async fn status(&self) -> anyhow::Result<DaemonStatus> {
        Ok(DaemonStatus::Stopped)
    }

    fn platform(&self) -> DaemonPlatform {
        DaemonPlatform::Manual
    }

    fn name(&self) -> &str {
        "manual"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn manual_daemon_reports_stopped() {
        let d = ManualDaemon;
        assert_eq!(d.status().await.unwrap(), DaemonStatus::Stopped);
        assert_eq!(d.platform(), DaemonPlatform::Manual);
    }

    #[tokio::test]
    async fn start_stop_are_noop() {
        let d = ManualDaemon;
        d.start().await.unwrap();
        d.stop().await.unwrap();
    }
}
