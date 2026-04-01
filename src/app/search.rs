use super::*;

impl App {
    pub(crate) fn open_search_overlay(&mut self) {
        let source = match self.active_panel {
            Panel::Skills => SearchSource::Skills,
            Panel::Mcp => SearchSource::Mcp,
            Panel::Plugins => SearchSource::Plugins,
            _ => return,
        };

        // All sources use background thread for network fetch
        self.set_message(format!("Loading {}...", source.label()));

        let (tx, rx) = mpsc::channel();

        match source {
            SearchSource::Skills => {
                let cache = self.skills_registry_cache.clone();
                let installed_names: Vec<String> =
                    self.data.skills.iter().map(|s| s.name.clone()).collect();
                let installed_dirs: Vec<String> = self
                    .data
                    .skills
                    .iter()
                    .map(|s| s.dir_name.clone())
                    .collect();

                std::thread::spawn(move || {
                    let entries = match cache {
                        Some(cached) => Ok(cached),
                        None => sources::skills_registry::fetch_all_skills(),
                    };
                    let result = entries.map(|entries| {
                        entries
                            .into_iter()
                            .map(|e| {
                                let installed = installed_names.contains(&e.name)
                                    || installed_dirs.contains(&e.dir_name);
                                let preview = e.preview_body(installed);
                                let extra = e.source.clone();
                                SearchOverlayItem {
                                    name: e.name.clone(),
                                    description: e.description.clone(),
                                    extra,
                                    installed,
                                    preview,
                                    data: SearchItemData::Skill(e),
                                }
                            })
                            .collect()
                    });
                    let _ = tx.send(result);
                });
            }
            SearchSource::Mcp => {
                let installed_names: Vec<String> = self
                    .data
                    .mcp
                    .user
                    .iter()
                    .chain(self.data.mcp.project.iter())
                    .flat_map(|s| {
                        let mut names = vec![s.name.clone()];
                        names.extend(s.args.iter().cloned());
                        names
                    })
                    .collect();

                std::thread::spawn(move || {
                    let result = sources::mcp_registry::search_all("").map(|entries| {
                        entries
                            .into_iter()
                            .map(|e| {
                                let installed = installed_names.iter().any(|n| n.contains(&e.name));
                                let version_info = if e.version.is_empty() {
                                    String::new()
                                } else {
                                    format!("v{} ", e.version)
                                };
                                let extra = format!(
                                    "{}{} {}",
                                    version_info,
                                    e.registry,
                                    e.popularity_dots()
                                );
                                let preview = e.preview_body();
                                SearchOverlayItem {
                                    name: e.name.clone(),
                                    description: e.description.clone(),
                                    extra,
                                    installed,
                                    preview,
                                    data: SearchItemData::Mcp(e),
                                }
                            })
                            .collect()
                    });
                    let _ = tx.send(result);
                });
            }
            SearchSource::Plugins => {
                let plugins_dir = self.paths.claude_dir.join("plugins");
                let installed_names: Vec<String> = self
                    .data
                    .plugins
                    .installed
                    .iter()
                    .map(|p| p.name.clone())
                    .collect();

                std::thread::spawn(move || {
                    let result =
                        sources::plugin_registry::search_all(&plugins_dir, "").map(|entries| {
                            entries
                                .into_iter()
                                .map(|e| {
                                    let installed = installed_names.contains(&e.name);
                                    let extra = if !e.category.is_empty() {
                                        format!("{} ({})", e.marketplace, e.category)
                                    } else if !e.component_summary().is_empty() {
                                        format!("{} {}", e.marketplace, e.component_summary())
                                    } else {
                                        e.marketplace.clone()
                                    };
                                    let preview = e.preview_body();
                                    SearchOverlayItem {
                                        name: e.name.clone(),
                                        description: e.description.clone(),
                                        extra,
                                        installed,
                                        preview,
                                        data: SearchItemData::Plugin(e),
                                    }
                                })
                                .collect()
                        });
                    let _ = tx.send(result);
                });
            }
        }

        self.search_receiver = Some(rx);
        self.search_source_pending = Some(source);
    }

    pub(crate) fn poll_search_results(&mut self) {
        let rx = match self.search_receiver.as_ref() {
            Some(rx) => rx,
            None => return,
        };

        match rx.try_recv() {
            Ok(result) => {
                let source = self
                    .search_source_pending
                    .take()
                    .unwrap_or(SearchSource::Skills);
                self.search_receiver = None;

                match result {
                    Ok(items) => {
                        // Cache skills entries for future searches
                        if source == SearchSource::Skills {
                            let skill_entries: Vec<_> = items
                                .iter()
                                .filter_map(|item| {
                                    if let SearchItemData::Skill(ref e) = item.data {
                                        Some(e.clone())
                                    } else {
                                        None
                                    }
                                })
                                .collect();
                            self.skills_registry_cache = Some(skill_entries);
                        }

                        let count = items.len();
                        self.search_overlay = Some(SearchOverlay {
                            source,
                            all_items: items,
                            filter: String::new(),
                            filter_cursor: 0,
                            selected: 0,
                            preview_scroll: 0,
                            preview_focused: false,
                        });
                        self.set_message(format!("Found {} items", count));
                    }
                    Err(e) => {
                        tracing::warn!("Registry search failed: {}", e);
                        self.set_message(format!("Search failed: {e}"));
                    }
                }
            }
            Err(mpsc::TryRecvError::Empty) => {
                // Still loading — do nothing, UI stays responsive
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                // Thread panicked or dropped sender
                tracing::warn!("Search background thread disconnected");
                self.search_receiver = None;
                self.search_source_pending = None;
                self.set_message("Search failed: background fetch dropped".to_string());
            }
        }
    }

    pub(crate) fn install_search_item(&mut self) {
        let item = {
            let overlay = match self.search_overlay.as_ref() {
                Some(o) => o,
                None => return,
            };
            match overlay.selected_item() {
                Some(item) => item.clone(),
                None => return,
            }
        };

        if item.installed {
            self.set_message(format!("'{}' is already installed", item.name));
            return;
        }

        match &item.data {
            SearchItemData::Skill(entry) => {
                self.input_mode = InputMode::Confirm(Box::new(ConfirmState {
                    message: format!(
                        "Install skill '{}' — (y)es user / (p)roject scope?",
                        entry.name
                    ),
                    purpose: ConfirmPurpose::InstallSkillFromRegistry {
                        entry: entry.clone(),
                    },
                }));
            }
            SearchItemData::Mcp(entry) => {
                self.input_mode = InputMode::Confirm(Box::new(ConfirmState {
                    message: format!("Install '{}' — (y)es user / (p)roject scope?", entry.name),
                    purpose: ConfirmPurpose::InstallMcpFromRegistry {
                        entry: entry.clone(),
                        scope: Scope::User,
                    },
                }));
            }
            SearchItemData::Plugin(entry) => {
                self.input_mode = InputMode::Confirm(Box::new(ConfirmState {
                    message: format!("Install '{}' from {}? (y/n)", entry.name, entry.marketplace),
                    purpose: ConfirmPurpose::InstallPlugin {
                        entry: entry.clone(),
                    },
                }));
            }
        }
    }
}
