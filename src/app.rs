use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::{widgets::ListState, DefaultTerminal};
use std::time::Duration;

use ccm::config::Paths;
use ccm::sources::{self, SourceData};
use crate::ui;

// ── Section definitions ─────────────────────────────────────────────────

pub struct SectionDef {
    pub label: &'static str,
    pub view: View,
}

impl SectionDef {
    pub fn count(&self, data: &SourceData) -> usize {
        match self.view {
            View::Memory => data.memory.files.len(),
            View::Skills => data.skills.len(),
            View::Mcp => data.mcp.user.len() + data.mcp.project.len(),
            View::Settings => data.settings.permissions.allow.len() + data.settings.permissions.deny.len(),
            View::Hooks => data.hooks.len(),
            View::ClaudeMd => data.claude_md.len(),
            View::Keybindings => data.keybindings.len(),
            View::Agents => data.agents.len(),
            _ => 0,
        }
    }

    pub fn preview<'a>(
        &self,
        data: &'a SourceData,
        filter: &str,
        _width: usize,
    ) -> Vec<ratatui::text::Line<'a>> {
        use ratatui::style::{Color, Modifier, Style};
        use ratatui::text::{Line, Span};

        let fl = filter.to_lowercase();
        let matches = |s: &str| filter.is_empty() || s.to_lowercase().contains(&fl);

        match self.view {
            View::Memory => {
                let mut lines = vec![Line::from("")];
                if !data.memory.project.is_empty() {
                    lines.push(Line::from(vec![
                        Span::styled("  project: ", Style::default().fg(Color::DarkGray)),
                        Span::styled(data.memory.project.as_str(), Style::default().fg(Color::White)),
                    ]));
                    lines.push(Line::from(""));
                }
                for f in &data.memory.files {
                    if !matches(&f.name) { continue; }
                    let badge = format!("[{}]", &f.mem_type[..f.mem_type.len().min(4)]);
                    lines.push(Line::from(vec![
                        Span::styled("  ", Style::default()),
                        Span::styled(f.name.as_str(), Style::default().fg(Color::Green)),
                        Span::styled(format!("  {badge}"), Style::default().fg(Color::Cyan)),
                    ]));
                }
                if data.memory.files.is_empty() {
                    lines.push(Line::from(Span::styled("  No memory files", Style::default().fg(Color::DarkGray))));
                }
                lines
            }
            View::Skills => {
                let mut lines = vec![Line::from("")];
                for s in &data.skills {
                    if !matches(&s.name) { continue; }
                    let (badge, color) = if s.user_invocable { ("[inv]", Color::Green) } else { ("[int]", Color::DarkGray) };
                    lines.push(Line::from(vec![
                        Span::styled("  ", Style::default()),
                        Span::styled(s.name.as_str(), Style::default().fg(color)),
                        Span::styled(format!("  {badge}"), Style::default().fg(color)),
                        Span::styled(format!("  [{}]", s.scope), Style::default().fg(Color::Cyan)),
                    ]));
                }
                if data.skills.is_empty() {
                    lines.push(Line::from(Span::styled("  No skills", Style::default().fg(Color::DarkGray))));
                }
                lines
            }
            View::Mcp => {
                let mut lines = vec![Line::from("")];
                for (label, servers) in &[("User", &data.mcp.user), ("Project", &data.mcp.project)] {
                    if servers.is_empty() { continue; }
                    lines.push(Line::from(Span::styled(format!("  {label}"), Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))));
                    for s in *servers {
                        if !matches(&s.name) { continue; }
                        let color = if s.disabled { Color::DarkGray } else { Color::Green };
                        lines.push(Line::from(vec![
                            Span::styled("    ", Style::default()),
                            Span::styled(s.name.as_str(), Style::default().fg(color)),
                        ]));
                    }
                }
                if data.mcp.user.is_empty() && data.mcp.project.is_empty() {
                    lines.push(Line::from(Span::styled("  No MCP servers", Style::default().fg(Color::DarkGray))));
                }
                lines
            }
            View::Settings => {
                let mut lines = vec![Line::from("")];
                let p = &data.settings.permissions;
                if !p.allow.is_empty() {
                    lines.push(Line::from(Span::styled("  Allow", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))));
                    for r in &p.allow {
                        if !matches(&r.rule) { continue; }
                        lines.push(Line::from(vec![
                            Span::styled("    ", Style::default()),
                            Span::styled(r.rule.as_str(), Style::default().fg(Color::White)),
                            Span::styled(format!("  [{}]", r.scope), Style::default().fg(Color::Cyan)),
                        ]));
                    }
                }
                if !p.deny.is_empty() {
                    lines.push(Line::from(Span::styled("  Deny", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))));
                    for r in &p.deny {
                        if !matches(&r.rule) { continue; }
                        lines.push(Line::from(vec![
                            Span::styled("    ", Style::default()),
                            Span::styled(r.rule.as_str(), Style::default().fg(Color::White)),
                            Span::styled(format!("  [{}]", r.scope), Style::default().fg(Color::Red)),
                        ]));
                    }
                }
                lines
            }
            View::Hooks => {
                let mut lines = vec![Line::from("")];
                let mut cur_event = String::new();
                for h in &data.hooks {
                    if !matches(&h.command) && !matches(&h.event) { continue; }
                    if h.event != cur_event {
                        cur_event = h.event.clone();
                        lines.push(Line::from(Span::styled(format!("  {cur_event}"), Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))));
                    }
                    lines.push(Line::from(vec![
                        Span::styled("    ", Style::default()),
                        Span::styled(h.matcher.as_str(), Style::default().fg(Color::Green)),
                        Span::styled(" -> ", Style::default().fg(Color::DarkGray)),
                        Span::styled(h.command.as_str(), Style::default().fg(Color::White)),
                    ]));
                }
                if data.hooks.is_empty() {
                    lines.push(Line::from(Span::styled("  No hooks", Style::default().fg(Color::DarkGray))));
                }
                lines
            }
            View::ClaudeMd => {
                let mut lines = vec![Line::from("")];
                for f in &data.claude_md {
                    if !matches(&f.name) { continue; }
                    let size = if f.size < 1024 { format!("{} B", f.size) } else { format!("{:.1} KB", f.size as f64 / 1024.0) };
                    lines.push(Line::from(vec![
                        Span::styled("  ", Style::default()),
                        Span::styled(f.name.as_str(), Style::default().fg(Color::Green)),
                        Span::styled(format!("  [{}]  {size}", f.scope), Style::default().fg(Color::DarkGray)),
                    ]));
                }
                if data.claude_md.is_empty() {
                    lines.push(Line::from(Span::styled("  No instruction files", Style::default().fg(Color::DarkGray))));
                }
                lines
            }
            View::Keybindings => {
                let mut lines = vec![Line::from("")];
                for b in &data.keybindings {
                    if !matches(&b.key) && !matches(&b.command) { continue; }
                    lines.push(Line::from(vec![
                        Span::styled("  ", Style::default()),
                        Span::styled(b.key.as_str(), Style::default().fg(Color::Yellow)),
                        Span::styled(" -> ", Style::default().fg(Color::DarkGray)),
                        Span::styled(b.command.as_str(), Style::default().fg(Color::White)),
                    ]));
                }
                if data.keybindings.is_empty() {
                    lines.push(Line::from(Span::styled("  No keybindings", Style::default().fg(Color::DarkGray))));
                }
                lines
            }
            View::Agents => {
                let mut lines = vec![Line::from("")];
                for a in &data.agents {
                    if !matches(&a.name) { continue; }
                    lines.push(Line::from(vec![
                        Span::styled("  ", Style::default()),
                        Span::styled(a.name.as_str(), Style::default().fg(Color::Green)),
                        Span::styled(format!("  [{}]", a.scope), Style::default().fg(Color::Cyan)),
                        if !a.model.is_empty() {
                            Span::styled(format!("  {}", a.model), Style::default().fg(Color::DarkGray))
                        } else {
                            Span::styled("", Style::default())
                        },
                    ]));
                }
                if data.agents.is_empty() {
                    lines.push(Line::from(Span::styled("  No agents", Style::default().fg(Color::DarkGray))));
                }
                lines
            }
            _ => vec![],
        }
    }
}

