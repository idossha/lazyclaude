use crate::config::Paths;
use crate::sources::{parse_frontmatter, Agent, Scope};

pub fn load(paths: &Paths) -> Vec<Agent> {
    let mut agents = Vec::new();

    scan_dir(&mut agents, &paths.user_agents_dir(), Scope::User);
    scan_dir(&mut agents, &paths.project_agents_dir(), Scope::Project);

    agents.sort_by(|a, b| a.name.cmp(&b.name));
    agents
}

/// Scan for flat .md files in the agents directory.
/// Claude Code agents are stored as `<name>.md` files directly in the
/// agents directory (NOT subdirectories with AGENT.md).
fn scan_dir(agents: &mut Vec<Agent>, dir: &std::path::Path, scope: Scope) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) => {
            if e.kind() != std::io::ErrorKind::NotFound {
                tracing::warn!("Failed to read agents dir {}: {}", dir.display(), e);
            }
            return;
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        // Only process .md files (flat files, not directories)
        if path.is_dir() {
            continue;
        }
        if path.extension().map(|e| e == "md").unwrap_or(false) {
            if let Ok(content) = std::fs::read_to_string(&path) {
                let file_stem = path
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                let (fm, body) = parse_frontmatter(&content);
                agents.push(Agent {
                    path,
                    name: fm.get("name").cloned().unwrap_or_else(|| file_stem.clone()),
                    description: fm.get("description").cloned().unwrap_or_default(),
                    model: fm.get("model").cloned().unwrap_or_default(),
                    body,
                    dir_name: file_stem,
                    scope,
                });
            }
        }
    }
}

/// Delete an agent by removing its .md file.
pub fn remove(agent_file: &std::path::Path) -> anyhow::Result<()> {
    std::fs::remove_file(agent_file)?;
    Ok(())
}
