mod actions;
mod event_loop;
mod keys;
mod mouse;
mod navigation;
mod resolve;
mod search;

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::DefaultTerminal;
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Duration;

use lazyclaude::config::Paths;
use lazyclaude::sources::{self, Scope, SourceData};
use crate::ui;

// ── Undo support ────────────────────────────────────────────────────────

pub enum UndoAction {
    DeletedMemory { path: PathBuf, content: String },
    DeletedSkill { dir_path: PathBuf, files: Vec<(PathBuf, Vec<u8>)> },
    DeletedAgent { dir_path: PathBuf, files: Vec<(PathBuf, Vec<u8>)> },
    DeletedMcpServer { scope: Scope, _name: String, config_snapshot: serde_json::Value },
    DeletedPermission { scope: Scope, kind: String, rule: String, _index: usize },
}

// ── Panel definitions ─────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Panel {
    Projects,  // 1
    Config,    // 2 — CLAUDE.md, rules
    Memory,    // 3
    Skills,    // 4
    Agents,    // 5
    Mcp,       // 6
    Settings,  // 7 — permissions, hooks, keybindings combined
    Sessions,  // 8
    Stats,     // 9
    Plugins,   // 0
    Todos,     // -
}

pub const PANELS: &[Panel] = &[
    Panel::Projects,
    Panel::Config,
    Panel::Memory,
    Panel::Skills,
    Panel::Agents,
    Panel::Mcp,
    Panel::Settings,
    Panel::Sessions,
    Panel::Stats,
    Panel::Plugins,
    Panel::Todos,
];

impl Panel {
    pub fn label(&self) -> &'static str {
        match self {
            Panel::Projects => "Projects",
            Panel::Config   => "Config",
            Panel::Memory   => "Memory",
            Panel::Skills   => "Skills",
            Panel::Agents   => "Agents",
            Panel::Mcp      => "MCP",
            Panel::Settings => "Settings",
            Panel::Sessions => "Sessions",
            Panel::Stats    => "Stats",
            Panel::Plugins  => "Plugins",
            Panel::Todos    => "Todos",
        }
    }

    pub fn index(&self) -> usize {
        match self {
            Panel::Projects =>  0,
            Panel::Config   =>  1,
            Panel::Memory   =>  2,
            Panel::Skills   =>  3,
            Panel::Agents   =>  4,
            Panel::Mcp      =>  5,
            Panel::Settings =>  6,
            Panel::Sessions =>  7,
            Panel::Stats    =>  8,
            Panel::Plugins  =>  9,
            Panel::Todos    => 10,
        }
    }

    /// Key label shown in the panel list (1-9, 0, or blank).
    pub fn key_label(&self) -> &'static str {
        match self.index() {
            0 => "1", 1 => "2", 2 => "3", 3 => "4", 4 => "5",
            5 => "6", 6 => "7", 7 => "8", 8 => "9", 9 => "0",
            _ => " ",
        }
    }

    pub fn from_index(i: usize) -> Option<Panel> {
        PANELS.get(i).copied()
    }

    /// Returns the number of rendered rows in the detail list.
    /// This must match what `build_detail_items` produces so j/k can
    /// reach every row.
    pub fn count(&self, app: &App) -> usize {
        match self {
            Panel::Projects => app.projects.len() + 1, // +1 for Global
            Panel::Memory   => app.data.memory.files.len(),
            Panel::Stats    => 0, // custom dashboard, no list
            Panel::Sessions => {
                let n = app.data.sessions.len();
                if n == 0 { 1 } else { n } // "No sessions" placeholder
            }
            Panel::Todos => {
                let n = app.data.todos.len();
                if n == 0 { 1 } else { n }
            }
            Panel::Plugins => {
                let p = &app.data.plugins;
                let mut n = 0;
                if !p.installed.is_empty() { n += 1 + p.installed.len(); }
                if !p.blocked.is_empty() { n += 1 + p.blocked.len(); }
                if !p.marketplaces.is_empty() { n += 1 + p.marketplaces.len(); }
                if n == 0 { 1 } else { n }
            }

            // Panels using scope groups: header + max(entries, 1 "none" hint) per scope
            Panel::Config => {
                scope_group_count(app.data.claude_md.iter().filter(|f| f.scope == Scope::Project).count())
                    + scope_group_count(app.data.claude_md.iter().filter(|f| f.scope == Scope::User).count())
            }
            Panel::Skills => {
                scope_group_count(app.data.skills.iter().filter(|s| s.scope == Scope::Project).count())
                    + scope_group_count(app.data.skills.iter().filter(|s| s.scope == Scope::User).count())
            }
            Panel::Agents => {
                scope_group_count(app.data.agents.iter().filter(|a| a.scope == Scope::Project).count())
                    + scope_group_count(app.data.agents.iter().filter(|a| a.scope == Scope::User).count())
            }
            Panel::Mcp => {
                scope_group_count(app.data.mcp.project.len())
                    + scope_group_count(app.data.mcp.user.len())
            }
            Panel::Settings => {
                let perms = &app.data.settings.permissions;
                let mut n = 0;
                if !perms.allow.is_empty() {
                    n += 1 + perms.allow.len();
                }
                if !perms.deny.is_empty() {
                    n += 1 + perms.deny.len();
                }
                if !app.data.hooks.is_empty() {
                    n += 1;
                    let mut events = std::collections::HashSet::new();
                    for h in &app.data.hooks {
                        if events.insert(&h.event) {
                            n += 1;
                        }
                        n += 1;
                    }
                }
                if !app.data.keybindings.is_empty() {
                    n += 1 + app.data.keybindings.len();
                }
                if let Some(obj) = app.data.settings.effective.as_object() {
                    let general_count = obj.iter()
                        .filter(|(k, _)| *k != "permissions" && *k != "hooks")
                        .count();
                    if general_count > 0 {
                        n += 1 + general_count;
                    }
                }
                n
            }
        }
    }
}

