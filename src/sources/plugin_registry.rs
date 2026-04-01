//! Plugin discovery from local marketplace directories and the official
//! Anthropic plugin marketplace.
//!
//! Sources:
//! - Local: `~/.claude/plugins/marketplaces/` (cached git repos)
//! - Remote: `anthropics/claude-plugins-official` marketplace.json on GitHub

use std::path::Path;

/// A discovered plugin from a marketplace.
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
    pub category: String,
    pub homepage: String,
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
        if !self.category.is_empty() {
            lines.push(format!("Category: {}", self.category));
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

        // Homepage
        if !self.homepage.is_empty() {
            lines.push("---".to_string());
            lines.push(String::new());
            lines.push(format!("Homepage: {}", self.homepage));
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
                tracing::warn!(
                    "Failed to read plugins dir {}: {}",
                    plugins_path.display(),
                    e
                );
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
                    tracing::warn!(
                        "Failed to read plugin metadata {}: {}",
                        json_path.display(),
                        e
                    );
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

            let name = json["name"].as_str().unwrap_or_default().to_string();
            let description = json["description"].as_str().unwrap_or_default().to_string();

            // Filter by query
            if !query_lower.is_empty()
                && !name.to_lowercase().contains(&query_lower)
                && !description.to_lowercase().contains(&query_lower)
            {
                continue;
            }

            let version = json["version"].as_str().unwrap_or_default().to_string();
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
            let has_mcp =
                plugin_dir.join("mcp").is_dir() || plugin_dir.join("mcp-servers").is_dir();

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
                category: String::new(),
                homepage: String::new(),
            });
        }
    }

    // Sort by name
    results.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(results)
}

// ── Official marketplace (anthropics/claude-plugins-official) ───────────

/// Fetch plugins from the official Anthropic marketplace via marketplace.json.
/// Returns all 100+ plugins in a single API call.
pub fn fetch_official_marketplace() -> Result<Vec<PluginEntry>, String> {
    tracing::info!("Fetching official plugin marketplace");

    let url = "https://raw.githubusercontent.com/anthropics/claude-plugins-official/main/.claude-plugin/marketplace.json";

    let response = ureq::get(url)
        .set("User-Agent", "lazyclaude")
        .call()
        .map_err(|e| format!("Marketplace fetch failed: {e}"))?;

    let entries: Vec<serde_json::Value> = response
        .into_json()
        .map_err(|e| format!("Marketplace JSON parse error: {e}"))?;

    let mut results = Vec::new();

    for entry in &entries {
        let name = match entry["name"].as_str() {
            Some(n) if !n.is_empty() => n.to_string(),
            _ => continue,
        };
        let description = entry["description"]
            .as_str()
            .unwrap_or_default()
            .to_string();
        let category = entry["category"].as_str().unwrap_or_default().to_string();
        let version = entry["version"].as_str().unwrap_or_default().to_string();
        let homepage = entry["homepage"].as_str().unwrap_or_default().to_string();
        let author = entry["author"]["name"]
            .as_str()
            .unwrap_or_default()
            .to_string();

        // Detect components from metadata hints
        let has_skills = entry.get("skills").is_some();
        let has_mcp = entry.get("lspServers").is_some();

        results.push(PluginEntry {
            name,
            description,
            version,
            author,
            marketplace: "claude-plugins-official".to_string(),
            readme: String::new(),
            has_agents: false,
            has_skills,
            has_hooks: false,
            has_commands: false,
            has_mcp,
            category,
            homepage,
        });
    }

    results.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(results)
}

/// Search all plugin sources: local marketplaces + official marketplace.
/// Local results first, then official (deduplicated by name).
pub fn search_all(plugins_dir: &Path, query: &str) -> Result<Vec<PluginEntry>, String> {
    // Local search is fast (filesystem)
    let mut all = search_local(plugins_dir, query).unwrap_or_default();

    // Official marketplace (network)
    match fetch_official_marketplace() {
        Ok(entries) => {
            tracing::info!("Official marketplace: {} plugins", entries.len());
            let query_lower = query.to_lowercase();
            let filtered: Vec<PluginEntry> = if query_lower.is_empty() {
                entries
            } else {
                entries
                    .into_iter()
                    .filter(|e| {
                        e.name.to_lowercase().contains(&query_lower)
                            || e.description.to_lowercase().contains(&query_lower)
                            || e.category.to_lowercase().contains(&query_lower)
                    })
                    .collect()
            };
            all.extend(filtered);
        }
        Err(e) => {
            tracing::warn!("Official marketplace failed: {}", e);
            // Continue with local results only
        }
    }

    // Deduplicate by name (local takes precedence)
    let mut seen = std::collections::HashSet::new();
    all.retain(|entry| seen.insert(entry.name.clone()));

    all.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(all)
}
