pub mod manager;
pub mod traits;

pub use manager::DefaultPluginManager;
pub use traits::{
    Hook, HookAction, HookEvent, HookEventType, Plugin, PluginCommand, PluginContext,
    PluginManager,
};

/// Create the default plugin manager instance.
pub fn create_plugin_manager() -> Box<dyn PluginManager> {
    Box::new(DefaultPluginManager::new())
}
