//! Default provider registry implementation.
//!
//! Stores provider factory configurations (name + api key) and creates
//! providers on demand via the existing `create_provider` factory.

use super::create_provider;
use super::traits::{Provider, ProviderRegistry};
use async_trait::async_trait;
use parking_lot::Mutex;

/// Default registry that stores provider names and creates them on demand
/// via the existing `create_provider()` factory.
pub struct DefaultProviderRegistry {
    /// Registered provider entries: (name, optional api_key).
    entries: Mutex<Vec<(String, Option<String>)>>,
    default: Mutex<Option<String>>,
}

impl DefaultProviderRegistry {
    pub fn new() -> Self {
        Self {
            entries: Mutex::new(Vec::new()),
            default: Mutex::new(None),
        }
    }

    /// Set the default provider name for `resolve()` fallback.
    pub fn set_default(&self, name: &str) {
        *self.default.lock() = Some(name.to_string());
    }
}

impl Default for DefaultProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ProviderRegistry for DefaultProviderRegistry {
    async fn register(&self, name: &str, _provider: Box<dyn Provider>) -> anyhow::Result<()> {
        let mut entries = self.entries.lock();
        // Deduplicate by name.
        if !entries.iter().any(|(n, _)| n == name) {
            entries.push((name.to_string(), None));
        }
        Ok(())
    }

    async fn get(&self, name: &str) -> Option<Box<dyn Provider>> {
        let entries = self.entries.lock();
        let entry = entries.iter().find(|(n, _)| n == name)?;
        create_provider(&entry.0, entry.1.as_deref()).ok()
    }

    fn list(&self) -> Vec<String> {
        self.entries.lock().iter().map(|(n, _)| n.clone()).collect()
    }

    async fn resolve(&self, preferred: Option<&str>) -> anyhow::Result<Box<dyn Provider>> {
        // Try preferred provider first.
        if let Some(name) = preferred {
            if let Some(provider) = self.get(name).await {
                return Ok(provider);
            }
        }

        // Try default provider.
        let default_name = self.default.lock().clone();
        if let Some(name) = &default_name {
            if let Some(provider) = self.get(name).await {
                return Ok(provider);
            }
        }

        // Try first registered provider.
        let first = self.entries.lock().first().map(|(n, _)| n.clone());
        if let Some(name) = &first {
            if let Some(provider) = self.get(name).await {
                return Ok(provider);
            }
        }

        anyhow::bail!("No provider available in registry")
    }

    fn name(&self) -> &str {
        "default"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_default_is_empty() {
        let registry = DefaultProviderRegistry::new();
        assert!(registry.list().is_empty());
    }

    #[tokio::test]
    async fn registry_resolve_empty_errors() {
        let registry = DefaultProviderRegistry::new();
        let result = registry.resolve(None).await;
        assert!(result.is_err());
    }
}
