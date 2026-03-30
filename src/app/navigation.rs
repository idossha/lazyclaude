use super::*;

impl App {
    pub fn panel_offset(&self) -> usize {
        self.panel_offsets[self.active_panel.index()]
    }

    /// Whether the current panel has a preview pane visible.
    pub fn has_preview(&self) -> bool {
        matches!(
            self.active_panel,
            Panel::Config
                | Panel::Memory
                | Panel::Skills
                | Panel::Agents
                | Panel::Mcp
                | Panel::Plugins
                | Panel::Settings
        )
    }

    pub(crate) fn move_panel_down(&mut self) {
        let cur = self.active_panel.index();
        if cur < PANELS.len() - 1 {
            self.active_panel = PANELS[cur + 1];
            self.detail_scroll = 0;
        }
    }

    pub(crate) fn move_panel_up(&mut self) {
        let cur = self.active_panel.index();
        if cur > 0 {
            self.active_panel = PANELS[cur - 1];
            self.detail_scroll = 0;
        }
    }

    pub(crate) fn move_down(&mut self) {
        let max = self.active_panel.count(self);
        let idx = self.active_panel.index();
        if max > 0 && self.panel_offsets[idx] < max - 1 {
            self.panel_offsets[idx] += 1;
        }
        self.detail_scroll = 0;
    }

    pub(crate) fn move_up(&mut self) {
        let idx = self.active_panel.index();
        self.panel_offsets[idx] = self.panel_offsets[idx].saturating_sub(1);
        self.detail_scroll = 0;
    }

    pub(crate) fn select_project(&mut self) {
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
        for i in 1..self.panel_offsets.len() {
            self.panel_offsets[i] = 0;
        }
        self.detail_scroll = 0;
        self.undo_stack.clear();
        self.set_message("Project loaded".to_string());
    }

    pub(crate) fn reload_data(&mut self) {
        self.data = sources::load_all(&self.paths);
        // Load sessions for selected project
        if self.selected_project > 0 {
            if let Some(project) = self.projects.get(self.selected_project - 1) {
                self.data.sessions = sources::sessions::load_sessions(&project.dir);
            }
        }
    }

    pub(crate) fn refresh(&mut self) {
        let detect_paths = Paths::detect();
        self.projects = sources::load_projects(&detect_paths);
        self.reload_data();
        self.skills_registry_cache = None; // Force re-fetch on next search
        self.set_message("Refreshed".to_string());
    }
}
