pub mod daemon;
pub mod heartbeat;
pub mod traits;
pub mod usage;

pub use daemon::ManualDaemon;
pub use heartbeat::DefaultHeartbeat;
pub use traits::{
    Daemon, DaemonPlatform, DaemonStatus, Heartbeat, HeartbeatResult, UsageBreakdown, UsageEvent,
    UsagePeriod, UsageSummary, UsageTracker,
};
pub use usage::InMemoryUsageTracker;

use std::time::Duration;

pub fn create_daemon() -> Box<dyn Daemon> {
    Box::new(ManualDaemon)
}

pub fn create_heartbeat(interval: Duration) -> Box<dyn Heartbeat> {
    Box::new(DefaultHeartbeat::new(interval))
}

pub fn create_usage_tracker() -> Box<dyn UsageTracker> {
    Box::new(InMemoryUsageTracker::new())
}