pub const SECTIONS: &[SectionDef] = &[
    SectionDef { label: "Memory", view: View::Memory },
    SectionDef { label: "Skills", view: View::Skills },
    SectionDef { label: "MCP Servers", view: View::Mcp },
    SectionDef { label: "Settings", view: View::Settings },
    SectionDef { label: "Hooks", view: View::Hooks },
    SectionDef { label: "CLAUDE.md", view: View::ClaudeMd },
    SectionDef { label: "Keybindings", view: View::Keybindings },
    SectionDef { label: "Agents", view: View::Agents },
];

// ── Core types ──────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq)]
pub enum View {
    Dashboard,
    Memory,
    Skills,
    Mcp,
    McpSearch,
    Settings,
    Hooks,
    ClaudeMd,
    Keybindings,
    Agents,
}

#[derive(Clone, Copy, PartialEq)]
pub enum Pane {
    Left,
    Right,
}

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
    InstallMcpFromRegistry { entry: sources::mcp_registry::RegistryEntry, scope: String },
}

// ── App state ───────────────────────────────────────────────────────────

pub struct App {
    pub running: bool,
    pub view: View,
    pub prev_view: Option<View>,
    pub section_index: usize,
    pub focused_pane: Pane,
    pub paths: Paths,
    pub data: SourceData,
    pub filter: String,
    pub show_help: bool,
    pub input_mode: InputMode,
    pub list_state: ListState,
    pub scroll: usize,
    pub message: Option<String>,
    pub registry_results: Vec<sources::mcp_registry::RegistryEntry>,
    /// Set by `e` key — picked up by `run()` which has terminal access.
    pub pending_edit: Option<std::path::PathBuf>,
    /// Maps each list position to an editable file path (None for headers/hints).
    /// Rebuilt every render by `build_items`.
    pub item_paths: Vec<Option<std::path::PathBuf>>,
    /// Maps each list position to file content for preview (None for headers/hints).
    /// Rebuilt every render by `build_items`.
    pub item_bodies: Vec<Option<String>>,
    /// Scroll offset for the content preview pane.
    pub preview_scroll: usize,
}

