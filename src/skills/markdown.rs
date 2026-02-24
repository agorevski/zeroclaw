use anyhow::Result;
use async_trait::async_trait;

use super::traits::{InstallSpec, Skill, SkillCommand, SkillContext, SkillLoader, SkillSource};

/// A skill loaded from a markdown file.
pub struct MarkdownSkill {
    skill_name: String,
    skill_description: String,
    source: SkillSource,
    content: String,
}

impl MarkdownSkill {
    pub fn new(name: String, description: String, source: SkillSource, content: String) -> Self {
        Self {
            skill_name: name,
            skill_description: description,
            source,
            content,
        }
    }
}

impl Skill for MarkdownSkill {
    fn name(&self) -> &str {
        &self.skill_name
    }

    fn description(&self) -> &str {
        &self.skill_description
    }

    fn source(&self) -> &SkillSource {
        &self.source
    }

    fn is_eligible(&self, _context: &SkillContext) -> bool {
        true
    }

    fn prompt_content(&self) -> &str {
        &self.content
    }

    fn commands(&self) -> Vec<SkillCommand> {
        Vec::new()
    }

    fn required_tools(&self) -> Vec<String> {
        Vec::new()
    }

    fn install_spec(&self) -> Option<&InstallSpec> {
        None
    }
}

/// Default loader that scans workspace paths for markdown skill files.
pub struct DefaultSkillLoader;

#[async_trait]
impl SkillLoader for DefaultSkillLoader {
    async fn load_skills(&self, sources: &[SkillSource]) -> Result<Vec<Box<dyn Skill>>> {
        let mut skills: Vec<Box<dyn Skill>> = Vec::new();

        for source in sources {
            if let SkillSource::Workspace(base_path) = source {
                let skills_dir = base_path.join("skills");
                if skills_dir.is_dir() {
                    let entries = std::fs::read_dir(&skills_dir)?;
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.extension().and_then(|e| e.to_str()) == Some("md") {
                            let content = std::fs::read_to_string(&path)?;
                            let name = path
                                .file_stem()
                                .and_then(|s| s.to_str())
                                .unwrap_or("unknown")
                                .to_string();
                            let description = format!("Skill loaded from {}", path.display());
                            let skill = MarkdownSkill::new(
                                name,
                                description,
                                SkillSource::Workspace(base_path.clone()),
                                content,
                            );
                            skills.push(Box::new(skill));
                        }
                    }
                }
            }
        }

        Ok(skills)
    }

    async fn install_skill(&self, _skill: &dyn Skill) -> Result<()> {
        // No-op stub: markdown skills do not require installation.
        Ok(())
    }

    fn name(&self) -> &str {
        "default"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn markdown_skill_basic_properties() {
        let skill = MarkdownSkill::new(
            "test_skill".to_string(),
            "A test skill".to_string(),
            SkillSource::Bundled,
            "# Test\nSome content".to_string(),
        );

        assert_eq!(skill.name(), "test_skill");
        assert_eq!(skill.description(), "A test skill");
        assert_eq!(skill.prompt_content(), "# Test\nSome content");
        assert!(skill.commands().is_empty());
        assert!(skill.required_tools().is_empty());
        assert!(skill.install_spec().is_none());
    }

    #[test]
    fn markdown_skill_always_eligible() {
        let skill = MarkdownSkill::new(
            "s".to_string(),
            "d".to_string(),
            SkillSource::Bundled,
            String::new(),
        );
        let ctx = SkillContext {
            os: "linux".to_string(),
            available_binaries: Vec::new(),
            env_vars: Vec::new(),
            workspace_dir: PathBuf::from("/tmp"),
        };
        assert!(skill.is_eligible(&ctx));
    }

    #[tokio::test]
    async fn default_loader_handles_missing_dir() {
        let loader = DefaultSkillLoader;
        let sources = vec![SkillSource::Workspace(PathBuf::from("/nonexistent/path"))];
        let result = loader.load_skills(&sources).await.unwrap();
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn default_loader_reads_markdown_files() {
        let tmp = std::env::temp_dir().join("zeroclaw_skill_test");
        let skills_dir = tmp.join("skills");
        std::fs::create_dir_all(&skills_dir).unwrap();
        std::fs::write(skills_dir.join("example.md"), "# Example skill").unwrap();
        std::fs::write(skills_dir.join("not_a_skill.txt"), "ignored").unwrap();

        let loader = DefaultSkillLoader;
        let sources = vec![SkillSource::Workspace(tmp.clone())];
        let result = loader.load_skills(&sources).await.unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name(), "example");
        assert_eq!(result[0].prompt_content(), "# Example skill");

        std::fs::remove_dir_all(&tmp).ok();
    }

    #[tokio::test]
    async fn install_skill_is_noop() {
        let loader = DefaultSkillLoader;
        let skill = MarkdownSkill::new(
            "s".to_string(),
            "d".to_string(),
            SkillSource::Bundled,
            String::new(),
        );
        assert!(loader.install_skill(&skill).await.is_ok());
    }
}
