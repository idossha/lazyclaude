//! Skill discovery from the anthropics/skills GitHub repository.
//!
//! Fetches available skills from <https://github.com/anthropics/skills>
//! and supports installing them to the local user skills directory.

use std::path::Path;

/// A discovered skill from the anthropics/skills repository.
#[derive(Clone, Debug, serde::Serialize)]
pub struct SkillEntry {
    pub name: String,
    pub description: String,
    pub dir_name: String,
}

impl SkillEntry {
    /// Generate a preview body for display.
    pub fn preview_body(&self, installed: bool) -> String {
        let mut lines = Vec::new();
        lines.push(format!("# {}", self.name));
        lines.push(String::new());
        if !self.description.is_empty() {
            for line in self.description.lines().take(6) {
                lines.push(line.to_string());
            }
            lines.push(String::new());
        }
        lines.push("---".to_string());
        lines.push(String::new());
        lines.push("Source: github.com/anthropics/skills".to_string());
        lines.push(format!("Directory: skills/{}", self.dir_name));
        if installed {
            lines.push(String::new());
            lines.push("Status: Already installed".to_string());
        }
        lines.push(String::new());
        lines.push("---".to_string());
        lines.push(String::new());
        lines.push("## Install".to_string());
        lines.push(format!(
            "  Downloads SKILL.md to ~/.claude/skills/{}/",
            self.dir_name
        ));
        lines.join("\n")
    }
}

#[derive(serde::Deserialize)]
struct GithubContent {
    name: String,
    #[serde(rename = "type")]
    content_type: String,
}

/// Fetch available skills from the anthropics/skills GitHub repository.
///
/// Lists skill directories under `skills/`, then fetches each `SKILL.md`
/// from raw.githubusercontent.com to extract name and description metadata.
pub fn fetch_skills() -> Result<Vec<SkillEntry>, String> {
    tracing::info!("Fetching skills from GitHub API");
    // List skill directories via GitHub Contents API
    let url = "https://api.github.com/repos/anthropics/skills/contents/skills";
    let response = ureq::get(url)
        .set("User-Agent", "lazyclaude")
        .set("Accept", "application/vnd.github.v3+json")
        .call()
        .map_err(|e| format!("GitHub API error: {e}"))?;

    let contents: Vec<GithubContent> = response
        .into_json()
        .map_err(|e| format!("JSON parse error: {e}"))?;

    let dirs: Vec<String> = contents
        .into_iter()
        .filter(|c| c.content_type == "dir")
        .map(|c| c.name)
        .collect();

    // Fetch SKILL.md for each directory (raw.githubusercontent.com has no API rate limit)
    let mut entries = Vec::new();
    for dir_name in &dirs {
        let raw_url = format!(
            "https://raw.githubusercontent.com/anthropics/skills/main/skills/{}/SKILL.md",
            dir_name
        );
        let skill_md = match ureq::get(&raw_url)
            .set("User-Agent", "lazyclaude")
            .call()
        {
            Ok(resp) => resp.into_string().unwrap_or_default(),
            Err(e) => {
                tracing::warn!("Failed to fetch skill {}: {}", dir_name, e);
                String::new()
            }
        };

        let (fm, _body) = super::parse_frontmatter(&skill_md);
        let name = fm.get("name").cloned().unwrap_or_else(|| dir_name.clone());
        let description = fm.get("description").cloned().unwrap_or_default();

        entries.push(SkillEntry {
            name,
            description,
            dir_name: dir_name.clone(),
        });
    }

    entries.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(entries)
}

/// Install a skill by downloading its SKILL.md to the user skills directory.
pub fn install_skill(skills_dir: &Path, entry: &SkillEntry) -> Result<(), String> {
    let skill_dir = skills_dir.join(&entry.dir_name);
    std::fs::create_dir_all(&skill_dir).map_err(|e| format!("Cannot create directory: {e}"))?;

    let raw_url = format!(
        "https://raw.githubusercontent.com/anthropics/skills/main/skills/{}/SKILL.md",
        entry.dir_name
    );

    let content = ureq::get(&raw_url)
        .set("User-Agent", "lazyclaude")
        .call()
        .map_err(|e| format!("Download failed: {e}"))?
        .into_string()
        .map_err(|e| format!("Read failed: {e}"))?;

    std::fs::write(skill_dir.join("SKILL.md"), content)
        .map_err(|e| format!("Write failed: {e}"))?;

    Ok(())
}