impl App {
    pub fn with_paths(paths: Paths) -> Self {
        let data = sources::load_all(&paths);
        let mut list_state = ListState::default();
        list_state.select(Some(0));

        Self {
            running: true,
            view: View::Dashboard,
            prev_view: None,
            section_index: 0,
            focused_pane: Pane::Left,
            paths,
            data,
            filter: String::new(),
            show_help: false,
            input_mode: InputMode::Normal,
            list_state,
            scroll: 0,
            message: None,
            registry_results: Vec::new(),
            pending_edit: None,
            item_paths: Vec::new(),
            item_bodies: Vec::new(),
            preview_scroll: 0,
        }
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

    fn handle_key(&mut self, key: KeyEvent) {
        // Dispatch based on input mode first
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
            KeyCode::Tab => {
                if self.view == View::Dashboard {
                    self.focused_pane = match self.focused_pane {
                        Pane::Left => Pane::Right,
                        Pane::Right => Pane::Left,
                    };
                }
            }
            // Preview scroll (shift+j/k)
            KeyCode::Char('J') => { self.preview_scroll = self.preview_scroll.saturating_add(3); }
            KeyCode::Char('K') => { self.preview_scroll = self.preview_scroll.saturating_sub(3); }
            // Navigation
            KeyCode::Char('j') | KeyCode::Down => self.move_down(),
            KeyCode::Char('k') | KeyCode::Up => self.move_up(),
            // MCP search view — handle before generic Enter/Back
            KeyCode::Enter if self.view == View::McpSearch => {
                self.action_install_registry_mcp();
            }
            KeyCode::Backspace | KeyCode::Char('h') if self.view == View::McpSearch => {
                self.view = View::Mcp;
                self.list_state = ListState::default();
                self.list_state.select(Some(0));
                self.registry_results.clear();
            }
            KeyCode::Enter | KeyCode::Char('l') => self.zoom_in(),
            KeyCode::Backspace | KeyCode::Char('h') if self.view != View::Dashboard => self.zoom_out(),
            KeyCode::Esc => {
                if !self.filter.is_empty() {
                    self.filter.clear();
                } else if self.view == View::McpSearch {
                    self.view = View::Mcp;
                    self.list_state = ListState::default();
                    self.list_state.select(Some(0));
                    self.registry_results.clear();
                } else if self.view != View::Dashboard {
                    self.zoom_out();
                }
            }
            // CRUD actions in zoomed views
            KeyCode::Char('a') if self.view != View::Dashboard => self.action_add(),
            KeyCode::Char('d') if self.view != View::Dashboard => self.action_delete(),
            KeyCode::Char('D') if self.view == View::Settings => {
                self.input_mode = InputMode::Input(InputState {
                    prompt: "Deny permission".to_string(),
                    value: String::new(),
                    cursor: 0,
                    purpose: InputPurpose::AddPermission { kind: "deny".to_string() },
                });
            }
            KeyCode::Char('t') if self.view == View::Mcp => self.action_toggle_mcp(),
            KeyCode::Char('s') if self.view == View::Mcp || self.view == View::McpSearch => {
                self.input_mode = InputMode::Input(InputState {
                    prompt: "Search MCP registry (npm)".to_string(),
                    value: String::new(),
                    cursor: 0,
                    purpose: InputPurpose::SearchMcpRegistry,
                });
            }
            KeyCode::Char('e') if matches!(self.view, View::Memory | View::ClaudeMd | View::Agents | View::Skills) => {
                self.action_edit_external();
            }
            _ => {}
        }
    }

