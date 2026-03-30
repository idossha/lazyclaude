use std::path::PathBuf;
use std::process::Command;

/// All resolved paths for Claude Code configuration.
/// Use `Paths::detect()` for runtime, `Paths::new()` for tests/external callers.
#[derive(Clone, Debug)]
pub struct Paths {
    pub home_dir: PathBuf,
    pub claude_dir: PathBuf,
    pub project_root: PathBuf,
}

impl Paths {
    /// Auto-detect paths from the environment.
    pub fn detect() -> Self {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/tmp"));
        Self {
            claude_dir: home.join(".claude"),
            project_root: detect_project_root(),
            home_dir: home,
        }
    }

    /// Construct with explicit paths (for tests, nvim plugin, CLI overrides).
    pub fn new(claude_dir: PathBuf, project_root: PathBuf) -> Self {
        let home_dir = claude_dir
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| claude_dir.clone());
        Self {
            home_dir,
            claude_dir,
            project_root,
        }
    }

    /// Encode a project path to Claude Code's directory name format.
    /// /Users/foo/myproject -> -Users-foo-myproject
    pub fn encode_project_path(path: &str) -> String {
        path.replace('/', "-")
    }

    /// Project config dir: ~/.claude/projects/<encoded-path>
    pub fn project_config_dir(&self) -> PathBuf {
        let encoded = Self::encode_project_path(&self.project_root.to_string_lossy());
        self.claude_dir.join("projects").join(encoded)
    }

    /// Memory directory for the current project
    pub fn memory_dir(&self) -> PathBuf {
        self.project_config_dir().join("memory")
    }

    /// User-level skills directory (~/.claude/skills/)
    pub fn user_skills_dir(&self) -> PathBuf {
        self.claude_dir.join("skills")
    }

    /// Project-level skills directory (<project>/.claude/skills/)
    pub fn project_skills_dir(&self) -> PathBuf {
        self.project_root.join(".claude").join("skills")
    }

    /// User-level agents directory (~/.claude/agents/)
    pub fn user_agents_dir(&self) -> PathBuf {
        self.claude_dir.join("agents")
    }

    /// Project-level agents directory (<project>/.claude/agents/)
    pub fn project_agents_dir(&self) -> PathBuf {
        self.project_root.join(".claude").join("agents")
    }

    /// Settings file path for a given scope
    pub fn settings_path(&self, scope: &str) -> PathBuf {
        match scope {
            "user" => self.claude_dir.join("settings.json"),
            "project" => self.project_root.join(".claude").join("settings.json"),
            "local" => self.project_root.join(".claude").join("settings.local.json"),
            _ => self.claude_dir.join("settings.json"),
        }
    }

    /// MCP config path for a given scope
    pub fn mcp_path(&self, scope: &str) -> PathBuf {
        match scope {
            "user" => self.claude_dir.join(".mcp.json"),
            "project" => self.project_root.join(".claude").join(".mcp.json"),
            _ => self.claude_dir.join(".mcp.json"),
        }
    }

    /// Keybindings file path
    pub fn keybindings_path(&self) -> PathBuf {
        self.claude_dir.join("keybindings.json")
    }
}

// ── Detection helpers ──────────────────────────────────────────────────

fn detect_project_root() -> PathBuf {
    if let Ok(output) = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
    {
        if output.status.success() {
            let s = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !s.is_empty() {
                return PathBuf::from(s);
            }
        }
    }
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}
