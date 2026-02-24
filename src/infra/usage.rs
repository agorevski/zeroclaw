use async_trait::async_trait;
use chrono::Utc;
use parking_lot::Mutex;
use std::collections::HashMap;

use super::traits::{UsageBreakdown, UsageEvent, UsagePeriod, UsageSummary, UsageTracker};

/// In-memory usage tracker backed by a `parking_lot::Mutex<Vec<UsageEvent>>`.
pub struct InMemoryUsageTracker {
    events: Mutex<Vec<UsageEvent>>,
}

impl InMemoryUsageTracker {
    pub fn new() -> Self {
        Self {
            events: Mutex::new(Vec::new()),
        }
    }

    /// Return the cutoff timestamp for the given period, or `None` for `All`.
    fn cutoff(period: &UsagePeriod) -> Option<chrono::DateTime<Utc>> {
        let now = Utc::now();
        match period {
            UsagePeriod::Hour => Some(now - chrono::Duration::hours(1)),
            UsagePeriod::Day => Some(now - chrono::Duration::days(1)),
            UsagePeriod::Week => Some(now - chrono::Duration::weeks(1)),
            UsagePeriod::Month => Some(now - chrono::Duration::days(30)),
            UsagePeriod::All => None,
        }
    }
}

#[async_trait]
impl UsageTracker for InMemoryUsageTracker {
    async fn record(&self, event: UsageEvent) -> anyhow::Result<()> {
        self.events.lock().push(event);
        Ok(())
    }

    async fn summary(&self, period: &UsagePeriod) -> anyhow::Result<UsageSummary> {
        let cutoff = Self::cutoff(period);
        let events = self.events.lock();
        let mut total_requests: u64 = 0;
        let mut total_input: u64 = 0;
        let mut total_output: u64 = 0;
        let mut total_cost: f64 = 0.0;

        for e in events.iter() {
            if let Some(c) = cutoff {
                if e.timestamp < c {
                    continue;
                }
            }
            total_requests += 1;
            total_input += e.input_tokens;
            total_output += e.output_tokens;
            total_cost += e.cost_usd.unwrap_or(0.0);
        }

        Ok(UsageSummary {
            total_requests,
            total_input_tokens: total_input,
            total_output_tokens: total_output,
            total_cost_usd: total_cost,
            period: period.clone(),
        })
    }

    async fn breakdown(&self, period: &UsagePeriod) -> anyhow::Result<Vec<UsageBreakdown>> {
        let cutoff = Self::cutoff(period);
        let events = self.events.lock();

        // Aggregate by (provider, model)
        let mut map: HashMap<(String, String), (u64, u64, u64, f64)> = HashMap::new();

        for e in events.iter() {
            if let Some(c) = cutoff {
                if e.timestamp < c {
                    continue;
                }
            }
            let entry = map
                .entry((e.provider.clone(), e.model.clone()))
                .or_insert((0, 0, 0, 0.0));
            entry.0 += 1;
            entry.1 += e.input_tokens;
            entry.2 += e.output_tokens;
            entry.3 += e.cost_usd.unwrap_or(0.0);
        }

        let results = map
            .into_iter()
            .map(
                |((provider, model), (requests, input_tokens, output_tokens, cost_usd))| {
                    UsageBreakdown {
                        provider,
                        model,
                        requests,
                        input_tokens,
                        output_tokens,
                        cost_usd,
                    }
                },
            )
            .collect();

        Ok(results)
    }

    fn name(&self) -> &str {
        "in_memory"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_event(provider: &str, model: &str, input: u64, output: u64) -> UsageEvent {
        UsageEvent {
            provider: provider.to_string(),
            model: model.to_string(),
            input_tokens: input,
            output_tokens: output,
            cost_usd: Some(0.01),
            timestamp: Utc::now(),
        }
    }

    #[tokio::test]
    async fn record_and_summary() {
        let tracker = InMemoryUsageTracker::new();
        tracker
            .record(sample_event("openai", "gpt-4", 100, 50))
            .await
            .unwrap();
        tracker
            .record(sample_event("openai", "gpt-4", 200, 100))
            .await
            .unwrap();

        let summary = tracker.summary(&UsagePeriod::All).await.unwrap();
        assert_eq!(summary.total_requests, 2);
        assert_eq!(summary.total_input_tokens, 300);
        assert_eq!(summary.total_output_tokens, 150);
    }

    #[tokio::test]
    async fn breakdown_groups_by_provider_model() {
        let tracker = InMemoryUsageTracker::new();
        tracker
            .record(sample_event("openai", "gpt-4", 100, 50))
            .await
            .unwrap();
        tracker
            .record(sample_event("anthropic", "claude", 200, 100))
            .await
            .unwrap();

        let bd = tracker.breakdown(&UsagePeriod::All).await.unwrap();
        assert_eq!(bd.len(), 2);
    }

    #[tokio::test]
    async fn empty_tracker_returns_zero_summary() {
        let tracker = InMemoryUsageTracker::new();
        let summary = tracker.summary(&UsagePeriod::All).await.unwrap();
        assert_eq!(summary.total_requests, 0);
    }
}