    fn handle_input_key(&mut self, key: KeyEvent) {
        // Take ownership of the input state temporarily
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
                            self.view = View::McpSearch;
                            self.list_state = ListState::default();
                            self.list_state.select(Some(0));
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
                    self.view = View::Mcp;
                    self.registry_results.clear();
                }
            }
        }
    }

    // ── Navigation ──────────────────────────────────────────────────────

    fn move_down(&mut self) {
        if self.view == View::Dashboard && self.focused_pane == Pane::Left {
            if self.section_index < SECTIONS.len() - 1 {
                self.section_index += 1;
                self.list_state.select(Some(self.section_index));
            }
        } else {
            let i = self.list_state.selected().unwrap_or(0);
            self.list_state.select(Some(i.saturating_add(1)));
        }
        self.preview_scroll = 0;
    }

    fn move_up(&mut self) {
        if self.view == View::Dashboard && self.focused_pane == Pane::Left {
            self.section_index = self.section_index.saturating_sub(1);
            self.list_state.select(Some(self.section_index));
        } else {
            let i = self.list_state.selected().unwrap_or(0);
            self.list_state.select(Some(i.saturating_sub(1)));
        }
        self.preview_scroll = 0;
    }

    fn zoom_in(&mut self) {
        if self.view == View::Dashboard {
            self.prev_view = Some(View::Dashboard);
            self.view = SECTIONS[self.section_index].view;
            self.list_state = ListState::default();
            self.list_state.select(Some(0));
            self.scroll = 0;
            self.filter.clear();
        }
    }

    fn zoom_out(&mut self) {
        self.view = View::Dashboard;
        self.list_state = ListState::default();
        self.list_state.select(Some(self.section_index));
        self.scroll = 0;
        self.filter.clear();
        self.show_help = false;
    }

    fn refresh(&mut self) {
        self.data = sources::load_all(&self.paths);
        self.message = Some("Refreshed".to_string());
    }

    // ── CRUD actions ────────────────────────────────────────────────────

    fn action_add(&mut self) {
        match self.view {
            View::Settings => {
                self.input_mode = InputMode::Input(InputState {
                    prompt: "Allow permission".to_string(),
                    value: String::new(),
                    cursor: 0,
                    purpose: InputPurpose::AddPermission { kind: "allow".to_string() },
                });
            }
            View::Mcp => {
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
        match self.view {
            View::Settings => {
                // Find which permission the cursor is on
                let idx = self.list_state.selected().unwrap_or(0);
                let perms = &self.data.settings.permissions;
                let allow_header = 1; // "Allow" header line
                let allow_count = perms.allow.len();
                let deny_header_offset = if allow_count > 0 { allow_header + allow_count } else { 0 };

                if idx > 0 && idx <= allow_count {
                    let perm = &perms.allow[idx - 1];
                    self.input_mode = InputMode::Confirm(ConfirmState {
                        message: format!("Delete allow rule '{}'?", perm.rule),
                        purpose: ConfirmPurpose::DeletePermission {
                            scope: perm.scope.clone(),
                            kind: "allow".to_string(),
                            index: idx - 1,
                        },
                    });
                } else if idx > deny_header_offset && idx <= deny_header_offset + perms.deny.len() + 1 {
                    let deny_idx = idx - deny_header_offset - 1;
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
            View::Mcp => {
                // Find which server the cursor is on — need to map list index to actual server
                // For now, simple heuristic based on flat list
                self.message = Some("Position cursor on a server and press 'd'".to_string());
            }
            _ => {}
        }
    }

    fn action_install_registry_mcp(&mut self) {
        let idx = self.list_state.selected().unwrap_or(0);
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

    fn action_toggle_mcp(&mut self) {
        self.message = Some("Toggle: position cursor on server, press 't'".to_string());
    }

    fn action_edit_external(&mut self) {
        let idx = self.list_state.selected().unwrap_or(0);
        // item_paths is rebuilt every render — handles scope headers correctly
        self.pending_edit = self.item_paths.get(idx).and_then(|p| p.clone());
    }
}
