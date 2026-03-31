pub mod agents;
pub mod claude_md;
pub mod commands;
pub mod hooks;
pub mod keybindings;
pub mod mcp;
pub mod mcp_registry;
pub mod memory;
pub mod plugin_registry;
pub mod plugins;
pub mod projects;
pub mod sessions;
pub mod settings;
pub mod skills;
pub mod skills_registry;
pub mod stats;
pub mod todos;

pub use projects::Project;
pub use sessions::Session;

use std::collections::HashMap;
use std::path::PathBuf;

// ── Scope enum ──────────────────────────────────────────────────────────

/// Compile-time–safe scope for configuration items.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Scope {
    #[default]
    User,
    Project,
    Local,
}

impl Scope {
    pub fn as_str(&self) -> &'static str {
        match self {
            Scope::User => "user",
            Scope::Project => "project",
            Scope::Local => "local",
        }
    }
}

impl std::fmt::Display for Scope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl PartialEq<&str> for Scope {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

// ── Data types ──────────────────────────────────────────────────────────

#[derive(Default, Clone, serde::Serialize)]
pub struct SourceData {
    pub memory: MemoryData,
    pub skills: Vec<Skill>,
    pub commands: Vec<Command>,
    pub mcp: McpData,
    pub settings: SettingsData,
    pub hooks: Vec<Hook>,
    pub claude_md: Vec<ClaudeMdFile>,
    pub keybindings: Vec<Keybinding>,
    pub agents: Vec<Agent>,
    pub sessions: Vec<Session>,
    pub stats: stats::StatsData,
    pub plugins: plugins::PluginsData,
    pub todos: Vec<todos::TodoItem>,
}

#[derive(Default, Clone, serde::Serialize)]
pub struct MemoryData {
    pub files: Vec<MemoryFile>,
    pub project: String,
    pub dir: PathBuf,
}

#[derive(Default, Clone, serde::Serialize)]
pub struct MemoryFile {
    pub path: PathBuf,
    pub name: String,
    pub description: String,
    pub mem_type: String,
    pub body: String,
    pub filename: String,
}

#[derive(Default, Clone, serde::Serialize)]
pub struct Skill {
    pub path: PathBuf,
    pub name: String,
    pub description: String,
    pub user_invocable: bool,
    pub body: String,
    pub dir_name: String,
    pub scope: Scope,
}

#[derive(Default, Clone, serde::Serialize)]
pub struct Agent {
    pub path: PathBuf,
    pub name: String,
    pub description: String,
    pub model: String,
    pub body: String,
    pub dir_name: String,
    pub scope: Scope,
}

#[derive(Default, Clone, serde::Serialize)]
pub struct Command {
    pub path: PathBuf,
    pub name: String,
    pub description: String,
    pub body: String,
    pub file_name: String,
    pub scope: Scope,
}

#[derive(Default, Clone, serde::Serialize)]
pub struct McpData {
    pub user: Vec<McpServer>,
    pub project: Vec<McpServer>,
}

#[derive(Default, Clone, serde::Serialize)]
pub struct McpServer {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
    pub disabled: bool,
}

impl McpServer {
    pub fn preview_body(&self, scope: &str) -> String {
        let mut lines = Vec::new();
        lines.push(format!("# {}", self.name));
        lines.push(String::new());
        let status = if self.disabled { "Disabled" } else { "Enabled" };
        lines.push(format!("Status: {status}"));
        lines.push(format!("Scope: {scope}"));
        lines.push(String::new());
        lines.push("---".to_string());
        lines.push(String::new());
        lines.push(format!("Command: {}", self.command));
        if !self.args.is_empty() {
            lines.push(format!("Args: {}", self.args.join(" ")));
        }
        if !self.env.is_empty() {
            lines.push(String::new());
            lines.push("## Environment".to_string());
            for (k, v) in &self.env {
                let display = if v.chars().count() > 20 {
                    let truncated: String = v.chars().take(17).collect();
                    format!("{truncated}...")
                } else {
                    v.clone()
                };
                lines.push(format!("- {k}: {display}"));
            }
        }
        lines.join("\n")
    }
}

#[derive(Default, Clone, serde::Serialize)]
pub struct SettingsData {
    pub permissions: Permissions,
    pub effective: serde_json::Value,
    pub scopes: SettingsScopes,
}

#[derive(Default, Clone, serde::Serialize)]
pub struct SettingsScopes {
    pub user: serde_json::Value,
    pub project: serde_json::Value,
    pub local_: serde_json::Value,
}

#[derive(Default, Clone, serde::Serialize)]
pub struct Permissions {
    pub allow: Vec<PermissionRule>,
    pub ask: Vec<PermissionRule>,
    pub deny: Vec<PermissionRule>,
}

#[derive(Default, Clone, serde::Serialize)]
pub struct PermissionRule {
    pub rule: String,
    pub scope: Scope,
}

#[derive(Default, Clone, serde::Serialize)]
pub struct Hook {
    pub event: String,
    pub matcher: String,
    pub command: String,
    pub hook_type: String,
    pub scope: Scope,
}

#[derive(Default, Clone, serde::Serialize)]
pub struct ClaudeMdFile {
    pub path: PathBuf,
    pub name: String,
    pub scope: Scope,
    pub file_type: String,
    pub content: String,
    pub size: u64,
}

#[derive(Default, Clone, serde::Serialize)]
pub struct Keybinding {
    pub key: String,
    pub command: String,
    pub context: String,
}

// ── Load all sources ────────────────────────────────────────────────────

pub fn load_all(paths: &crate::config::Paths) -> SourceData {
    SourceData {
        memory: memory::load(paths),
        skills: skills::load(paths),
        commands: commands::load(paths),
        mcp: mcp::load(paths),
        settings: settings::load(paths),
        hooks: hooks::load(paths),
        claude_md: claude_md::load(paths),
        keybindings: keybindings::load(paths),
        agents: agents::load(paths),
        sessions: sessions::load_sessions(&paths.project_config_dir()),
        stats: stats::load(paths),
        plugins: plugins::load(paths),
        todos: todos::load(paths),
    }
}

/// Discover all known projects under the claude config directory.
pub fn load_projects(paths: &crate::config::Paths) -> Vec<Project> {
    projects::discover(paths.claude_dir())
}

// ── Helpers ─────────────────────────────────────────────────────────────

/// Read a JSON file, returning Value::Null on any failure.
pub fn read_json(path: &PathBuf) -> serde_json::Value {
    match std::fs::read_to_string(path) {
        Ok(s) => match serde_json::from_str(&s) {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!("Failed to parse JSON {}: {}", path.display(), e);
                serde_json::Value::Null
            }
        },
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => {
            tracing::warn!("Failed to read {}: {}", path.display(), e);
            serde_json::Value::Null
        }
        Err(_) => serde_json::Value::Null, // File not found is expected, don't log
    }
}

/// Write a JSON value to a file with pretty printing.
pub fn write_json(path: &PathBuf, value: &serde_json::Value) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(value)?;
    std::fs::write(path, json)?;
    Ok(())
}

/// Parse YAML frontmatter from a markdown file.
/// Returns (frontmatter key-value pairs, body).
pub fn parse_frontmatter(content: &str) -> (HashMap<String, String>, String) {
    let mut fm = HashMap::new();

    if !content.starts_with("---") {
        return (fm, content.to_string());
    }

    let rest = &content[3..];
    if let Some(end) = rest.find("\n---") {
        let fm_block = &rest[..end];
        let body = rest[end + 4..].trim_start_matches('\n').to_string();

        for line in fm_block.lines() {
            if let Some((key, val)) = line.split_once(':') {
                fm.insert(key.trim().to_string(), val.trim().to_string());
            }
        }
        (fm, body)
    } else {
        (fm, content.to_string())
    }
}
