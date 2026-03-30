use std::path::PathBuf;

use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::ListItem,
};

use crate::app::{App, Panel};
use lazyclaude::sources::Scope;

pub(crate) type ItemRow = (ListItem<'static>, Option<PathBuf>, Option<String>);

pub(crate) fn build_detail_items(
    app: &App,
) -> (Vec<ListItem<'static>>, Vec<Option<PathBuf>>, Vec<Option<String>>) {
    use fuzzy_matcher::skim::SkimMatcherV2;
    use fuzzy_matcher::FuzzyMatcher;
    let matcher = SkimMatcherV2::default();
    let matches = |s: &str| {
        app.filter.is_empty() || matcher.fuzzy_match(s, &app.filter).is_some()
    };

    let mut items = Vec::new();
    let mut paths: Vec<Option<PathBuf>> = Vec::new();
    let mut bodies: Vec<Option<String>> = Vec::new();

    match app.active_panel {
        Panel::Projects => {
            // First item: Global
            let style = if app.selected_project == 0 {
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            items.push(ListItem::new(Line::from(vec![
                Span::styled("  ", Style::default()),
                Span::styled("Global (User)", style),
            ])));
            paths.push(None);
            bodies.push(None);

            for (i, project) in app.projects.iter().enumerate() {
                if !matches(&project.short_name) && !matches(&project.name) {
                    continue;
                }
                let selected = app.selected_project == i + 1;
                let color = if selected {
                    Color::Green
                } else if project.exists {
                    Color::White
                } else {
                    Color::DarkGray
                };
                let marker = if selected { " *" } else { "" };
                items.push(ListItem::new(Line::from(vec![
                    Span::styled("  ", Style::default()),
                    Span::styled(
                        project.short_name.clone(),
                        Style::default().fg(color),
                    ),
                    Span::styled(marker.to_string(), Style::default().fg(Color::Green)),
                    if !project.exists {
                        Span::styled(" (missing)", Style::default().fg(Color::Red))
                    } else {
                        Span::styled("", Style::default())
                    },
                ])));
                paths.push(None);
                bodies.push(Some(project.name.clone()));
            }
        }

        Panel::Config => {
            let proj: Vec<_> = app.data.claude_md.iter().filter(|f| f.scope == Scope::Project).collect();
            let user: Vec<_> = app.data.claude_md.iter().filter(|f| f.scope == Scope::User).collect();
            let proj_entries: Vec<ItemRow> = proj
                .iter()
                .filter(|f| matches(&f.name))
                .map(|f| (claude_md_item(f), Some(f.path.clone()), Some(f.content.clone())))
                .collect();
            let user_entries: Vec<ItemRow> = user
                .iter()
                .filter(|f| matches(&f.name))
                .map(|f| (claude_md_item(f), Some(f.path.clone()), Some(f.content.clone())))
                .collect();
            push_scope_group(&mut items, &mut paths, &mut bodies, &format!("Project ({})", proj.len()), proj_entries);
            push_scope_group(&mut items, &mut paths, &mut bodies, &format!("User ({})", user.len()), user_entries);
        }

        Panel::Memory => {
            for f in &app.data.memory.files {
                if !matches(&f.name) && !matches(&f.description) {
                    continue;
                }
                let badge = format!("[{}]", &f.mem_type[..f.mem_type.len().min(4)]);
                items.push(ListItem::new(Line::from(vec![
                    Span::styled("  ", Style::default()),
                    Span::styled(f.name.clone(), Style::default().fg(Color::Green)),
                    Span::styled(format!("  {badge}"), Style::default().fg(Color::Cyan)),
                ])));
                paths.push(Some(f.path.clone()));
                bodies.push(Some(f.body.clone()));
            }
        }

        Panel::Skills => {
            let proj: Vec<_> = app.data.skills.iter().filter(|s| s.scope == Scope::Project).collect();
            let user: Vec<_> = app.data.skills.iter().filter(|s| s.scope == Scope::User).collect();
            let proj_entries: Vec<ItemRow> = proj
                .iter()
                .filter(|s| matches(&s.name) || matches(&s.description))
                .map(|s| (skill_item(s), Some(s.path.clone()), Some(s.body.clone())))
                .collect();
            let user_entries: Vec<ItemRow> = user
                .iter()
                .filter(|s| matches(&s.name) || matches(&s.description))
                .map(|s| (skill_item(s), Some(s.path.clone()), Some(s.body.clone())))
                .collect();
            push_scope_group(&mut items, &mut paths, &mut bodies, &format!("Project ({})", proj.len()), proj_entries);
            push_scope_group(&mut items, &mut paths, &mut bodies, &format!("User ({})", user.len()), user_entries);
        }

        Panel::Agents => {
            let proj: Vec<_> = app.data.agents.iter().filter(|a| a.scope == Scope::Project).collect();
            let user: Vec<_> = app.data.agents.iter().filter(|a| a.scope == Scope::User).collect();
            let proj_entries: Vec<ItemRow> = proj
                .iter()
                .filter(|a| matches(&a.name) || matches(&a.description))
                .map(|a| (agent_item(a), Some(a.path.clone()), Some(a.body.clone())))
                .collect();
            let user_entries: Vec<ItemRow> = user
                .iter()
                .filter(|a| matches(&a.name) || matches(&a.description))
                .map(|a| (agent_item(a), Some(a.path.clone()), Some(a.body.clone())))
                .collect();
            push_scope_group(&mut items, &mut paths, &mut bodies, &format!("Project ({})", proj.len()), proj_entries);
            push_scope_group(&mut items, &mut paths, &mut bodies, &format!("User ({})", user.len()), user_entries);
        }

        Panel::Mcp => {
                let proj_entries: Vec<ItemRow> = app
                    .data
                    .mcp
                    .project
                    .iter()
                    .filter(|s| matches(&s.name))
                    .map(|s| (mcp_item(s), Some(PathBuf::from(format!("{}:{}", "project", s.name))), Some(s.preview_body("project"))))
                    .collect();
                let user_entries: Vec<ItemRow> = app
                    .data
                    .mcp
                    .user
                    .iter()
                    .filter(|s| matches(&s.name))
                    .map(|s| (mcp_item(s), Some(PathBuf::from(format!("{}:{}", "user", s.name))), Some(s.preview_body("user"))))
                    .collect();
                push_scope_group(
                    &mut items,
                    &mut paths,
                    &mut bodies,
                    &format!("Project ({})", app.data.mcp.project.len()),
                    proj_entries,
                );
                push_scope_group(
                    &mut items,
                    &mut paths,
                    &mut bodies,
                    &format!("User ({})", app.data.mcp.user.len()),
                    user_entries,
                );
        }

        Panel::Settings => {
            let perms = &app.data.settings.permissions;
            if !perms.allow.is_empty() {
                items.push(ListItem::new(Line::from(Span::styled(
                    "  Allow",
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ))));
                paths.push(None);
                bodies.push(None);
                for (i, rule) in perms.allow.iter().enumerate() {
                    if !matches(&rule.rule) {
                        continue;
                    }
                    items.push(ListItem::new(Line::from(vec![
                        Span::styled("    ", Style::default()),
                        Span::styled(rule.rule.clone(), Style::default().fg(Color::White)),
                        Span::styled(
                            format!("  [{}]", rule.scope),
                            Style::default().fg(Color::Cyan),
                        ),
                    ])));
                    paths.push(Some(PathBuf::from(format!("perm:allow:{}:{}", rule.scope, i))));
                    bodies.push(Some(format!(
                        "# Allow Permission\n\nRule: {}\nScope: {}\n\n---\n\nThis rule is defined in the {} settings.",
                        rule.rule, rule.scope, rule.scope
                    )));
                }
            }
            if !perms.deny.is_empty() {
                items.push(ListItem::new(Line::from(Span::styled(
                    "  Deny",
                    Style::default()
                        .fg(Color::Red)
                        .add_modifier(Modifier::BOLD),
                ))));
                paths.push(None);
                bodies.push(None);
                for (i, rule) in perms.deny.iter().enumerate() {
                    if !matches(&rule.rule) {
                        continue;
                    }
                    items.push(ListItem::new(Line::from(vec![
                        Span::styled("    ", Style::default()),
                        Span::styled(rule.rule.clone(), Style::default().fg(Color::White)),
                        Span::styled(
                            format!("  [{}]", rule.scope),
                            Style::default().fg(Color::Red),
                        ),
                    ])));
                    paths.push(Some(PathBuf::from(format!("perm:deny:{}:{}", rule.scope, i))));
                    bodies.push(Some(format!(
                        "# Deny Permission\n\nRule: {}\nScope: {}\n\n---\n\nThis rule is defined in the {} settings.",
                        rule.rule, rule.scope, rule.scope
                    )));
                }
            }

            // Hooks sub-section
            if !app.data.hooks.is_empty() {
                items.push(ListItem::new(Line::from(Span::styled(
                    "  Hooks",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ))));
                paths.push(None);
                bodies.push(None);
                let mut cur_event = String::new();
                for hook in &app.data.hooks {
                    if !matches(&hook.command) && !matches(&hook.event) && !matches(&hook.matcher) {
                        continue;
                    }
                    if hook.event != cur_event {
                        cur_event.clone_from(&hook.event);
                        items.push(ListItem::new(Line::from(Span::styled(
                            format!("    {cur_event}"),
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::BOLD),
                        ))));
                        paths.push(None);
                        bodies.push(None);
                    }
                    items.push(ListItem::new(Line::from(vec![
                        Span::styled("      ", Style::default()),
                        Span::styled(hook.matcher.clone(), Style::default().fg(Color::Green)),
                        Span::styled(" -> ", Style::default().fg(Color::DarkGray)),
                        Span::styled(hook.command.clone(), Style::default().fg(Color::White)),
                    ])));
                    paths.push(None);
                    bodies.push(Some(format!(
                        "# Hook: {}\n\nEvent: {}\nMatcher: {}\nCommand: {}\nType: {}\nScope: {}",
                        hook.command, hook.event, hook.matcher, hook.command, hook.hook_type, hook.scope
                    )));
                }
            }

            // Keybindings sub-section
            if !app.data.keybindings.is_empty() {
                items.push(ListItem::new(Line::from(Span::styled(
                    "  Keybindings",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ))));
                paths.push(None);
                bodies.push(None);
                for b in &app.data.keybindings {
                    if !matches(&b.key) && !matches(&b.command) {
                        continue;
                    }
                    let mut spans = vec![
                        Span::styled("    ", Style::default()),
                        Span::styled(
                            b.key.clone(),
                            Style::default()
                                .fg(Color::Yellow)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(" -> ", Style::default().fg(Color::DarkGray)),
                        Span::styled(b.command.clone(), Style::default().fg(Color::White)),
                    ];
                    if !b.context.is_empty() {
                        spans.push(Span::styled(
                            format!("  [{}]", b.context),
                            Style::default().fg(Color::Cyan),
                        ));
                    }
                    items.push(ListItem::new(Line::from(spans)));
                    paths.push(None);
                    bodies.push(Some(format!(
                        "# Keybinding\n\nKey: {}\nCommand: {}\nContext: {}",
                        b.key, b.command, if b.context.is_empty() { "(global)" } else { &b.context }
                    )));
                }
            }

            // General settings
            if let Some(obj) = app.data.settings.effective.as_object() {
                let general: Vec<_> = obj
                    .iter()
                    .filter(|(k, _)| *k != "permissions" && *k != "hooks")
                    .collect();
                if !general.is_empty() {
                    items.push(ListItem::new(Line::from(Span::styled(
                        "  General",
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    ))));
                    paths.push(None);
                    bodies.push(None);
                    for (key, value) in &general {
                        let val_str = match value {
                            serde_json::Value::String(s) => s.clone(),
                            other => other.to_string(),
                        };

                        let user_val = app.data.settings.scopes.user.get(*key);
                        let project_val = app.data.settings.scopes.project.get(*key);
                        let local_val = app.data.settings.scopes.local_.get(*key);

                        let format_val = |v: Option<&serde_json::Value>| -> String {
                            match v {
                                Some(serde_json::Value::String(s)) => s.clone(),
                                Some(v) => v.to_string(),
                                None => "(not set)".to_string(),
                            }
                        };

                        let body = format!(
                            "# {}\n\nEffective: {}\n\n---\n\nUser: {}\nProject: {}\nLocal: {}",
                            key, val_str, format_val(user_val), format_val(project_val), format_val(local_val)
                        );

                        items.push(ListItem::new(Line::from(vec![
                            Span::styled("    ", Style::default()),
                            Span::styled(key.to_string(), Style::default().fg(Color::Cyan)),
                            Span::styled(": ", Style::default().fg(Color::DarkGray)),
                            Span::styled(val_str, Style::default().fg(Color::White)),
                        ])));
                        paths.push(None);
                        bodies.push(Some(body));
                    }
                }
            }
        }

        Panel::Sessions => {
            for session in &app.data.sessions {
                if !matches(&session.id) {
                    continue;
                }
                let size = if session.size < 1024 {
                    format!("{} B", session.size)
                } else if session.size < 1024 * 1024 {
                    format!("{:.1} KB", session.size as f64 / 1024.0)
                } else {
                    format!("{:.1} MB", session.size as f64 / (1024.0 * 1024.0))
                };
                let summary = session
                    .summary
                    .as_deref()
                    .unwrap_or("(no summary)");
                items.push(ListItem::new(Line::from(vec![
                    Span::styled("  ", Style::default()),
                    Span::styled(
                        session.id[..session.id.len().min(8)].to_string(),
                        Style::default().fg(Color::Yellow),
                    ),
                    Span::styled(format!("  {size}"), Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        format!("  {summary}"),
                        Style::default().fg(Color::White),
                    ),
                ])));
                paths.push(Some(session.path.clone()));
                bodies.push(None);
            }
            if app.data.sessions.is_empty() {
                items.push(ListItem::new(Line::from(Span::styled(
                    "  No sessions",
                    Style::default().fg(Color::DarkGray),
                ))));
                paths.push(None);
                bodies.push(None);
            }
        }

        Panel::Stats => {
            // Rendered by custom dashboard, not list items
        }

        Panel::Todos => {
            for todo in &app.data.todos {
                if !matches(&todo.content) && !matches(&todo.id) {
                    continue;
                }
                let (badge, color) = match todo.status.as_str() {
                    "completed" => ("\u{2714}", Color::Green),    // checkmark
                    "in_progress" => ("\u{25cb}", Color::Yellow), // circle
                    _ => ("\u{25cb}", Color::Cyan),               // circle
                };
                let text = if todo.content.chars().count() > 80 {
                    let truncated: String = todo.content.chars().take(77).collect();
                    format!("{truncated}...")
                } else {
                    todo.content.clone()
                };
                items.push(ListItem::new(Line::from(vec![
                    Span::styled("  ", Style::default()),
                    Span::styled(format!("{badge} "), Style::default().fg(color)),
                    Span::styled(text, Style::default().fg(Color::White)),
                ])));
                paths.push(None);
                bodies.push(None);
            }
            if app.data.todos.is_empty() {
                items.push(ListItem::new(Line::from(Span::styled(
                    "  No todos",
                    Style::default().fg(Color::DarkGray),
                ))));
                paths.push(None);
                bodies.push(None);
            }
        }

        Panel::Plugins => {
                let p = &app.data.plugins;
                if !p.installed.is_empty() {
                    items.push(ListItem::new(Line::from(Span::styled(
                        "  Installed",
                        Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
                    ))));
                    paths.push(None);
                    bodies.push(None);
                    for plugin in &p.installed {
                        if !matches(&plugin.name) { continue; }
                        items.push(ListItem::new(Line::from(vec![
                            Span::styled("    \u{25cf} ", Style::default().fg(Color::Green)),
                            Span::styled(plugin.name.clone(), Style::default().fg(Color::White)),
                            Span::styled(format!("  v{}", plugin.version), Style::default().fg(Color::Cyan)),
                            Span::styled(format!("  [{}]", plugin.scope), Style::default().fg(Color::DarkGray)),
                        ])));
                        paths.push(Some(PathBuf::from(format!("plugin:installed:{}", plugin.name))));
                        bodies.push(Some(plugin.preview_body()));
                    }
                }
                if !p.blocked.is_empty() {
                    items.push(ListItem::new(Line::from(Span::styled(
                        "  Blocked",
                        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                    ))));
                    paths.push(None);
                    bodies.push(None);
                    for plugin in &p.blocked {
                        if !matches(&plugin.name) { continue; }
                        items.push(ListItem::new(Line::from(vec![
                            Span::styled("    \u{25cf} ", Style::default().fg(Color::Red)),
                            Span::styled(plugin.name.clone(), Style::default().fg(Color::DarkGray)),
                            Span::styled(format!("  {}", plugin.reason), Style::default().fg(Color::Red)),
                        ])));
                        paths.push(Some(PathBuf::from(format!("plugin:blocked:{}", plugin.name))));
                        bodies.push(Some(plugin.preview_body()));
                    }
                }
                if !p.marketplaces.is_empty() {
                    items.push(ListItem::new(Line::from(Span::styled(
                        "  Marketplaces",
                        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                    ))));
                    paths.push(None);
                    bodies.push(None);
                    for mp in &p.marketplaces {
                        if !matches(&mp.name) { continue; }
                        items.push(ListItem::new(Line::from(vec![
                            Span::styled("    ", Style::default()),
                            Span::styled(mp.name.clone(), Style::default().fg(Color::Yellow)),
                            Span::styled(format!("  {}", mp.repo), Style::default().fg(Color::DarkGray)),
                        ])));
                        paths.push(Some(PathBuf::from(format!("plugin:marketplace:{}", mp.name))));
                        bodies.push(Some(mp.preview_body()));
                    }
                }
                if p.installed.is_empty() && p.blocked.is_empty() && p.marketplaces.is_empty() {
                    items.push(ListItem::new(Line::from(Span::styled(
                        "  No plugins",
                        Style::default().fg(Color::DarkGray),
                    ))));
                    paths.push(None);
                    bodies.push(None);
                }
        }
    }

    (items, paths, bodies)
}

// -- Scope group helper ---------------------------------------------------

pub(crate) fn scope_header(label: &str) -> ListItem<'static> {
    ListItem::new(Line::from(Span::styled(
        format!("  {label}"),
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    )))
}

pub(crate) fn empty_hint(msg: &str) -> ListItem<'static> {
    ListItem::new(Line::from(Span::styled(
        format!("    {msg}"),
        Style::default().fg(Color::DarkGray),
    )))
}

