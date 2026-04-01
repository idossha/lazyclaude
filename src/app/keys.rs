use super::*;

impl App {
    // ── Key dispatch ───────────────────────────────────────────────────

    pub(crate) fn handle_key(&mut self, key: KeyEvent) {
        match &self.input_mode {
            InputMode::Normal => {
                if self.search_overlay.is_some() {
                    self.handle_search_overlay_key(key);
                } else {
                    self.handle_normal_key(key);
                }
            }
            InputMode::Input(_) => self.handle_input_key(key),
            InputMode::Confirm(_) => self.handle_confirm_key(key),
        }
    }

    fn handle_normal_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('q') => self.running = false,
            KeyCode::Char('?') => {
                self.show_help = !self.show_help;
                self.detail_scroll = 0;
            }
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
            KeyCode::Char(c @ '1'..='9') => {
                let idx = (c as usize) - ('1' as usize);
                if let Some(panel) = Panel::from_index(idx) {
                    tracing::debug!("Panel switched to {}", panel.label());
                    self.active_panel = panel;
                    self.detail_scroll = 0;
                }
            }
            KeyCode::Char('0') => {
                if let Some(panel) = Panel::from_index(9) {
                    tracing::debug!("Panel switched to {}", panel.label());
                    self.active_panel = panel;
                    self.detail_scroll = 0;
                }
            }

            // Toggle focus: Panels -> Detail -> Preview (if available) -> Panels
            KeyCode::Tab => {
                self.focus = match self.focus {
                    Focus::Panels => Focus::Detail,
                    Focus::Detail => {
                        if self.has_preview() {
                            Focus::Preview
                        } else {
                            Focus::Panels
                        }
                    }
                    Focus::Preview => Focus::Panels,
                };
            }

            // Detail scroll (Shift+j/k) — always works as shortcut
            KeyCode::Char('J') => {
                self.detail_scroll = self.detail_scroll.saturating_add(3);
            }
            KeyCode::Char('K') => {
                self.detail_scroll = self.detail_scroll.saturating_sub(3);
            }

