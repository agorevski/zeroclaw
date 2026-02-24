use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillCommand {
    pub name: String,
    pub description: String,
    pub handler_hint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InstallMethod {
    Homebrew { package: String },
    Npm { package: String, global: bool },
    Go { package: String },
    Uv { package: String },
    Url { url: String, extract: bool },
    Shell { command: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallSpec {
    pub methods: Vec<InstallMethod>,
    pub verify_command: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillContext {
    pub os: String,
    pub available_binaries: Vec<String>,
    pub env_vars: Vec<String>,
    pub workspace_dir: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SkillSource {
    Bundled,
    Workspace(PathBuf),
    Plugin(String),
}

pub trait Skill: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn source(&self) -> &SkillSource;
    fn is_eligible(&self, context: &SkillContext) -> bool;
    fn prompt_content(&self) -> &str;
    fn commands(&self) -> Vec<SkillCommand>;
    fn required_tools(&self) -> Vec<String>;
    fn install_spec(&self) -> Option<&InstallSpec>;
}

#[async_trait]
pub trait SkillLoader: Send + Sync {
    async fn load_skills(&self, sources: &[SkillSource]) -> Result<Vec<Box<dyn Skill>>>;
    async fn install_skill(&self, skill: &dyn Skill) -> Result<()>;
    fn name(&self) -> &str;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skill_command_serialization_roundtrip() {
        let cmd = SkillCommand {
            name: "test_cmd".to_string(),
            description: "A test command".to_string(),
            handler_hint: Some("handler".to_string()),
        };
        let json = serde_json::to_string(&cmd).unwrap();
        let deserialized: SkillCommand = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.name, "test_cmd");
    }

    #[test]
    fn install_method_variants_serialize() {
        let methods = vec![
            InstallMethod::Homebrew {
                package: "pkg".to_string(),
            },
            InstallMethod::Npm {
                package: "pkg".to_string(),
                global: true,
            },
            InstallMethod::Shell {
                command: "echo ok".to_string(),
            },
        ];
        let json = serde_json::to_string(&methods).unwrap();
        assert!(json.contains("Homebrew"));
    }

    #[test]
    fn skill_source_variants() {
        let bundled = SkillSource::Bundled;
        let workspace = SkillSource::Workspace(PathBuf::from("/tmp"));
        let plugin = SkillSource::Plugin("test".to_string());

        let json = serde_json::to_string(&bundled).unwrap();
        assert!(json.contains("Bundled"));
        let json = serde_json::to_string(&workspace).unwrap();
        assert!(json.contains("Workspace"));
        let json = serde_json::to_string(&plugin).unwrap();
        assert!(json.contains("Plugin"));
    }
}
