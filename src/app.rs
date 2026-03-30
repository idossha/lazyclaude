use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::DefaultTerminal;
use std::path::PathBuf;
use std::time::Duration;

use lazyclaude::config::Paths;
use lazyclaude::sources::{self, SourceData};
use crate::ui;

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
        }
    }

    pub fn index(&self) -> usize {
        match self {
            Panel::Projects => 0,
            Panel::Config   => 1,
            Panel::Memory   => 2,
            Panel::Skills   => 3,
            Panel::Agents   => 4,
            Panel::Mcp      => 5,
            Panel::Settings => 6,
            Panel::Sessions => 7,
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
            Panel::Sessions => {
                let n = app.data.sessions.len();
                if n == 0 { 1 } else { n } // "No sessions" placeholder
            }

            // Panels using scope groups: header + max(entries, 1 "none" hint) per scope
            Panel::Config => {
                scope_group_count(app.data.claude_md.iter().filter(|f| f.scope == "project").count())
                    + scope_group_count(app.data.claude_md.iter().filter(|f| f.scope == "user").count())
            }
            Panel::Skills => {
                scope_group_count(app.data.skills.iter().filter(|s| s.scope == "project").count())
                    + scope_group_count(app.data.skills.iter().filter(|s| s.scope == "user").count())
            }
            Panel::Agents => {
                scope_group_count(app.data.agents.iter().filter(|a| a.scope == "project").count())
                    + scope_group_count(app.data.agents.iter().filter(|a| a.scope == "user").count())
            }
            Panel::Mcp => {
                if app.mcp_search_active {
                    app.registry_results.len()
                } else {
                    scope_group_count(app.data.mcp.project.len())
                        + scope_group_count(app.data.mcp.user.len())
                }
            }
            Panel::Settings => {
                let perms = &app.data.settings.permissions;
                let mut n = 0;
                // Allow section
                if !perms.allow.is_empty() {
                    n += 1 + perms.allow.len(); // header + items
                }
                // Deny section
                if !perms.deny.is_empty() {
                    n += 1 + perms.deny.len();
                }
                // Hooks section
                if !app.data.hooks.is_empty() {
                    n += 1; // "Hooks" header
                    let mut events = std::collections::HashSet::new();
                    for h in &app.data.hooks {
                        if events.insert(&h.event) {
                            n += 1; // event sub-header
                        }
                        n += 1; // hook item
                    }
                }
                // Keybindings section
                if !app.data.keybindings.is_empty() {
                    n += 1 + app.data.keybindings.len();
                }
                // General settings section
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
    AddMcpServer { scope: String },
    SearchMcpRegistry,
}

pub enum ConfirmPurpose {
    DeletePermission { scope: String, kind: String, index: usize },
    DeleteMcpServer { scope: String, name: String },
    InstallMcpFromRegistry { entry: sources::mcp_registry::RegistryEntry, scope: String },
}

// ── App state ───────────────────────────────────────────────────────────

pub struct App {
    pub running: bool,

    // Panel navigation
    pub active_panel: Panel,
    pub panel_offsets: [usize; 8],
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

    // MCP search
    pub registry_results: Vec<sources::mcp_registry::RegistryEntry>,
    pub mcp_search_active: bool,

    // External edit
    pub pending_edit: Option<PathBuf>,
    pub item_paths: Vec<Option<PathBuf>>,
    pub item_bodies: Vec<Option<String>>,
}

impl App {
    pub fn new(paths: Paths) -> Self {
        let claude_dir = paths.claude_dir.clone();
        let projects = sources::load_projects(&paths);
        let data = sources::load_all(&paths);

        Self {
            running: true,
            active_panel: Panel::Projects,
            panel_offsets: [0; 8],
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
            registry_results: Vec::new(),
            mcp_search_active: false,
            pending_edit: None,
            item_paths: Vec::new(),
            item_bodies: Vec::new(),
        }
    }

    /// Keep the old name working so main.rs compiles unchanged.
    pub fn with_paths(paths: Paths) -> Self {
        Self::new(paths)
    }

    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        while self.running {
            // Handle pending editor launch (needs terminal access)
            if let Some(path) = self.pending_edit.take() {
                let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vim".to_string());

                // Leave TUI mode
                crossterm::terminal::disable_raw_mode().ok();
                crossterm::execute!(
                    std::io::stdout(),
                    crossterm::terminal::LeaveAlternateScreen,
                    crossterm::cursor::Show
                )
                .ok();

                // Run editor — blocks until user closes it
                let _ = std::process::Command::new(&editor)
                    .arg(&path)
                    .status();

                // Return to TUI mode
                crossterm::execute!(
                    std::io::stdout(),
                    crossterm::terminal::EnterAlternateScreen,
                    crossterm::cursor::Hide
                )
                .ok();
                crossterm::terminal::enable_raw_mode().ok();

                // Force full redraw — ratatui's buffer is stale after editor
                terminal.clear()?;

                self.refresh();
                continue;
            }

            terminal.draw(|frame| ui::render(frame, self))?;

            if event::poll(Duration::from_millis(50))? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press {
                        self.handle_key(key);
                    }
                }
            }
        }
        Ok(())
    }

    // ── Key dispatch ───────────────────────────────────────────────────

    fn handle_key(&mut self, key: KeyEvent) {
        match &self.input_mode {
            InputMode::Normal => self.handle_normal_key(key),
            InputMode::Input(_) => self.handle_input_key(key),
            InputMode::Confirm(_) => self.handle_confirm_key(key),
        }
    }

    fn handle_normal_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('q') => self.running = false,
            KeyCode::Char('?') => self.show_help = !self.show_help,
            KeyCode::Char('/') => {
                self.input_mode = InputMode::Input(InputState {
                    prompt: "Filter".to_string(),
                    value: self.filter.clone(),
                    cursor: self.filter.len(),
                    purpose: InputPurpose::Filter,
                });
            }
            KeyCode::Char('R') => self.refresh(),

            // Direct panel selection with number keys
            KeyCode::Char(c @ '1'..='8') => {
                let idx = (c as usize) - ('1' as usize);
                if let Some(panel) = Panel::from_index(idx) {
                    self.active_panel = panel;
                    self.detail_scroll = 0;
                }
            }

            // Toggle focus
            KeyCode::Tab => {
                self.focus = match self.focus {
                    Focus::Panels => Focus::Detail,
                    Focus::Detail => Focus::Panels,
                };
            }

            // Detail scroll (Shift+j/k)
            KeyCode::Char('J') => {
                self.detail_scroll = self.detail_scroll.saturating_add(3);
            }
            KeyCode::Char('K') => {
                self.detail_scroll = self.detail_scroll.saturating_sub(3);
            }

            // Navigation: panels (left) vs detail (right)
            KeyCode::Char('j') | KeyCode::Down => {
                if self.focus == Focus::Panels {
                    self.move_panel_down();
                } else {
                    self.move_down();
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if self.focus == Focus::Panels {
                    self.move_panel_up();
                } else {
                    self.move_up();
                }
            }

            // Enter — select project or switch to detail
            KeyCode::Enter => {
                if self.mcp_search_active {
                    self.action_install_registry_mcp();
                } else if self.active_panel == Panel::Projects {
                    self.select_project();
                } else if self.focus == Focus::Panels {
                    self.focus = Focus::Detail;
                }
            }

            // Right — always navigate focus to detail
            KeyCode::Char('l') => {
                if self.focus == Focus::Panels {
                    self.focus = Focus::Detail;
                }
            }

            // Back / left
            KeyCode::Backspace | KeyCode::Char('h') => {
                if self.mcp_search_active {
                    self.mcp_search_active = false;
                    self.registry_results.clear();
                } else if self.focus == Focus::Detail {
                    self.focus = Focus::Panels;
                }
            }

            // Escape
            KeyCode::Esc => {
                if !self.filter.is_empty() {
                    self.filter.clear();
                } else if self.mcp_search_active {
                    self.mcp_search_active = false;
                    self.registry_results.clear();
                } else if self.focus == Focus::Detail {
                    self.focus = Focus::Panels;
                }
            }

            // Edit in external editor
            KeyCode::Char('e') if matches!(
                self.active_panel,
                Panel::Config | Panel::Memory | Panel::Skills | Panel::Agents
            ) => {
                self.action_edit_external();
            }

            // Add action
            KeyCode::Char('a') => self.action_add(),

            // Delete action
            KeyCode::Char('d') => self.action_delete(),

            // Add deny permission
            KeyCode::Char('D') if self.active_panel == Panel::Settings => {
                self.input_mode = InputMode::Input(InputState {
                    prompt: "Deny permission".to_string(),
                    value: String::new(),
                    cursor: 0,
                    purpose: InputPurpose::AddPermission { kind: "deny".to_string() },
                });
            }

            // Search MCP registry
            KeyCode::Char('s') if self.active_panel == Panel::Mcp || self.mcp_search_active => {
                self.input_mode = InputMode::Input(InputState {
                    prompt: "Search MCP registry (npm)".to_string(),
                    value: String::new(),
                    cursor: 0,
                    purpose: InputPurpose::SearchMcpRegistry,
                });
            }

            // Toggle MCP server
            KeyCode::Char('t') if self.active_panel == Panel::Mcp => {
                self.action_toggle_mcp();
            }

            _ => {}
        }
    }

    fn handle_input_key(&mut self, key: KeyEvent) {
        let InputMode::Input(ref mut state) = self.input_mode else { return };

        match key.code {
            KeyCode::Enter => {
                let value = state.value.clone();
                let purpose = std::mem::replace(&mut state.purpose, InputPurpose::Filter);
                self.input_mode = InputMode::Normal;
                self.process_input(value, purpose);
            }
            KeyCode::Esc => {
                self.input_mode = InputMode::Normal;
            }
            KeyCode::Char(c) => {
                state.value.insert(state.cursor, c);
                state.cursor += 1;
            }
            KeyCode::Backspace => {
                if state.cursor > 0 {
                    state.cursor -= 1;
                    state.value.remove(state.cursor);
                }
            }
            KeyCode::Left => {
                if state.cursor > 0 {
                    state.cursor -= 1;
                }
            }
            KeyCode::Right => {
                if state.cursor < state.value.len() {
                    state.cursor += 1;
                }
            }
            _ => {}
        }
    }

    fn handle_confirm_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                let InputMode::Confirm(ref _state) = self.input_mode else { return };
                let purpose = std::mem::replace(
                    &mut (match &mut self.input_mode {
                        InputMode::Confirm(s) => s,
                        _ => unreachable!(),
                    })
                    .purpose,
                    ConfirmPurpose::DeletePermission {
                        scope: String::new(),
                        kind: String::new(),
                        index: 0,
                    },
                );
                self.input_mode = InputMode::Normal;
                self.process_confirm(purpose);
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                self.input_mode = InputMode::Normal;
            }
            _ => {}
        }
    }

    fn process_input(&mut self, value: String, purpose: InputPurpose) {
        match purpose {
            InputPurpose::Filter => {
                self.filter = value;
            }
            InputPurpose::AddPermission { kind } => {
                if !value.is_empty() {
                    if let Err(e) = sources::settings::add_permission(&self.paths, "user", &kind, &value) {
                        self.message = Some(format!("Error: {e}"));
                    } else {
                        self.message = Some(format!("Added {kind}: {value}"));
                        self.data.settings = sources::settings::load(&self.paths);
                    }
                }
            }
            InputPurpose::SearchMcpRegistry => {
                if !value.is_empty() {
                    self.message = Some(format!("Searching npm for '{value}'..."));
                    match sources::mcp_registry::search_npm(&value) {
                        Ok(results) => {
                            let count = results.len();
                            self.registry_results = results;
                            self.mcp_search_active = true;
                            self.message = Some(format!("Found {count} packages"));
                        }
                        Err(e) => {
                            self.message = Some(format!("Search failed: {e}"));
                        }
                    }
                }
            }
            InputPurpose::AddMcpServer { scope } => {
                // Format: name command arg1 arg2 ...
                let parts: Vec<&str> = value.splitn(3, ' ').collect();
                if parts.len() >= 2 {
                    let name = parts[0];
                    let command = parts[1];
                    let args: Vec<String> = if parts.len() > 2 {
                        parts[2].split_whitespace().map(String::from).collect()
                    } else {
                        vec![]
                    };
                    if let Err(e) = sources::mcp::add(&self.paths, &scope, name, command, &args) {
                        self.message = Some(format!("Error: {e}"));
                    } else {
                        self.message = Some(format!("Added MCP server: {name}"));
                        self.data.mcp = sources::mcp::load(&self.paths);
                    }
                }
            }
        }
    }

    fn process_confirm(&mut self, purpose: ConfirmPurpose) {
        match purpose {
            ConfirmPurpose::DeletePermission { scope, kind, index } => {
                if let Err(e) = sources::settings::remove_permission(&self.paths, &scope, &kind, index) {
                    self.message = Some(format!("Error: {e}"));
                } else {
                    self.message = Some("Permission deleted".to_string());
                    self.data.settings = sources::settings::load(&self.paths);
                }
            }
            ConfirmPurpose::DeleteMcpServer { scope, name } => {
                if let Err(e) = sources::mcp::remove(&self.paths, &scope, &name) {
                    self.message = Some(format!("Error: {e}"));
                } else {
                    self.message = Some(format!("Deleted: {name}"));
                    self.data.mcp = sources::mcp::load(&self.paths);
                }
            }
            ConfirmPurpose::InstallMcpFromRegistry { entry, scope } => {
                if let Err(e) = sources::mcp::add(
                    &self.paths,
                    &scope,
                    &entry.name,
                    &entry.install_command,
                    &entry.install_args,
                ) {
                    self.message = Some(format!("Error: {e}"));
                } else {
                    self.message = Some(format!("Installed: {}", entry.name));
                    self.data.mcp = sources::mcp::load(&self.paths);
                    self.mcp_search_active = false;
                    self.registry_results.clear();
                }
            }
        }
    }

    // ── Navigation ──────────────────────────────────────────────────────

    pub fn panel_offset(&self) -> usize {
        self.panel_offsets[self.active_panel.index()]
    }

    fn move_panel_down(&mut self) {
        let cur = self.active_panel.index();
        if cur < PANELS.len() - 1 {
            self.active_panel = PANELS[cur + 1];
            self.detail_scroll = 0;
        }
    }

    fn move_panel_up(&mut self) {
        let cur = self.active_panel.index();
        if cur > 0 {
            self.active_panel = PANELS[cur - 1];
            self.detail_scroll = 0;
        }
    }

    fn move_down(&mut self) {
        let max = self.active_panel.count(self);
        let idx = self.active_panel.index();
        if max > 0 && self.panel_offsets[idx] < max - 1 {
            self.panel_offsets[idx] += 1;
        }
        self.detail_scroll = 0;
    }

    fn move_up(&mut self) {
        let idx = self.active_panel.index();
        self.panel_offsets[idx] = self.panel_offsets[idx].saturating_sub(1);
        self.detail_scroll = 0;
    }

    fn select_project(&mut self) {
        let idx = self.panel_offsets[Panel::Projects.index()];
        if idx == 0 {
            // Global (User) — use default detected paths
            self.selected_project = 0;
            self.paths = Paths::detect();
        } else if let Some(project) = self.projects.get(idx - 1) {
            self.selected_project = idx;
            self.paths = Paths::from_project(&self.claude_dir, project);
        }
        self.reload_data();
        // Reset panel offsets for content panels (keep Projects offset)
        for i in 1..8 {
            self.panel_offsets[i] = 0;
        }
        self.detail_scroll = 0;
        self.message = Some("Project loaded".to_string());
    }

    fn reload_data(&mut self) {
        self.data = sources::load_all(&self.paths);
        // Load sessions for selected project
        if self.selected_project > 0 {
            if let Some(project) = self.projects.get(self.selected_project - 1) {
                self.data.sessions = sources::sessions::load_sessions(&project.dir);
            }
        }
    }

    fn refresh(&mut self) {
        let detect_paths = Paths::detect();
        self.projects = sources::load_projects(&detect_paths);
        self.reload_data();
        self.message = Some("Refreshed".to_string());
    }

    // ── CRUD actions ────────────────────────────────────────────────────

    fn action_add(&mut self) {
        match self.active_panel {
            Panel::Settings => {
                self.input_mode = InputMode::Input(InputState {
                    prompt: "Allow permission".to_string(),
                    value: String::new(),
                    cursor: 0,
                    purpose: InputPurpose::AddPermission { kind: "allow".to_string() },
                });
            }
            Panel::Mcp => {
                self.input_mode = InputMode::Input(InputState {
                    prompt: "Add server (name command args...)".to_string(),
                    value: String::new(),
                    cursor: 0,
                    purpose: InputPurpose::AddMcpServer { scope: "user".to_string() },
                });
            }
            _ => {}
        }
    }

    fn action_delete(&mut self) {
        match self.active_panel {
            Panel::Settings => {
                let idx = self.panel_offset();
                let perms = &self.data.settings.permissions;
                let allow_count = perms.allow.len();

                if idx < allow_count {
                    let perm = &perms.allow[idx];
                    self.input_mode = InputMode::Confirm(ConfirmState {
                        message: format!("Delete allow rule '{}'?", perm.rule),
                        purpose: ConfirmPurpose::DeletePermission {
                            scope: perm.scope.clone(),
                            kind: "allow".to_string(),
                            index: idx,
                        },
                    });
                } else {
                    let deny_idx = idx - allow_count;
                    if deny_idx < perms.deny.len() {
                        let perm = &perms.deny[deny_idx];
                        self.input_mode = InputMode::Confirm(ConfirmState {
                            message: format!("Delete deny rule '{}'?", perm.rule),
                            purpose: ConfirmPurpose::DeletePermission {
                                scope: perm.scope.clone(),
                                kind: "deny".to_string(),
                                index: deny_idx,
                            },
                        });
                    }
                }
            }
            Panel::Mcp => {
                if let Some((scope, name)) = self.resolve_mcp_server() {
                    self.input_mode = InputMode::Confirm(ConfirmState {
                        message: format!("Delete MCP server '{name}' from {scope}?"),
                        purpose: ConfirmPurpose::DeleteMcpServer { scope, name },
                    });
                } else {
                    self.message = Some("No server selected".to_string());
                }
            }
            _ => {}
        }
    }

    fn action_install_registry_mcp(&mut self) {
        let idx = self.panel_offset();
        if let Some(entry) = self.registry_results.get(idx) {
            self.input_mode = InputMode::Confirm(ConfirmState {
                message: format!("Install '{}' to user scope?", entry.name),
                purpose: ConfirmPurpose::InstallMcpFromRegistry {
                    entry: entry.clone(),
                    scope: "user".to_string(),
                },
            });
        }
    }

    /// Resolve the current panel offset to a (scope, server_name) for MCP.
    /// Returns None if the cursor is on a header or "none" hint.
    fn resolve_mcp_server(&self) -> Option<(String, String)> {
        let idx = self.panel_offset();
        // Layout mirrors build_detail_items for Panel::Mcp:
        // 0: "Project (N)" header
        // 1..N: project servers (or 1 "none" hint)
        // N+1: "User (M)" header
        // N+2..end: user servers (or 1 "none" hint)
        let proj_count = self.data.mcp.project.len();
        let proj_items = if proj_count == 0 { 1 } else { proj_count }; // "none" hint counts as 1
        let user_count = self.data.mcp.user.len();

        // Project header is index 0
        // Project items are 1..=proj_items
        // User header is 1 + proj_items
        // User items are 2 + proj_items ..

        if idx == 0 {
            return None; // project header
        }
        if idx <= proj_items {
            if proj_count == 0 { return None; } // "none" hint
            let server = &self.data.mcp.project[idx - 1];
            return Some(("project".to_string(), server.name.clone()));
        }
        let user_header = 1 + proj_items;
        if idx == user_header {
            return None; // user header
        }
        let user_offset = idx - user_header - 1;
        if user_count == 0 { return None; } // "none" hint
        if user_offset < user_count {
            let server = &self.data.mcp.user[user_offset];
            return Some(("user".to_string(), server.name.clone()));
        }
        None
    }

    fn action_toggle_mcp(&mut self) {
        if let Some((scope, name)) = self.resolve_mcp_server() {
            if let Err(e) = sources::mcp::toggle(&self.paths, &scope, &name) {
                self.message = Some(format!("Error: {e}"));
            } else {
                self.message = Some(format!("Toggled: {name}"));
                self.data.mcp = sources::mcp::load(&self.paths);
            }
        } else {
            self.message = Some("No server selected".to_string());
        }
    }

    fn action_edit_external(&mut self) {
        let idx = self.panel_offset();
        // item_paths is rebuilt every render — handles scope headers correctly
        self.pending_edit = self.item_paths.get(idx).and_then(|p| p.clone());
    }
}