pub(crate) fn push_scope_group(
    items: &mut Vec<ListItem<'static>>,
    paths: &mut Vec<Option<PathBuf>>,
    bodies: &mut Vec<Option<String>>,
    header: &str,
    entries: Vec<(ListItem<'static>, Option<PathBuf>, Option<String>)>,
) {
    items.push(scope_header(header));
    paths.push(None);
    bodies.push(None);
    if entries.is_empty() {
        items.push(empty_hint("none"));
        paths.push(None);
        bodies.push(None);
    } else {
        for (item, path, body) in entries {
            items.push(item);
            paths.push(path);
            bodies.push(body);
        }
    }
}

// -- Item builders --------------------------------------------------------

pub(crate) fn skill_item(s: &lazyclaude::sources::Skill) -> ListItem<'static> {
    let (badge, color) = if s.user_invocable {
        ("[inv]", Color::Green)
    } else {
        ("[int]", Color::DarkGray)
    };
    ListItem::new(Line::from(vec![
        Span::styled("    ", Style::default()),
        Span::styled(s.name.clone(), Style::default().fg(color)),
        Span::styled(format!("  {badge}"), Style::default().fg(color)),
    ]))
}

pub(crate) fn agent_item(a: &lazyclaude::sources::Agent) -> ListItem<'static> {
    let mut spans = vec![
        Span::styled("    ", Style::default()),
        Span::styled(a.name.clone(), Style::default().fg(Color::Green)),
    ];
    if !a.model.is_empty() {
        spans.push(Span::styled(
            format!("  {}", a.model),
            Style::default().fg(Color::Yellow),
        ));
    }
    ListItem::new(Line::from(spans))
}

