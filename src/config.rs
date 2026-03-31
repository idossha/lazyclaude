use std::path::{Path, PathBuf};
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

    /// Construct from a discovered project.
    ///
    /// The project's `name` field is the decoded absolute path to the
    /// project root (e.g. `/Users/foo/myproject`).
    pub fn from_project(claude_dir: &Path, project: &crate::sources::Project) -> Self {
        Self {
            home_dir: dirs::home_dir().unwrap_or_else(|| PathBuf::from("/tmp")),
            claude_dir: claude_dir.to_path_buf(),
            project_root: PathBuf::from(&project.name),
        }
    }

    /// Return a reference to the claude config directory.
    pub fn claude_dir(&self) -> &Path {
        &self.claude_dir
    }

    /// Encode a project path to Claude Code's directory name format.
    /// Claude Code replaces all non-alphanumeric characters with hyphens.
    /// /Users/foo/my_project -> -Users-foo-my-project
    /// /Users/foo/cc.nvim    -> -Users-foo-cc-nvim
    pub fn encode_project_path(path: &str) -> String {
        path.chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '-' {
                    c
                } else {
                    '-'
                }
            })
            .collect()
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

    /// User-level commands directory (~/.claude/commands/)
    pub fn user_commands_dir(&self) -> PathBuf {
        self.claude_dir.join("commands")
    }

    /// Project-level commands directory (<project>/.claude/commands/)
    pub fn project_commands_dir(&self) -> PathBuf {
        self.project_root.join(".claude").join("commands")
    }

    /// Settings file path for a given scope
    pub fn settings_path(&self, scope: &str) -> PathBuf {
        match scope {
            "user" => self.claude_dir.join("settings.json"),
            "project" => self.project_root.join(".claude").join("settings.json"),
            "local" => self
                .project_root
                .join(".claude")
                .join("settings.local.json"),
            _ => self.claude_dir.join("settings.json"),
        }
    }

    /// MCP config path for a given scope.
    /// User-level: ~/.mcp.json (standard cross-tool location)
    /// Project-level: <project>/.mcp.json (project root, not inside .claude/)
    pub fn mcp_path(&self, scope: &str) -> PathBuf {
        match scope {
            "user" => self.home_dir.join(".mcp.json"),
            "project" => self.project_root.join(".mcp.json"),
            _ => self.home_dir.join(".mcp.json"),
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
