#[allow(clippy::module_inception)]
pub mod agent;
pub mod classifier;
pub mod dispatcher;
pub mod loop_;
pub mod memory_loader;
pub mod prompt;
pub mod traits;

#[cfg(test)]
mod tests;

#[allow(unused_imports)]
pub use agent::{Agent, AgentBuilder};
#[allow(unused_imports)]
pub use loop_::{process_message, run};
#[allow(unused_imports)]
pub use traits::{
    AgentInput, AgentOrchestrator, AgentOutput, CompactMessage, ContextCompactor,
    NoopQueryClassifier, PromptBuilder, PromptSectionTrait, QueryClassification, QueryClassifier,
};