pub(crate) fn mcp_item(s: &lazyclaude::sources::McpServer) -> ListItem<'static> {
    let (badge, badge_color, name_color) = if s.disabled {
        ("  \u{25cf}", Color::Red, Color::DarkGray)
    } else {
        ("  \u{25cf}", Color::Green, Color::White)
    };
    let cmd = format!("{} {}", s.command, s.args.join(" "));
    ListItem::new(Line::from(vec![
        Span::styled("   ", Style::default()),
        Span::styled(badge, Style::default().fg(badge_color)),
        Span::styled(format!(" {}", s.name), Style::default().fg(name_color)),
        Span::styled(format!("  {cmd}"), Style::default().fg(Color::DarkGray)),
    ]))
}

pub(crate) fn claude_md_item(f: &lazyclaude::sources::ClaudeMdFile) -> ListItem<'static> {
    let size = if f.size < 1024 {
        format!("{} B", f.size)
    } else {
        format!("{:.1} KB", f.size as f64 / 1024.0)
    };
    let tag = if f.file_type == "rule" { " [rule]" } else { "" };
    ListItem::new(Line::from(vec![
        Span::styled("    ", Style::default()),
        Span::styled(f.name.clone(), Style::default().fg(Color::Green)),
        Span::styled(
            format!("{tag}  {size}"),
            Style::default().fg(Color::DarkGray),
        ),
    ]))
}