/// Number of rendered rows for a scope group: 1 header + max(entries, 1 "none" hint).
fn scope_group_count(entries: usize) -> usize {
    1 + if entries == 0 { 1 } else { entries }
}

// ── Focus ───────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq)]
pub enum Focus {
    Panels,
    Detail,
    Preview,
}

// ── Input / confirm types ───────────────────────────────────────────────

pub enum InputMode {
    Normal,
    Input(InputState),
    Confirm(ConfirmState),
}

pub struct InputState {
    pub prompt: String,
    pub value: String,
    pub cursor: usize,
    pub purpose: InputPurpose,
}

pub struct ConfirmState {
    pub message: String,
    pub purpose: ConfirmPurpose,
}

pub enum InputPurpose {
    Filter,
    AddPermission { kind: String },
    AddMcpServer { scope: Scope },
    CreateSkill,
    CreateAgent,
}

pub enum ConfirmPurpose {
    DeletePermission { scope: Scope, kind: String, index: usize },
    DeleteMcpServer { scope: Scope, name: String },
    DeleteMemory { path: PathBuf, name: String },
    DeleteSkill { path: PathBuf, name: String },
    DeleteAgent { path: PathBuf, name: String },
    DeletePlugin { name: String },
    UnblockPlugin { name: String },
    InstallMcpFromRegistry { entry: sources::mcp_registry::RegistryEntry, scope: Scope },
    InstallPlugin { entry: sources::plugin_registry::PluginEntry },
    InstallSkillFromRegistry { entry: sources::skills_registry::SkillEntry },
}

// ── Search overlay ──────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq)]
pub enum SearchSource {
    Skills,
    Mcp,
    Plugins,
}

impl SearchSource {
    pub fn label(&self) -> &'static str {
        match self {
            SearchSource::Skills => "Skills Registry (anthropics/skills)",
            SearchSource::Mcp => "MCP Registry (npm)",
            SearchSource::Plugins => "Plugin Marketplace",
        }
    }
}

pub struct SearchOverlay {
    pub source: SearchSource,
    pub all_items: Vec<SearchOverlayItem>,
    pub filter: String,
    pub filter_cursor: usize,
    pub selected: usize,
    pub preview_scroll: usize,
    pub preview_focused: bool,
}

impl SearchOverlay {
    pub fn filtered_indices(&self) -> Vec<usize> {
        use fuzzy_matcher::skim::SkimMatcherV2;
        use fuzzy_matcher::FuzzyMatcher;
        let matcher = SkimMatcherV2::default();
        let mut indices: Vec<usize> = self.all_items
            .iter()
            .enumerate()
            .filter(|(_, item)| {
                self.filter.is_empty()
                    || matcher.fuzzy_match(&item.name, &self.filter).is_some()
                    || matcher.fuzzy_match(&item.description, &self.filter).is_some()
            })
            .map(|(i, _)| i)
            .collect();
        if !self.filter.is_empty() {
            indices.sort_by(|&a, &b| {
                let score_a = matcher.fuzzy_match(&self.all_items[a].name, &self.filter).unwrap_or(0);
                let score_b = matcher.fuzzy_match(&self.all_items[b].name, &self.filter).unwrap_or(0);
                score_b.cmp(&score_a) // Higher score first
            });
        }
        indices
    }

    pub fn selected_item(&self) -> Option<&SearchOverlayItem> {
        let indices = self.filtered_indices();
        indices
            .get(self.selected)
            .and_then(|&i| self.all_items.get(i))
    }
}

#[derive(Clone)]
pub struct SearchOverlayItem {
    pub name: String,
    pub description: String,
    pub extra: String,
    pub installed: bool,
    pub preview: String,
    pub data: SearchItemData,
}

#[derive(Clone)]
pub enum SearchItemData {
    Skill(sources::skills_registry::SkillEntry),
    Mcp(sources::mcp_registry::RegistryEntry),
    Plugin(sources::plugin_registry::PluginEntry),
}

