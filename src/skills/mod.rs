pub mod markdown;
pub mod traits;

pub use markdown::{DefaultSkillLoader, MarkdownSkill};
pub use traits::{
    InstallMethod, InstallSpec, Skill, SkillCommand, SkillContext, SkillLoader, SkillSource,
};

pub fn create_skill_loader() -> Box<dyn SkillLoader> {
    Box::new(DefaultSkillLoader)
}
