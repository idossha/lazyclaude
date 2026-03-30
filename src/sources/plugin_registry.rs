//! Plugin discovery from local marketplace directories.
//!
//! Scans the cached marketplace repos under `~/.claude/plugins/marketplaces/`
//! for available plugins, reading their `plugin.json` metadata and optional README.

use std::path::Path;

/// A discovered plugin from a local marketplace.
#[derive(Clone, Debug, serde::Serialize)]
pub struct PluginEntry {
    pub name: String,
    pub description: String,
    pub version: String,
    pub author: String,
    pub marketplace: String,
    pub readme: String,
    pub has_agents: bool,
    pub has_skills: bool,
    pub has_hooks: bool,
    pub has_commands: bool,
    pub has_mcp: bool,
}

impl PluginEntry {
    /// Generate a rich preview body for display in the detail pane.
    pub fn preview_body(&self) -> String {
        let mut lines = Vec::new();

        lines.push(format!("# {}", self.name));
        lines.push(String::new());

        if !self.description.is_empty() {
            lines.push(self.description.clone());
            lines.push(String::new());
        }

        lines.push("---".to_string());
        lines.push(String::new());

        if !self.version.is_empty() {
            lines.push(format!("Version: {}", self.version));
        }
        if !self.author.is_empty() {
            lines.push(format!("Author: {}", self.author));
        }
        lines.push(format!("Marketplace: {}", self.marketplace));

        lines.push(String::new());

        // Components
        let components = self.component_tags();
        if !components.is_empty() {
            lines.push("## Components".to_string());
            for tag in &components {
                lines.push(format!("- {tag}"));
            }
            lines.push(String::new());
        }

        // Install ID
        lines.push("---".to_string());
        lines.push(String::new());
        lines.push("## Install".to_string());
        lines.push(format!("  {}@{}", self.name, self.marketplace));
        lines.push(String::new());

        // README excerpt
        if !self.readme.is_empty() {
            lines.push("---".to_string());
            lines.push(String::new());
            // Show first ~40 lines of README
            for line in self.readme.lines().take(40) {
                lines.push(line.to_string());
            }
            if self.readme.lines().count() > 40 {
                lines.push(String::new());
                lines.push("  ... (truncated)".to_string());
            }
        }

        lines.join("\n")
    }

    /// Return a list of component type tags this plugin provides.
    pub fn component_tags(&self) -> Vec<&'static str> {
        let mut tags = Vec::new();
        if self.has_agents {
            tags.push("Agents");
        }
        if self.has_skills {
            tags.push("Skills");
        }
        if self.has_hooks {
            tags.push("Hooks");
        }
        if self.has_commands {
            tags.push("Commands");
        }
        if self.has_mcp {
            tags.push("MCP Servers");
        }
        tags
    }

    /// Short component summary for list display (e.g. "agents skills hooks").
    pub fn component_summary(&self) -> String {
        self.component_tags()
            .iter()
            .map(|t| t.to_lowercase())
            .collect::<Vec<_>>()
            .join(" ")
    }
}

/// Search local marketplace directories for plugins matching a query.
pub fn search_local(plugins_dir: &Path, query: &str) -> Result<Vec<PluginEntry>, String> {
    let marketplaces_dir = plugins_dir.join("marketplaces");
    if !marketplaces_dir.exists() {
        return Ok(Vec::new());
    }

    let query_lower = query.to_lowercase();
    let mut results = Vec::new();

    let mp_entries = std::fs::read_dir(&marketplaces_dir)
        .map_err(|e| format!("Cannot read marketplaces dir: {e}"))?;

    for mp_entry in mp_entries.flatten() {
        let mp_name = mp_entry.file_name().to_string_lossy().to_string();
        let plugins_path = mp_entry.path().join("plugins");
        if !plugins_path.is_dir() {
            continue;
        }

        let plugin_dirs = match std::fs::read_dir(&plugins_path) {
            Ok(d) => d,
            Err(e) => {
                tracing::warn!("Failed to read plugins dir {}: {}", plugins_path.display(), e);
                continue;
            }
        };

        for plugin_entry in plugin_dirs.flatten() {
            let plugin_dir = plugin_entry.path();
            if !plugin_dir.is_dir() {
                continue;
            }

            let json_path = plugin_dir.join(".claude-plugin").join("plugin.json");
            if !json_path.exists() {
                continue;
            }

            let json_str = match std::fs::read_to_string(&json_path) {
                Ok(s) => s,
                Err(e) => {
                    tracing::warn!("Failed to read plugin metadata {}: {}", json_path.display(), e);
                    continue;
                }
            };

            let json: serde_json::Value = match serde_json::from_str(&json_str) {
                Ok(v) => v,
                Err(e) => {
                    tracing::warn!("Failed to parse plugin JSON {}: {}", json_path.display(), e);
                    continue;
                }
            };

            let name = json["name"]
                .as_str()
                .unwrap_or_default()
                .to_string();
            let description = json["description"]
                .as_str()
                .unwrap_or_default()
                .to_string();

            // Filter by query
            if !query_lower.is_empty()
                && !name.to_lowercase().contains(&query_lower)
                && !description.to_lowercase().contains(&query_lower)
            {
                continue;
            }

            let version = json["version"]
                .as_str()
                .unwrap_or_default()
                .to_string();
            let author = json["author"]["name"]
                .as_str()
                .unwrap_or_default()
                .to_string();

            // Read README if available
            let readme_path = plugin_dir.join("README.md");
            let readme = std::fs::read_to_string(&readme_path).unwrap_or_default();

            // Detect components by checking for subdirectories
            let has_agents = plugin_dir.join("agents").is_dir();
            let has_skills = plugin_dir.join("skills").is_dir();
            let has_hooks = plugin_dir.join("hooks").is_dir();
            let has_commands = plugin_dir.join("commands").is_dir();
            let has_mcp = plugin_dir.join("mcp").is_dir()
                || plugin_dir.join("mcp-servers").is_dir();

            results.push(PluginEntry {
                name,
                description,
                version,
                author,
                marketplace: mp_name.clone(),
                readme,
                has_agents,
                has_skills,
                has_hooks,
                has_commands,
                has_mcp,
            });
        }
    }

    // Sort by name
    results.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(results)
}
