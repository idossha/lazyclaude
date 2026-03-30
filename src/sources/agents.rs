use crate::config::Paths;
use crate::sources::{parse_frontmatter, Agent, Scope};

pub fn load(paths: &Paths) -> Vec<Agent> {
    let mut agents = Vec::new();

    scan_dir(&mut agents, &paths.user_agents_dir(), Scope::User);
    scan_dir(&mut agents, &paths.project_agents_dir(), Scope::Project);

    agents.sort_by(|a, b| a.name.cmp(&b.name));
    agents
}

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
        let dir_path = entry.path();
        if !dir_path.is_dir() {
            continue;
        }
        let agent_file = dir_path.join("AGENT.md");
        if let Ok(content) = std::fs::read_to_string(&agent_file) {
            let dir_name = dir_path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            let (fm, body) = parse_frontmatter(&content);
            agents.push(Agent {
                path: agent_file,
                name: fm.get("name").cloned().unwrap_or_else(|| dir_name.clone()),
                description: fm.get("description").cloned().unwrap_or_default(),
                model: fm.get("model").cloned().unwrap_or_default(),
                body,
                dir_name,
                scope,
            });
        }
    }
}

/// Delete an agent by removing its parent directory (e.g. agents/my-agent/).
pub fn remove(agent_file: &std::path::Path) -> anyhow::Result<()> {
    if let Some(dir) = agent_file.parent() {
        std::fs::remove_dir_all(dir)?;
    }
    Ok(())
}
