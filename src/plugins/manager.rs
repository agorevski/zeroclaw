use anyhow::Result;
use async_trait::async_trait;
use parking_lot::Mutex;

use super::traits::{Hook, HookAction, HookEvent, HookEventType, Plugin, PluginManager};

/// Default in-process plugin manager.
///
/// Stores loaded plugins behind a `Mutex` and dispatches hook events in
/// priority order. `load_plugin` / `unload_plugin` are stubs—dynamic
/// plugin discovery is future work.
pub struct DefaultPluginManager {
    plugins: Mutex<Vec<Box<dyn Plugin>>>,
}

impl DefaultPluginManager {
    pub fn new() -> Self {
        Self {
            plugins: Mutex::new(Vec::new()),
        }
    }
}

impl Default for DefaultPluginManager {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PluginManager for DefaultPluginManager {
    async fn load_plugin(&self, _path: &std::path::Path) -> Result<()> {
        // Dynamic plugin loading is future work.
        Ok(())
    }

    async fn unload_plugin(&self, _name: &str) -> Result<()> {
        // Dynamic plugin unloading is future work.
        Ok(())
    }

    fn list_plugins(&self) -> Vec<&str> {
        // Mutex guard lifetime prevents returning borrowed &str directly;
        // return an empty vec for now (no dynamic loading yet).
        vec![]
    }

    fn get_all_tools(&self) -> Vec<Box<dyn crate::tools::Tool>> {
        let plugins = self.plugins.lock();
        plugins.iter().flat_map(|p| p.tools()).collect()
    }

    fn get_all_hooks(&self, _event_type: &HookEventType) -> Vec<&dyn Hook> {
        // Mutex guard lifetime prevents returning borrowed references;
        // return an empty vec for now.
        vec![]
    }

    async fn dispatch_hook(&self, event: &HookEvent) -> Result<HookAction> {
        let all_hooks: Vec<Box<dyn Hook>> = {
            let plugins = self.plugins.lock();
            plugins.iter().flat_map(|p| p.hooks()).collect()
        };

        let mut relevant: Vec<&dyn Hook> = all_hooks
            .iter()
            .filter(|h| h.event_types().contains(&event.event_type))
            .map(|h| h.as_ref())
            .collect();

        // Lower priority value runs first.
        relevant.sort_by_key(|h| h.priority());

        let mut action = HookAction::Continue;
        for hook in relevant {
            action = hook.execute(event).await?;
            if matches!(action, HookAction::Cancel { .. }) {
                return Ok(action);
            }
        }

        Ok(action)
    }

    fn name(&self) -> &str {
        "default"
    }
}
