//! Skill discovery from GitHub repositories.
//!
//! Fetches available skills from:
//! - <https://github.com/anthropics/skills> (official Anthropic skills)
//! - <https://github.com/ComposioHQ/awesome-claude-skills> (community skills)

use std::path::Path;

/// A discovered skill from a GitHub skills repository.
#[derive(Clone, Debug, serde::Serialize)]
pub struct SkillEntry {
    pub name: String,
    pub description: String,
    pub dir_name: String,
    /// Display label for the source repo (e.g., "anthropics/skills")
    pub source: String,
    /// Base URL for raw file downloads (used by install)
    pub raw_base_url: String,
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
        lines.push(format!("Source: github.com/{}", self.source));
        lines.push(format!("Directory: {}", self.dir_name));
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

/// Fetch skills from a GitHub repo that uses the SKILL.md convention.
///
/// `api_url` — GitHub Contents API URL for the directory listing
/// `raw_base` — base URL for raw file downloads (without trailing slash)
/// `source` — display label (e.g., "anthropics/skills")
fn fetch_from_repo(api_url: &str, raw_base: &str, source: &str) -> Result<Vec<SkillEntry>, String> {
    tracing::info!("Fetching skills from {}", source);

    let response = ureq::get(api_url)
        .set("User-Agent", "lazyclaude")
        .set("Accept", "application/vnd.github.v3+json")
        .call()
        .map_err(|e| format!("{source}: GitHub API error: {e}"))?;

    let contents: Vec<GithubContent> = response
        .into_json()
        .map_err(|e| format!("{source}: JSON parse error: {e}"))?;

    let dirs: Vec<String> = contents
        .into_iter()
        .filter(|c| c.content_type == "dir")
        .map(|c| c.name)
        .collect();

    let mut entries = Vec::new();
    for dir_name in &dirs {
        let raw_url = format!("{raw_base}/{dir_name}/SKILL.md");
        let skill_md = match ureq::get(&raw_url).set("User-Agent", "lazyclaude").call() {
            Ok(resp) => resp.into_string().unwrap_or_default(),
            Err(e) => {
                tracing::warn!("Failed to fetch skill {}/{}: {}", source, dir_name, e);
                continue;
            }
        };

        if skill_md.is_empty() {
            continue;
        }

        let (fm, _body) = super::parse_frontmatter(&skill_md);
        let name = fm.get("name").cloned().unwrap_or_else(|| dir_name.clone());
        let description = fm.get("description").cloned().unwrap_or_default();

        entries.push(SkillEntry {
            name,
            description,
            dir_name: dir_name.clone(),
            source: source.to_string(),
            raw_base_url: raw_base.to_string(),
        });
    }

    Ok(entries)
}

/// Fetch available skills from the official anthropics/skills repository.
pub fn fetch_skills() -> Result<Vec<SkillEntry>, String> {
    fetch_from_repo(
        "https://api.github.com/repos/anthropics/skills/contents/skills",
        "https://raw.githubusercontent.com/anthropics/skills/main/skills",
        "anthropics/skills",
    )
}

/// Fetch community skills from ComposioHQ/awesome-claude-skills.
/// Skills are at the repo root (not in a subdirectory).
pub fn fetch_composio_skills() -> Result<Vec<SkillEntry>, String> {
    fetch_from_repo(
        "https://api.github.com/repos/ComposioHQ/awesome-claude-skills/contents/",
        "https://raw.githubusercontent.com/ComposioHQ/awesome-claude-skills/master",
        "ComposioHQ/awesome-claude-skills",
    )
}

/// Fetch skills from all known repositories in parallel and merge results.
/// Deduplicates by dir_name, preferring official over community.
pub fn fetch_all_skills() -> Result<Vec<SkillEntry>, String> {
    let h_official = std::thread::spawn(fetch_skills);
    let h_composio = std::thread::spawn(fetch_composio_skills);

    let mut all = Vec::new();
    let mut errors = Vec::new();

    match h_official.join() {
        Ok(Ok(entries)) => {
            tracing::info!("anthropics/skills: {} results", entries.len());
            all.extend(entries);
        }
        Ok(Err(e)) => {
            tracing::warn!("anthropics/skills failed: {}", e);
            errors.push(e);
        }
        Err(_) => errors.push("anthropics/skills thread panicked".to_string()),
    }

    match h_composio.join() {
        Ok(Ok(entries)) => {
            tracing::info!(
                "ComposioHQ/awesome-claude-skills: {} results",
                entries.len()
            );
            all.extend(entries);
        }
        Ok(Err(e)) => {
            tracing::warn!("ComposioHQ/awesome-claude-skills failed: {}", e);
            errors.push(e);
        }
        Err(_) => errors.push("ComposioHQ thread panicked".to_string()),
    }

    if all.is_empty() && !errors.is_empty() {
        return Err(errors.join("; "));
    }

    // Deduplicate by dir_name (official first)
    let mut seen = std::collections::HashSet::new();
    all.retain(|entry| seen.insert(entry.dir_name.clone()));

    all.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(all)
}

/// Install a skill by downloading its SKILL.md to the user skills directory.
pub fn install_skill(skills_dir: &Path, entry: &SkillEntry) -> Result<(), String> {
    let skill_dir = skills_dir.join(&entry.dir_name);
    std::fs::create_dir_all(&skill_dir).map_err(|e| format!("Cannot create directory: {e}"))?;

    let raw_url = format!("{}/{}/SKILL.md", entry.raw_base_url, entry.dir_name);

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
