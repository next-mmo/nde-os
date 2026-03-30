use crate::llm::ToolDef;
use crate::sandbox::Sandbox;
use crate::skills::SkillLoader;
use crate::tools::Tool;
use anyhow::Result;
use async_trait::async_trait;

/// Lists available skills that the agent can use.
pub struct SkillListTool;

#[async_trait]
impl Tool for SkillListTool {
    fn definition(&self) -> ToolDef {
        ToolDef {
            name: "skill_list".into(),
            description: "List available agent skills (SKILL.md files). Skills extend agent capabilities with specialized knowledge and workflows.".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Optional search query to filter skills by name, description, or trigger keywords"
                    }
                },
                "required": []
            }),
        }
    }

    async fn execute(&self, args: serde_json::Value, sandbox: &Sandbox) -> Result<String> {
        let query = args.get("query").and_then(|v| v.as_str());

        // Search for skills in standard locations
        let search_paths = vec![
            sandbox.root().join("skills"),
            sandbox.root().join(".agents").join("skills"),
            // Also check user-level skill directories
            dirs_skill_path(),
        ];

        let loader = SkillLoader::new(search_paths.into_iter().filter(|p| p.exists()).collect());

        let all_skills = loader.discover()?;

        let skills = if let Some(q) = query {
            loader.find_matching(q, &all_skills)
        } else {
            all_skills
        };

        if skills.is_empty() {
            return Ok(if query.is_some() {
                format!("No skills found matching '{}'", query.unwrap())
            } else {
                "No skills available. Add SKILL.md files to the sandbox skills/ directory.".into()
            });
        }

        let mut output = format!("=== Available Skills ({}) ===\n\n", skills.len());
        for skill in &skills {
            output.push_str(&format!("  📖 {} — {}\n", skill.name, skill.description));
            if !skill.triggers.is_empty() {
                output.push_str(&format!("     triggers: {}\n", skill.triggers.join(", ")));
            }
            output.push_str(&format!("     path: {}\n\n", skill.path));
        }

        Ok(output)
    }
}

/// Get the platform-appropriate user skills directory.
fn dirs_skill_path() -> std::path::PathBuf {
    if cfg!(windows) {
        let userprofile = std::env::var("USERPROFILE").unwrap_or_default();
        std::path::PathBuf::from(userprofile)
            .join(".agents")
            .join("skills")
    } else {
        let home = std::env::var("HOME").unwrap_or_default();
        std::path::PathBuf::from(home)
            .join(".agents")
            .join("skills")
    }
}
