use super::*;

impl App {
    pub(crate) fn process_input(&mut self, value: String, purpose: InputPurpose) {
        match purpose {
            InputPurpose::Filter => {
                self.filter = value;
            }
            InputPurpose::AddPermission { kind } => {
                if !value.is_empty() {
                    if let Err(e) =
                        sources::settings::add_permission(&self.paths, "user", &kind, &value)
                    {
                        self.set_message(format!("Error: {e}"));
                    } else {
                        self.set_message(format!("Added {kind}: {value}"));
                        self.data.settings = sources::settings::load(&self.paths);
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
                    if let Err(e) =
                        sources::mcp::add(&self.paths, scope.as_str(), name, command, &args)
                    {
                        self.set_message(format!("Error: {e}"));
                    } else {
                        self.set_message(format!("Added MCP server: {name}"));
                        self.data.mcp = sources::mcp::load(&self.paths);
                    }
                }
            }
            InputPurpose::CreateSkill => {
                if !value.is_empty() {
                    let dir = self.paths.user_skills_dir().join(&value);
                    if dir.exists() {
                        self.set_message(format!("Skill '{}' already exists", value));
                    } else {
                        match std::fs::create_dir_all(&dir) {
                            Ok(()) => {
                                let skill_md = dir.join("SKILL.md");
                                let template = format!(
                                    "---\nname: {}\ndescription: \nuser_invocable: true\n---\n\n# {}\n\nAdd your skill instructions here.\n",
                                    value, value
                                );
                                if let Err(e) = std::fs::write(&skill_md, template) {
                                    self.set_message(format!("Error: {}", e));
                                } else {
                                    self.set_message(format!(
                                        "Created skill '{}' — opening editor",
                                        value
                                    ));
                                    self.data.skills = sources::skills::load(&self.paths);
                                    self.pending_edit = Some(skill_md);
                                }
                            }
                            Err(e) => self.set_message(format!("Error: {}", e)),
                        }
                    }
                }
            }
            InputPurpose::CreateAgent => {
                if !value.is_empty() {
                    // Agents are flat .md files: ~/.claude/agents/<name>.md
                    let agents_dir = self.paths.user_agents_dir();
                    let agent_md = agents_dir.join(format!("{}.md", value));
                    if agent_md.exists() {
                        self.set_message(format!("Agent '{}' already exists", value));
                    } else if let Err(e) = std::fs::create_dir_all(&agents_dir) {
                        self.set_message(format!("Error: {}", e));
                    } else {
                        let template = format!(
                            "---\nname: {}\ndescription: \nmodel: sonnet\n---\n\nAdd your agent instructions here.\n",
                            value
                        );
                        if let Err(e) = std::fs::write(&agent_md, template) {
                            self.set_message(format!("Error: {}", e));
                        } else {
                            self.set_message(format!("Created agent '{}' — opening editor", value));
                            self.data.agents = sources::agents::load(&self.paths);
                            self.pending_edit = Some(agent_md);
                        }
                    }
                }
            }
        }
    }

    pub(crate) fn process_confirm(&mut self, purpose: ConfirmPurpose) {
        match purpose {
            ConfirmPurpose::DeletePermission { scope, kind, index } => {
                // Resolve rule text for undo before deleting
                let perms = &self.data.settings.permissions;
                let rules = match kind.as_str() {
                    "allow" => &perms.allow,
                    "ask" => &perms.ask,
                    "deny" => &perms.deny,
                    _ => &perms.deny,
                };
                let rule_text = rules.get(index).map(|r| r.rule.clone()).unwrap_or_default();
                if let Err(e) =
                    sources::settings::remove_permission(&self.paths, scope.as_str(), &kind, index)
                {
                    tracing::error!("Failed to delete permission: {}", e);
                    self.set_message(format!("Error: {e}"));
                } else {
                    self.push_undo(UndoAction::Permission {
                        scope,
                        kind: kind.clone(),
                        rule: rule_text,
                        _index: index,
                    });
                    self.set_message("Permission deleted".to_string());
                    self.data.settings = sources::settings::load(&self.paths);
                }
            }
            ConfirmPurpose::DeleteMcpServer { scope, name } => {
                // Save the MCP config before removal for undo
                let config_snapshot = sources::read_json(&self.paths.mcp_path(scope.as_str()));
                if let Err(e) = sources::mcp::remove(&self.paths, scope.as_str(), &name) {
                    tracing::error!("Failed to delete MCP server '{}': {}", name, e);
                    self.set_message(format!("Error: {e}"));
                } else {
                    self.push_undo(UndoAction::McpServer {
                        scope,
                        _name: name.clone(),
                        config_snapshot,
                    });
                    self.set_message(format!("Deleted: {name}"));
                    self.data.mcp = sources::mcp::load(&self.paths);
                }
            }
            ConfirmPurpose::InstallMcpFromRegistry { entry, scope } => {
                if let Err(e) = sources::mcp::add(
                    &self.paths,
                    scope.as_str(),
                    &entry.name,
                    &entry.install_command,
                    &entry.install_args,
                ) {
                    tracing::error!("Failed to install MCP server '{}': {}", entry.name, e);
                    self.set_message(format!("Error: {e}"));
                } else {
                    tracing::info!("Installed MCP server: {}", entry.name);
                    self.set_message(format!("Installed: {}", entry.name));
                    self.data.mcp = sources::mcp::load(&self.paths);
                    self.search_overlay = None;
                }
            }
            ConfirmPurpose::InstallSkillFromRegistry { entry } => {
                let skills_dir = self.paths.user_skills_dir();
                match sources::skills_registry::install_skill(&skills_dir, &entry) {
                    Ok(()) => {
                        tracing::info!("Installed skill: {}", entry.name);
                        self.set_message(format!("Installed: {}", entry.name));
                        self.data.skills = sources::skills::load(&self.paths);
                        self.search_overlay = None;
                    }
                    Err(e) => {
                        tracing::error!("Failed to install skill '{}': {}", entry.name, e);
                        self.set_message(format!("Error: {e}"));
                    }
                }
            }
            ConfirmPurpose::DeleteMemory { path, name } => {
                // Read content before removing for undo
                let content = std::fs::read_to_string(&path).unwrap_or_default();
                if let Err(e) = sources::memory::remove(&path) {
                    tracing::error!("Failed to delete memory '{}': {}", name, e);
                    self.set_message(format!("Error: {e}"));
                } else {
                    self.push_undo(UndoAction::Memory {
                        path: path.clone(),
                        content,
                    });
                    self.set_message(format!("Deleted: {name}"));
                    self.data.memory = sources::memory::load(&self.paths);
                }
            }
            ConfirmPurpose::DeleteSkill { path, name } => {
                // Save all files in the skill directory before removing for undo
                let mut saved_files = Vec::new();
                let dir_path = path.parent().map(|d| d.to_path_buf());
                if let Some(ref dir) = dir_path {
                    if let Ok(entries) = std::fs::read_dir(dir) {
                        for entry in entries.flatten() {
                            if let Ok(data) = std::fs::read(entry.path()) {
                                saved_files.push((entry.path(), data));
                            }
                        }
                    }
                }
                if let Err(e) = sources::skills::remove(&path) {
                    tracing::error!("Failed to delete skill '{}': {}", name, e);
                    self.set_message(format!("Error: {e}"));
                } else {
                    if let Some(dir) = dir_path {
                        self.push_undo(UndoAction::Skill {
                            dir_path: dir,
                            files: saved_files,
                        });
                    }
                    self.set_message(format!("Deleted: {name}"));
                    self.data.skills = sources::skills::load(&self.paths);
                }
            }
            ConfirmPurpose::DeleteAgent { path, name } => {
                // Save the agent file content before removing for undo
                let content = std::fs::read_to_string(&path).unwrap_or_default();
                if let Err(e) = sources::agents::remove(&path) {
                    tracing::error!("Failed to delete agent '{}': {}", name, e);
                    self.set_message(format!("Error: {e}"));
                } else {
                    self.push_undo(UndoAction::Agent {
                        file_path: path.clone(),
                        content,
                    });
                    self.set_message(format!("Deleted: {name}"));
                    self.data.agents = sources::agents::load(&self.paths);
                }
            }
            ConfirmPurpose::DeletePlugin { name } => {
                if let Err(e) = sources::plugins::remove(&self.paths, &name) {
                    tracing::error!("Failed to delete plugin '{}': {}", name, e);
                    self.set_message(format!("Error: {e}"));
                } else {
                    self.set_message(format!("Removed: {name}"));
                    self.data.plugins = sources::plugins::load(&self.paths);
                }
            }
            ConfirmPurpose::UnblockPlugin { name } => {
                if let Err(e) = sources::plugins::unblock(&self.paths, &name) {
                    self.set_message(format!("Error: {e}"));
                } else {
                    self.set_message(format!("Removed from blocklist: {name}"));
                    self.data.plugins = sources::plugins::load(&self.paths);
                }
            }
            ConfirmPurpose::InstallPlugin { entry } => {
                match sources::plugins::install(
                    &self.paths,
                    &entry.name,
                    &entry.version,
                    &entry.marketplace,
                ) {
                    Ok(()) => {
                        tracing::info!("Installed plugin: {}", entry.name);
                        self.set_message(format!("Installed: {}", entry.name));
                        self.data.plugins = sources::plugins::load(&self.paths);
                        self.search_overlay = None;
                    }
                    Err(e) => {
                        tracing::error!("Failed to install plugin '{}': {}", entry.name, e);
                        self.set_message(format!("Error: {e}"));
                    }
                }
            }
        }
    }

    pub(crate) fn action_add(&mut self) {
        match self.active_panel {
            Panel::Settings => {
                self.input_mode = InputMode::Input(InputState {
                    prompt: "Allow permission".to_string(),
                    value: String::new(),
                    cursor: 0,
                    purpose: InputPurpose::AddPermission {
                        kind: "allow".to_string(),
                    },
                });
            }
            Panel::Mcp => {
                self.input_mode = InputMode::Input(InputState {
                    prompt: "Add server (name command args...)".to_string(),
                    value: String::new(),
                    cursor: 0,
                    purpose: InputPurpose::AddMcpServer {
                        scope: sources::Scope::User,
                    },
                });
            }
            Panel::Skills => {
                self.input_mode = InputMode::Input(InputState {
                    prompt: "Skill name (e.g. my-skill)".to_string(),
                    value: String::new(),
                    cursor: 0,
                    purpose: InputPurpose::CreateSkill,
                });
            }
            Panel::Agents => {
                self.input_mode = InputMode::Input(InputState {
                    prompt: "Agent name (e.g. my-agent)".to_string(),
                    value: String::new(),
                    cursor: 0,
                    purpose: InputPurpose::CreateAgent,
                });
            }
            _ => {}
        }
    }

    pub(crate) fn action_delete(&mut self) {
        match self.active_panel {
            Panel::Settings => {
                if let Some((kind, scope, rule_index, rule_text)) = self.resolve_permission() {
                    self.input_mode = InputMode::Confirm(Box::new(ConfirmState {
                        message: format!("Delete {kind} rule '{rule_text}'?"),
                        purpose: ConfirmPurpose::DeletePermission {
                            scope,
                            kind,
                            index: rule_index,
                        },
                    }));
                }
            }
            Panel::Mcp => {
                if let Some((scope, name)) = self.resolve_mcp_server() {
                    self.input_mode = InputMode::Confirm(Box::new(ConfirmState {
                        message: format!("Delete MCP server '{name}' from {scope}?"),
                        purpose: ConfirmPurpose::DeleteMcpServer { scope, name },
                    }));
                } else {
                    self.set_message("No server selected".to_string());
                }
            }
            Panel::Memory => {
                let idx = self.panel_offset();
                if let Some(path) = self.item_paths.get(idx).and_then(|p| p.as_ref()) {
                    let name = self
                        .data
                        .memory
                        .files
                        .iter()
                        .find(|f| &f.path == path)
                        .map(|f| f.name.clone())
                        .unwrap_or_else(|| "memory".to_string());
                    self.input_mode = InputMode::Confirm(Box::new(ConfirmState {
                        message: format!("Delete memory '{name}'?"),
                        purpose: ConfirmPurpose::DeleteMemory {
                            path: path.clone(),
                            name,
                        },
                    }));
                }
            }
            Panel::Skills => {
                let idx = self.panel_offset();
                if let Some(path) = self.item_paths.get(idx).and_then(|p| p.as_ref()) {
                    // Find the skill name from the bodies/paths
                    let name = self
                        .data
                        .skills
                        .iter()
                        .find(|s| &s.path == path)
                        .map(|s| s.name.clone())
                        .unwrap_or_else(|| "skill".to_string());
                    self.input_mode = InputMode::Confirm(Box::new(ConfirmState {
                        message: format!("Delete skill '{name}'?"),
                        purpose: ConfirmPurpose::DeleteSkill {
                            path: path.clone(),
                            name,
                        },
                    }));
                }
            }
            Panel::Agents => {
                let idx = self.panel_offset();
                if let Some(path) = self.item_paths.get(idx).and_then(|p| p.as_ref()) {
                    let name = self
                        .data
                        .agents
                        .iter()
                        .find(|a| &a.path == path)
                        .map(|a| a.name.clone())
                        .unwrap_or_else(|| "agent".to_string());
                    self.input_mode = InputMode::Confirm(Box::new(ConfirmState {
                        message: format!("Delete agent '{name}'?"),
                        purpose: ConfirmPurpose::DeleteAgent {
                            path: path.clone(),
                            name,
                        },
                    }));
                }
            }
            Panel::Plugins => {
                if let Some((kind, name)) = self.resolve_plugin() {
                    match kind.as_str() {
                        "installed" => {
                            self.input_mode = InputMode::Confirm(Box::new(ConfirmState {
                                message: format!("Remove plugin '{name}'?"),
                                purpose: ConfirmPurpose::DeletePlugin { name },
                            }));
                        }
                        "blocked" => {
                            self.input_mode = InputMode::Confirm(Box::new(ConfirmState {
                                message: format!("Remove '{name}' from blocklist?"),
                                purpose: ConfirmPurpose::UnblockPlugin { name },
                            }));
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }

    pub(crate) fn action_toggle_mcp(&mut self) {
        if let Some((scope, name)) = self.resolve_mcp_server() {
            if let Err(e) = sources::mcp::toggle(&self.paths, scope.as_str(), &name) {
                self.set_message(format!("Error: {e}"));
            } else {
                self.set_message(format!("Toggled: {name}"));
                self.data.mcp = sources::mcp::load(&self.paths);
            }
        } else {
            self.set_message("No server selected".to_string());
        }
    }

    pub(crate) fn action_edit_external(&mut self) {
        let idx = self.panel_offset();
        // item_paths is rebuilt every render — handles scope headers correctly
        self.pending_edit = self.item_paths.get(idx).and_then(|p| p.clone());
    }

    pub(super) fn action_export(&mut self) {
        let json = match self.active_panel {
            Panel::Memory => serde_json::to_string_pretty(&self.data.memory.files),
            Panel::Skills => serde_json::to_string_pretty(&self.data.skills),
            Panel::Agents => serde_json::to_string_pretty(&self.data.agents),
            Panel::Mcp => serde_json::to_string_pretty(&self.data.mcp),
            Panel::Settings => serde_json::to_string_pretty(&self.data.settings),
            Panel::Plugins => serde_json::to_string_pretty(&self.data.plugins),
            Panel::Sessions => serde_json::to_string_pretty(&self.data.sessions),
            Panel::Stats => serde_json::to_string_pretty(&self.data.stats),
            Panel::Todos => serde_json::to_string_pretty(&self.data.todos),
            Panel::Config => serde_json::to_string_pretty(&self.data.claude_md),
            Panel::Projects => {
                // Export project list
                serde_json::to_string_pretty(&self.projects)
            }
        };

        match json {
            Ok(text) => match arboard::Clipboard::new().and_then(|mut cb| cb.set_text(&text)) {
                Ok(()) => {
                    let lines = text.lines().count();
                    self.set_message(format!(
                        "Exported {} panel ({} lines) to clipboard",
                        self.active_panel.label(),
                        lines
                    ));
                }
                Err(e) => self.set_message(format!("Clipboard error: {}", e)),
            },
            Err(e) => self.set_message(format!("Serialize error: {}", e)),
        }
    }

    /// Push an undo action, capping the stack at 20 entries.
    fn push_undo(&mut self, action: UndoAction) {
        if self.undo_stack.len() >= 20 {
            self.undo_stack.remove(0);
        }
        self.undo_stack.push(action);
    }

    pub(super) fn action_undo(&mut self) {
        let action = match self.undo_stack.pop() {
            Some(a) => a,
            None => {
                self.set_message("Nothing to undo".to_string());
                return;
            }
        };
        match action {
            UndoAction::Memory { path, content } => {
                if let Some(parent) = path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                match std::fs::write(&path, &content) {
                    Ok(()) => {
                        self.set_message("Undo: restored memory".to_string());
                        self.data.memory = sources::memory::load(&self.paths);
                    }
                    Err(e) => self.set_message(format!("Undo failed: {e}")),
                }
            }
            UndoAction::Skill { dir_path, files } => {
                let _ = std::fs::create_dir_all(&dir_path);
                for (file_path, data) in &files {
                    let _ = std::fs::write(file_path, data);
                }
                self.set_message("Undo: restored skill".to_string());
                self.data.skills = sources::skills::load(&self.paths);
            }
            UndoAction::Agent { file_path, content } => {
                if let Some(parent) = file_path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                match std::fs::write(&file_path, &content) {
                    Ok(()) => {
                        self.set_message("Undo: restored agent".to_string());
                        self.data.agents = sources::agents::load(&self.paths);
                    }
                    Err(e) => self.set_message(format!("Undo failed: {e}")),
                }
            }
            UndoAction::McpServer {
                scope,
                _name: _,
                config_snapshot,
            } => {
                let mcp_path = self.paths.mcp_path(scope.as_str());
                match sources::write_json(&mcp_path, &config_snapshot) {
                    Ok(()) => {
                        self.set_message("Undo: restored MCP server".to_string());
                        self.data.mcp = sources::mcp::load(&self.paths);
                    }
                    Err(e) => self.set_message(format!("Undo failed: {e}")),
                }
            }
            UndoAction::Permission {
                scope, kind, rule, ..
            } => {
                match sources::settings::add_permission(&self.paths, scope.as_str(), &kind, &rule) {
                    Ok(()) => {
                        self.set_message(format!("Undo: restored {} permission", kind));
                        self.data.settings = sources::settings::load(&self.paths);
                    }
                    Err(e) => self.set_message(format!("Undo failed: {e}")),
                }
            }
        }
    }

    pub(super) fn action_copy(&mut self) {
        let idx = self.panel_offset();
        let text = match self.active_panel {
            Panel::Config | Panel::Memory | Panel::Skills | Panel::Agents => self
                .item_paths
                .get(idx)
                .and_then(|p| p.as_ref())
                .map(|p| p.to_string_lossy().to_string()),
            Panel::Mcp => {
                // item_paths has "scope:name" synthetic paths
                self.item_paths.get(idx).and_then(|p| p.as_ref()).map(|p| {
                    let s = p.to_string_lossy();
                    s.split_once(':')
                        .map(|(_, name)| name.to_string())
                        .unwrap_or(s.to_string())
                })
            }
            Panel::Settings => {
                self.item_paths.get(idx).and_then(|p| p.as_ref()).map(|p| {
                    let s = p.to_string_lossy();
                    // perm:kind:scope:idx:rule format
                    s.rsplit_once(':')
                        .map(|(_, rule)| rule.to_string())
                        .unwrap_or(s.to_string())
                })
            }
            Panel::Sessions => self
                .item_paths
                .get(idx)
                .and_then(|p| p.as_ref())
                .map(|p| p.to_string_lossy().to_string()),
            Panel::Plugins => self.item_paths.get(idx).and_then(|p| p.as_ref()).map(|p| {
                let s = p.to_string_lossy();
                s.split_once(':')
                    .map(|(_, name)| name.to_string())
                    .unwrap_or(s.to_string())
            }),
            _ => None,
        };

        if let Some(text) = text {
            match arboard::Clipboard::new().and_then(|mut cb| cb.set_text(&text)) {
                Ok(()) => self.set_message(format!("Copied: {}", text)),
                Err(e) => self.set_message(format!("Clipboard error: {}", e)),
            }
        }
    }
}