// ── App state ───────────────────────────────────────────────────────────

pub struct App {
    pub running: bool,

    // Panel navigation
    pub active_panel: Panel,
    pub panel_offsets: [usize; 11],
    pub focus: Focus,

    // Project selection
    pub projects: Vec<sources::Project>,
    pub selected_project: usize, // index into projects list; 0 = Global
    pub claude_dir: PathBuf,

    // Data for current project
    pub paths: Paths,
    pub data: SourceData,

    // UI state
    pub filter: String,
    pub show_help: bool,
    pub input_mode: InputMode,
    pub detail_scroll: usize,
    pub message: Option<String>,
    pub message_ttl: u8, // frames remaining to show message

    // Search overlay (unified for Skills, MCP, Plugins)
    pub search_overlay: Option<SearchOverlay>,

    // Background search (network fetches run off the UI thread)
    pub search_receiver: Option<mpsc::Receiver<std::result::Result<Vec<SearchOverlayItem>, String>>>,
    pub search_source_pending: Option<SearchSource>,

    // Skills registry cache (avoid re-fetching from GitHub)
    pub skills_registry_cache: Option<Vec<sources::skills_registry::SkillEntry>>,

    // External edit
    pub pending_edit: Option<PathBuf>,
    pub item_paths: Vec<Option<PathBuf>>,
    pub item_bodies: Vec<Option<String>>,

    // Undo stack for destructive operations (capped at 20)
    pub undo_stack: Vec<UndoAction>,

    // File-system watcher for auto-refresh (held to keep watcher alive)
    #[allow(dead_code)]
    pub watcher: Option<notify::RecommendedWatcher>,
    pub watch_rx: Option<mpsc::Receiver<()>>,
}

impl App {
    pub fn new(paths: Paths) -> Self {
        let claude_dir = paths.claude_dir.clone();
        let projects = sources::load_projects(&paths);
        let data = sources::load_all(&paths);

        // Set up file-system watcher for auto-refresh
        let (watcher, watch_rx) = Self::init_watcher(&paths);

        Self {
            running: true,
            active_panel: Panel::Projects,
            panel_offsets: [0; 11],
            focus: Focus::Panels,
            projects,
            selected_project: 0, // Global
            claude_dir,
            paths,
            data,
            filter: String::new(),
            show_help: false,
            input_mode: InputMode::Normal,
            detail_scroll: 0,
            message: None,
            message_ttl: 0,
            search_overlay: None,
            search_receiver: None,
            search_source_pending: None,
            skills_registry_cache: None,
            pending_edit: None,
            item_paths: Vec::new(),
            item_bodies: Vec::new(),
            undo_stack: Vec::new(),
            watcher,
            watch_rx,
        }
    }

    /// Create a file-system watcher that sends a signal when relevant files change.
    fn init_watcher(paths: &Paths) -> (Option<notify::RecommendedWatcher>, Option<mpsc::Receiver<()>>) {
        use notify::{Watcher, RecursiveMode, Config};

        let (watch_tx, watch_rx) = mpsc::channel();
        let watcher = notify::RecommendedWatcher::new(
            move |res: Result<notify::Event, notify::Error>| {
                if let Ok(event) = res {
                    use notify::EventKind;
                    match event.kind {
                        EventKind::Modify(_) | EventKind::Create(_) | EventKind::Remove(_) => {
                            let _ = watch_tx.send(());
                        }
                        _ => {}
                    }
                }
            },
            Config::default(),
        );

        match watcher {
            Ok(mut w) => {
                // Watch ~/.claude/ recursively (covers memory, skills, agents, settings, etc.)
                let _ = w.watch(paths.claude_dir.as_ref(), RecursiveMode::Recursive);
                // Watch <project>/.claude/ recursively (project-level config)
                let project_claude = paths.project_root.join(".claude");
                if project_claude.exists() {
                    let _ = w.watch(project_claude.as_ref(), RecursiveMode::Recursive);
                }
                // Watch .mcp.json files (user and project level)
                let user_mcp = paths.mcp_path("user");
                if user_mcp.exists() {
                    let _ = w.watch(&user_mcp, RecursiveMode::NonRecursive);
                }
                let project_mcp = paths.mcp_path("project");
                if project_mcp.exists() {
                    let _ = w.watch(&project_mcp, RecursiveMode::NonRecursive);
                }
                (Some(w), Some(watch_rx))
            }
            Err(e) => {
                tracing::warn!("Failed to initialize file watcher: {}", e);
                (None, None)
            }
        }
    }

    /// Keep the old name working so main.rs compiles unchanged.
    pub fn with_paths(paths: Paths) -> Self {
        Self::new(paths)
    }

    /// Set a status message that persists for ~3 seconds (60 frames at 50ms).
    pub(crate) fn set_message(&mut self, msg: String) {
        self.message = Some(msg);
        self.message_ttl = 60;
    }
}
