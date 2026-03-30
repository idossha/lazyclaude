pub mod agents;
pub mod claude_md;
pub mod hooks;
pub mod keybindings;
pub mod mcp;
pub mod mcp_registry;
pub mod memory;
pub mod settings;
pub mod skills;

use std::collections::HashMap;
use std::path::PathBuf;

// ── Data types ──────────────────────────────────────────────────────────

#[derive(Default, Clone, serde::Serialize)]
pub struct SourceData {
    pub memory: MemoryData,
    pub skills: Vec<Skill>,
    pub mcp: McpData,
    pub settings: SettingsData,
    pub hooks: Vec<Hook>,
    pub claude_md: Vec<ClaudeMdFile>,
    pub keybindings: Vec<Keybinding>,
    pub agents: Vec<Agent>,
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
    pub scope: String,
}

#[derive(Default, Clone, serde::Serialize)]
pub struct Agent {
    pub path: PathBuf,
    pub name: String,
    pub description: String,
    pub model: String,
    pub body: String,
    pub dir_name: String,
    pub scope: String,
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
    pub deny: Vec<PermissionRule>,
}

#[derive(Default, Clone, serde::Serialize)]
pub struct PermissionRule {
    pub rule: String,
    pub scope: String,
}

#[derive(Default, Clone, serde::Serialize)]
pub struct Hook {
    pub event: String,
    pub matcher: String,
    pub command: String,
    pub hook_type: String,
    pub scope: String,
}

#[derive(Default, Clone, serde::Serialize)]
pub struct ClaudeMdFile {
    pub path: PathBuf,
    pub name: String,
    pub scope: String,
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
        mcp: mcp::load(paths),
        settings: settings::load(paths),
        hooks: hooks::load(paths),
        claude_md: claude_md::load(paths),
        keybindings: keybindings::load(paths),
        agents: agents::load(paths),
    }
}

// ── Helpers ─────────────────────────────────────────────────────────────

/// Read a JSON file, returning Value::Null on any failure.
pub fn read_json(path: &PathBuf) -> serde_json::Value {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or(serde_json::Value::Null)
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
