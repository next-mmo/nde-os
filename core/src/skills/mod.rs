use anyhow::{Context, Result};
use std::path::Path;

/// SKILL.md loader — discovers and parses skill files.
/// Skills are YAML frontmatter + markdown body files that extend agent capabilities.
pub struct SkillLoader {
    search_paths: Vec<std::path::PathBuf>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Skill {
    pub name: String,
    pub description: String,
    pub triggers: Vec<String>,
    pub body: String,
    pub path: String,
}

impl SkillLoader {
    pub fn new(search_paths: Vec<std::path::PathBuf>) -> Self {
        Self { search_paths }
    }

    /// Discover all SKILL.md files across search paths.
    pub fn discover(&self) -> Result<Vec<Skill>> {
        let mut skills = Vec::new();

        for search_path in &self.search_paths {
            if !search_path.exists() {
                continue;
            }

            let pattern = search_path
                .join("**/SKILL.md")
                .to_string_lossy()
                .to_string();
            for entry in glob::glob(&pattern).unwrap_or_else(|_| glob::glob("").unwrap()) {
                if let Ok(path) = entry {
                    match self.parse_skill(&path) {
                        Ok(skill) => skills.push(skill),
                        Err(e) => tracing::warn!("Failed to parse {}: {}", path.display(), e),
                    }
                }
            }
        }

        Ok(skills)
    }

    /// Parse a single SKILL.md file.
    fn parse_skill(&self, path: &Path) -> Result<Skill> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read skill: {}", path.display()))?;

        // Split YAML frontmatter from markdown body
        let (frontmatter, body) = if content.starts_with("---") {
            let parts: Vec<&str> = content.splitn(3, "---").collect();
            if parts.len() >= 3 {
                (parts[1].trim(), parts[2].trim().to_string())
            } else {
                ("", content.clone())
            }
        } else {
            ("", content.clone())
        };

        // Parse frontmatter (simple key: value format)
        let meta: SkillFrontmatter = if !frontmatter.is_empty() {
            let mut name = None;
            let mut description = None;
            let mut triggers = None;

            for line in frontmatter.lines() {
                if let Some((k, v)) = line.split_once(':') {
                    let key = k.trim();
                    let val = v.trim().to_string();
                    match key {
                        "name" => name = Some(val),
                        "description" => description = Some(val),
                        "triggers" => {
                            // Parse comma-separated triggers
                            triggers = Some(val.split(',').map(|s| s.trim().to_string()).collect());
                        }
                        _ => {}
                    }
                }
            }

            SkillFrontmatter {
                name,
                description,
                triggers,
            }
        } else {
            SkillFrontmatter::default()
        };

        Ok(Skill {
            name: meta.name.unwrap_or_else(|| {
                path.parent()
                    .and_then(|p| p.file_name())
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown")
                    .to_string()
            }),
            description: meta.description.unwrap_or_default(),
            triggers: meta.triggers.unwrap_or_default(),
            body,
            path: path.to_string_lossy().into(),
        })
    }

    /// Find skills matching a query string.
    pub fn find_matching(&self, query: &str, skills: &[Skill]) -> Vec<Skill> {
        let lower = query.to_lowercase();
        skills
            .iter()
            .filter(|s| {
                s.triggers.iter().any(|t| lower.contains(&t.to_lowercase()))
                    || s.name.to_lowercase().contains(&lower)
                    || s.description.to_lowercase().contains(&lower)
            })
            .cloned()
            .collect()
    }
}

#[derive(Debug, Default, serde::Deserialize)]
struct SkillFrontmatter {
    name: Option<String>,
    description: Option<String>,
    triggers: Option<Vec<String>>,
}