            // Navigation: panels (left) vs detail (right) vs preview scroll
            KeyCode::Char('j') | KeyCode::Down => {
                if self.show_help && self.focus != Focus::Panels {
                    self.detail_scroll = self.detail_scroll.saturating_add(1);
                } else {
                    match self.focus {
                        Focus::Panels => self.move_panel_down(),
                        Focus::Detail => self.move_down(),
                        Focus::Preview => {
                            self.detail_scroll = self.detail_scroll.saturating_add(1);
                        }
                    }
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if self.show_help && self.focus != Focus::Panels {
                    self.detail_scroll = self.detail_scroll.saturating_sub(1);
                } else {
                    match self.focus {
                        Focus::Panels => self.move_panel_up(),
                        Focus::Detail => self.move_up(),
                        Focus::Preview => {
                            self.detail_scroll = self.detail_scroll.saturating_sub(1);
                        }
                    }
                }
            }

            // Enter — select project or move into preview
            KeyCode::Enter => {
                if self.active_panel == Panel::Projects && self.focus != Focus::Preview {
                    self.select_project();
                } else if self.focus == Focus::Panels {
                    self.focus = Focus::Detail;
                } else if self.focus == Focus::Detail && self.has_preview() {
                    self.focus = Focus::Preview;
                }
            }

            // Right — navigate focus rightward: Panels -> Detail -> Preview
            KeyCode::Char('l') => match self.focus {
                Focus::Panels => self.focus = Focus::Detail,
                Focus::Detail if self.has_preview() => self.focus = Focus::Preview,
                _ => {}
            },

            // Back / left — navigate focus leftward: Preview -> Detail -> Panels
            KeyCode::Backspace | KeyCode::Char('h') => match self.focus {
                Focus::Preview => self.focus = Focus::Detail,
                Focus::Detail => self.focus = Focus::Panels,
                Focus::Panels => {}
            },

            // Escape
            KeyCode::Esc => {
                if self.focus == Focus::Preview {
                    self.focus = Focus::Detail;
                } else if !self.filter.is_empty() {
                    self.filter.clear();
                } else if self.focus == Focus::Detail {
                    self.focus = Focus::Panels;
                }
            }

            // Edit in external editor
            KeyCode::Char('e')
                if matches!(
                    self.active_panel,
                    Panel::Config | Panel::Memory | Panel::Skills | Panel::Agents
                ) =>
            {
                self.action_edit_external();
            }

            // Add action
            KeyCode::Char('a') => self.action_add(),

            // Delete action
            KeyCode::Char('d') => self.action_delete(),

            // Deny selected permission
            KeyCode::Char('D') if self.active_panel == Panel::Settings => {
                if let Some((kind, scope, index, rule)) = self.resolve_permission() {
                    if kind == "deny" {
                        self.set_message("Already in deny list".to_string());
                    } else {
                        self.input_mode = InputMode::Confirm(Box::new(ConfirmState {
                            message: format!("Deny permission '{rule}'?"),
                            purpose: ConfirmPurpose::DenyPermission {
                                scope,
                                kind,
                                index,
                                rule,
                            },
                        }));
                    }
                }
            }

            // Search overlay (Skills, MCP, Plugins)
            KeyCode::Char('s')
                if matches!(
                    self.active_panel,
                    Panel::Skills | Panel::Mcp | Panel::Plugins
                ) =>
            {
                self.open_search_overlay();
            }

            // Toggle MCP server
            KeyCode::Char('t') if self.active_panel == Panel::Mcp => {
                self.action_toggle_mcp();
            }

            // Export panel to clipboard as JSON
            KeyCode::Char('x') => self.action_export(),

            // Copy to clipboard
            KeyCode::Char('y') => self.action_copy(),

            // Undo last delete
            KeyCode::Char('u') => self.action_undo(),

            _ => {}
        }
    }

    fn handle_input_key(&mut self, key: KeyEvent) {
        let InputMode::Input(ref mut state) = self.input_mode else {
            return;
        };

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
                let byte_pos = state
                    .value
                    .char_indices()
                    .nth(state.cursor)
                    .map(|(i, _)| i)
                    .unwrap_or(state.value.len());
                state.value.insert(byte_pos, c);
                state.cursor += 1;
            }
            KeyCode::Backspace => {
                if state.cursor > 0 {
                    state.cursor -= 1;
                    let byte_pos = state
                        .value
                        .char_indices()
                        .nth(state.cursor)
                        .map(|(i, _)| i)
                        .unwrap_or(state.value.len());
                    state.value.remove(byte_pos);
                }
            }
            KeyCode::Left => {
                if state.cursor > 0 {
                    state.cursor -= 1;
                }
            }
            KeyCode::Right => {
                if state.cursor < state.value.chars().count() {
                    state.cursor += 1;
                }
            }
            _ => {}
        }
    }

    fn handle_confirm_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                let old_mode = std::mem::replace(&mut self.input_mode, InputMode::Normal);
                if let InputMode::Confirm(state) = old_mode {
                    self.process_confirm(state.purpose);
                }
            }
            KeyCode::Char('p') | KeyCode::Char('P') => {
                let old_mode = std::mem::replace(&mut self.input_mode, InputMode::Normal);
                if let InputMode::Confirm(state) = old_mode {
                    match state.purpose {
                        ConfirmPurpose::InstallMcpFromRegistry { entry, .. } => {
                            self.process_confirm(ConfirmPurpose::InstallMcpFromRegistry {
                                entry,
                                scope: Scope::Project,
                            });
                        }
                        ConfirmPurpose::InstallSkillFromRegistry { entry } => {
                            let skills_dir = self.paths.project_skills_dir();
                            match sources::skills_registry::install_skill(&skills_dir, &entry) {
                                Ok(()) => {
                                    self.set_message(format!(
                                        "Installed to project: {}",
                                        entry.name
                                    ));
                                    self.data.skills = sources::skills::load(&self.paths);
                                    self.search_overlay = None;
                                }
                                Err(e) => self.set_message(format!("Error: {e}")),
                            }
                        }
                        other => {
                            // Not a scope-selectable confirm, put it back
                            self.input_mode = InputMode::Confirm(Box::new(ConfirmState {
                                message: String::new(),
                                purpose: other,
                            }));
                        }
                    }
                }
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                self.input_mode = InputMode::Normal;
            }
            _ => {}
        }
    }

    pub(crate) fn handle_search_overlay_key(&mut self, key: KeyEvent) {
        let Some(overlay) = self.search_overlay.as_mut() else {
            return;
        };

        // When preview is focused, j/k scroll the preview
        if overlay.preview_focused {
            match key.code {
                KeyCode::Char('j') | KeyCode::Down => {
                    overlay.preview_scroll = overlay.preview_scroll.saturating_add(1);
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    overlay.preview_scroll = overlay.preview_scroll.saturating_sub(1);
                }
                KeyCode::Char('J') => {
                    overlay.preview_scroll = overlay.preview_scroll.saturating_add(3);
                }
                KeyCode::Char('K') => {
                    overlay.preview_scroll = overlay.preview_scroll.saturating_sub(3);
                }
                KeyCode::Tab | KeyCode::Esc => {
                    overlay.preview_focused = false;
                }
                KeyCode::Enter => {
                    self.install_search_item();
                }
                KeyCode::Char('y') => {
                    if let Some(item) = overlay.selected_item() {
                        let name = item.name.clone();
                        match arboard::Clipboard::new().and_then(|mut cb| cb.set_text(&name)) {
                            Ok(()) => self.set_message(format!("Copied: {}", name)),
                            Err(e) => self.set_message(format!("Clipboard error: {}", e)),
                        }
                    }
                }
                _ => {}
            }
            return;
        }

        // Filter input mode: characters go to filter, arrows navigate list
        match key.code {
            KeyCode::Esc => {
                self.search_overlay = None;
                self.search_receiver = None;
                self.search_source_pending = None;
            }
            KeyCode::Enter => {
                self.install_search_item();
            }
            KeyCode::Down => {
                let count = overlay.filtered_indices().len();
                if count > 0 && overlay.selected < count - 1 {
                    overlay.selected += 1;
                    overlay.preview_scroll = 0;
                }
            }
            KeyCode::Up => {
                if overlay.selected > 0 {
                    overlay.selected -= 1;
                    overlay.preview_scroll = 0;
                }
            }
            KeyCode::Tab => {
                overlay.preview_focused = true;
            }
            KeyCode::Char(c) => {
                let byte_pos = overlay
                    .filter
                    .char_indices()
                    .nth(overlay.filter_cursor)
                    .map(|(i, _)| i)
                    .unwrap_or(overlay.filter.len());
                overlay.filter.insert(byte_pos, c);
                overlay.filter_cursor += 1;
                overlay.selected = 0;
                overlay.preview_scroll = 0;
            }
            KeyCode::Backspace => {
                if overlay.filter_cursor > 0 {
                    overlay.filter_cursor -= 1;
                    let byte_pos = overlay
                        .filter
                        .char_indices()
                        .nth(overlay.filter_cursor)
                        .map(|(i, _)| i)
                        .unwrap_or(overlay.filter.len());
                    overlay.filter.remove(byte_pos);
                    overlay.selected = 0;
                    overlay.preview_scroll = 0;
                } else {
                    // Backspace with empty filter closes the overlay
                    self.search_overlay = None;
                    self.search_receiver = None;
                    self.search_source_pending = None;
                }
            }
            KeyCode::Left => {
                if overlay.filter_cursor > 0 {
                    overlay.filter_cursor -= 1;
                }
            }
            KeyCode::Right => {
                if overlay.filter_cursor < overlay.filter.chars().count() {
                    overlay.filter_cursor += 1;
                }
            }
            _ => {}
        }
    }
}
