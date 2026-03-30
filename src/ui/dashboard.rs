use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

use crate::app::{App, Focus, Panel, PANELS};

pub fn render(frame: &mut Frame, app: &mut App) {
    let area = frame.area();

    // Outer block
    let outer = Block::default()
        .borders(Borders::ALL)
        .title(" lazyclaude — Claude Code Manager ")
        .title_bottom(Line::from(vec![
            Span::styled(" q", Style::default().fg(Color::Yellow)),
            Span::styled("=Quit ", Style::default().fg(Color::DarkGray)),
            Span::styled("?", Style::default().fg(Color::Yellow)),
            Span::styled("=Help ", Style::default().fg(Color::DarkGray)),
            Span::styled("/", Style::default().fg(Color::Yellow)),
            Span::styled("=Filter ", Style::default().fg(Color::DarkGray)),
            Span::styled("Tab", Style::default().fg(Color::Yellow)),
            Span::styled("=Focus ", Style::default().fg(Color::DarkGray)),
            Span::styled("1-8", Style::default().fg(Color::Yellow)),
            Span::styled("=Panel ", Style::default().fg(Color::DarkGray)),
        ]));
    let inner = outer.inner(area);
    frame.render_widget(outer, area);

    // Split into left (30%) and right (70%)
    let chunks =
        Layout::horizontal([Constraint::Percentage(30), Constraint::Percentage(70)]).split(inner);

    render_panels(frame, app, chunks[0]);
    if app.show_help {
        render_help(frame, chunks[1]);
    } else {
        render_detail(frame, app, chunks[1]);
    }
}

fn render_panels(frame: &mut Frame, app: &mut App, area: Rect) {
    let border_color = match app.focus {
        Focus::Panels => Color::Cyan,
        Focus::Detail => Color::DarkGray,
    };

    // Show project name in the panel title
    let project_label = if app.selected_project == 0 {
        "Global".to_string()
    } else if let Some(p) = app.projects.get(app.selected_project - 1) {
        p.short_name.clone()
    } else {
        "Global".to_string()
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(format!(" Panels [{project_label}] "));

    let items: Vec<ListItem> = PANELS
        .iter()
        .map(|panel| {
            let count = panel.count(app);
            let label = format!("  {} {} ({})", panel.index() + 1, panel.label(), count);
            let style = if *panel == app.active_panel {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            ListItem::new(label).style(style)
        })
        .collect();

    let mut list_state = ListState::default();
    list_state.select(Some(app.active_panel.index()));

    let list = List::new(items)
        .block(block)
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(" > ");

    frame.render_stateful_widget(list, area, &mut list_state);
}

fn render_detail(frame: &mut Frame, app: &mut App, area: Rect) {
    let border_color = match app.focus {
        Focus::Panels => Color::DarkGray,
        Focus::Detail => Color::Cyan,
    };

    let panel = app.active_panel;
    let count = panel.count(app);
    let title = if app.filter.is_empty() {
        format!(" {} — {} items ", panel.label(), count)
    } else {
        format!(" {} — filter: \"{}\" ", panel.label(), app.filter)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(title);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let (items, paths, bodies) = build_detail_items(app);
    app.item_paths = paths;
    app.item_bodies = bodies;

    // For panels with content preview, split horizontally
    let has_preview = matches!(
        panel,
        Panel::Config | Panel::Memory | Panel::Skills | Panel::Agents
    );

    if has_preview && !items.is_empty() {
        let chunks =
            Layout::horizontal([Constraint::Percentage(40), Constraint::Percentage(60)])
                .split(inner);

        let mut list_state = ListState::default();
        list_state.select(Some(app.panel_offset()));

        let list = List::new(items)
            .highlight_style(highlight_style())
            .highlight_symbol(" > ");
        frame.render_stateful_widget(list, chunks[0], &mut list_state);

        // Preview pane
        let idx = app.panel_offset();
        let content = app
            .item_bodies
            .get(idx)
            .and_then(|b| b.as_deref())
            .unwrap_or("");

        let preview_block = Block::default()
            .borders(Borders::LEFT)
            .border_style(Style::default().fg(Color::DarkGray))
            .title(Span::styled(
                " Preview ",
                Style::default().fg(Color::DarkGray),
            ));
        let preview_inner = preview_block.inner(chunks[1]);
        frame.render_widget(preview_block, chunks[1]);

        let lines = render_markdown(content);
        let paragraph = Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .scroll((app.detail_scroll as u16, 0));
        frame.render_widget(paragraph, preview_inner);
    } else {
        let mut list_state = ListState::default();
        list_state.select(Some(app.panel_offset()));

        let list = List::new(items)
            .highlight_style(highlight_style())
            .highlight_symbol(" > ");
        frame.render_stateful_widget(list, inner, &mut list_state);
    }
}

fn highlight_style() -> Style {
    Style::default()
        .bg(Color::DarkGray)
        .fg(Color::White)
        .add_modifier(Modifier::BOLD)
}

// ── Build detail items ──────────────────────────────────────────────────

use std::path::PathBuf;

type ItemRow = (ListItem<'static>, Option<PathBuf>, Option<String>);

fn build_detail_items(
    app: &App,
) -> (Vec<ListItem<'static>>, Vec<Option<PathBuf>>, Vec<Option<String>>) {
    let fl = app.filter.to_lowercase();
    let matches = |s: &str| app.filter.is_empty() || s.to_lowercase().contains(&fl);

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
            let proj: Vec<_> = app.data.claude_md.iter().filter(|f| f.scope == "project").collect();
            let user: Vec<_> = app.data.claude_md.iter().filter(|f| f.scope == "user").collect();
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
            let proj: Vec<_> = app.data.skills.iter().filter(|s| s.scope == "project").collect();
            let user: Vec<_> = app.data.skills.iter().filter(|s| s.scope == "user").collect();
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
            let proj: Vec<_> = app.data.agents.iter().filter(|a| a.scope == "project").collect();
            let user: Vec<_> = app.data.agents.iter().filter(|a| a.scope == "user").collect();
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
            if app.mcp_search_active {
                for entry in &app.registry_results {
                    if !matches(&entry.name) {
                        continue;
                    }
                    let desc = if entry.description.len() > 60 {
                        format!("{}...", &entry.description[..57])
                    } else {
                        entry.description.clone()
                    };
                    items.push(ListItem::new(Line::from(vec![
                        Span::styled("  ", Style::default()),
                        Span::styled(entry.name.clone(), Style::default().fg(Color::Green)),
                        Span::styled(
                            format!("  v{}", entry.version),
                            Style::default().fg(Color::Cyan),
                        ),
                        Span::styled(format!("  {desc}"), Style::default().fg(Color::DarkGray)),
                    ])));
                    paths.push(None);
                    bodies.push(None);
                }
            } else {
                let proj_entries: Vec<ItemRow> = app
                    .data
                    .mcp
                    .project
                    .iter()
                    .filter(|s| matches(&s.name))
                    .map(|s| (mcp_item(s), None, None))
                    .collect();
                let user_entries: Vec<ItemRow> = app
                    .data
                    .mcp
                    .user
                    .iter()
                    .filter(|s| matches(&s.name))
                    .map(|s| (mcp_item(s), None, None))
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
                for rule in &perms.allow {
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
                    paths.push(None);
                    bodies.push(None);
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
                for rule in &perms.deny {
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
                    paths.push(None);
                    bodies.push(None);
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
                    bodies.push(None);
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
                    bodies.push(None);
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
                        items.push(ListItem::new(Line::from(vec![
                            Span::styled("    ", Style::default()),
                            Span::styled(key.to_string(), Style::default().fg(Color::Cyan)),
                            Span::styled(": ", Style::default().fg(Color::DarkGray)),
                            Span::styled(val_str, Style::default().fg(Color::White)),
                        ])));
                        paths.push(None);
                        bodies.push(None);
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
    }

    (items, paths, bodies)
}

// ── Scope group helper ──────────────────────────────────────────────────

fn scope_header(label: &str) -> ListItem<'static> {
    ListItem::new(Line::from(Span::styled(
        format!("  {label}"),
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    )))
}

fn empty_hint(msg: &str) -> ListItem<'static> {
    ListItem::new(Line::from(Span::styled(
        format!("    {msg}"),
        Style::default().fg(Color::DarkGray),
    )))
}

fn push_scope_group(
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

// ── Item builders ───────────────────────────────────────────────────────

fn skill_item(s: &lazyclaude::sources::Skill) -> ListItem<'static> {
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

fn agent_item(a: &lazyclaude::sources::Agent) -> ListItem<'static> {
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

fn mcp_item(s: &lazyclaude::sources::McpServer) -> ListItem<'static> {
    let (badge, badge_color, name_color) = if s.disabled {
        ("  ●", Color::Red, Color::DarkGray)
    } else {
        ("  ●", Color::Green, Color::White)
    };
    let cmd = format!("{} {}", s.command, s.args.join(" "));
    ListItem::new(Line::from(vec![
        Span::styled("   ", Style::default()),
        Span::styled(badge, Style::default().fg(badge_color)),
        Span::styled(format!(" {}", s.name), Style::default().fg(name_color)),
        Span::styled(format!("  {cmd}"), Style::default().fg(Color::DarkGray)),
    ]))
}

fn claude_md_item(f: &lazyclaude::sources::ClaudeMdFile) -> ListItem<'static> {
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

// ── Markdown preview ────────────────────────────────────────────────────

fn render_markdown(content: &str) -> Vec<Line<'static>> {
    let mut lines = Vec::new();

    for raw_line in content.lines() {
        let trimmed = raw_line.trim_start();

        if trimmed.starts_with("# ") {
            lines.push(Line::from(Span::styled(
                format!(" {}", &trimmed[2..]),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )));
        } else if trimmed.starts_with("## ") {
            lines.push(Line::from(Span::styled(
                format!(" {}", &trimmed[3..]),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )));
        } else if trimmed.starts_with("### ") {
            lines.push(Line::from(Span::styled(
                format!(" {}", &trimmed[4..]),
                Style::default().fg(Color::Yellow),
            )));
        } else if trimmed.starts_with("---") && trimmed.chars().all(|c| c == '-') {
            lines.push(Line::from(Span::styled(
                " ---",
                Style::default().fg(Color::DarkGray),
            )));
        } else if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
            lines.push(Line::from(vec![
                Span::styled(" ", Style::default()),
                Span::styled("  ", Style::default().fg(Color::Green)),
                Span::styled(
                    trimmed[2..].to_string(),
                    Style::default().fg(Color::White),
                ),
            ]));
        } else if trimmed.starts_with("```") {
            lines.push(Line::from(Span::styled(
                format!(" {trimmed}"),
                Style::default().fg(Color::DarkGray),
            )));
        } else if trimmed.contains(':') && !trimmed.starts_with(' ') && lines.len() < 20 {
            if let Some((key, val)) = trimmed.split_once(':') {
                lines.push(Line::from(vec![
                    Span::styled(format!(" {}", key.trim()), Style::default().fg(Color::Cyan)),
                    Span::styled(": ", Style::default().fg(Color::DarkGray)),
                    Span::styled(val.trim().to_string(), Style::default().fg(Color::White)),
                ]));
            } else {
                lines.push(Line::from(Span::styled(
                    format!(" {raw_line}"),
                    Style::default().fg(Color::White),
                )));
            }
        } else if trimmed.is_empty() {
            lines.push(Line::from(""));
        } else {
            lines.push(Line::from(Span::styled(
                format!(" {raw_line}"),
                Style::default().fg(Color::White),
            )));
        }
    }

    if lines.is_empty() {
        lines.push(Line::from(Span::styled(
            " (empty)",
            Style::default().fg(Color::DarkGray),
        )));
    }

    lines
}

// ── Help ────────────────────────────────────────────────────────────────

fn render_help(frame: &mut Frame, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(" Help ");

    let help_text = vec![
        Line::from(""),
        help_line("1-8", "Switch panel"),
        help_line("j/k", "Navigate items"),
        help_line("J/K", "Scroll detail preview"),
        help_line("Enter", "Select project / confirm action"),
        help_line("l", "Focus detail pane"),
        help_line("h/BS", "Back to panels"),
        help_line("Tab", "Toggle panels/detail focus"),
        help_line("/", "Filter items"),
        help_line("?", "Toggle help"),
        help_line("R", "Refresh data"),
        help_line("q", "Quit"),
        Line::from(""),
        Line::from(Span::styled(
            "  Panel actions:",
            Style::default().fg(Color::DarkGray),
        )),
        help_line("e", "Edit in $EDITOR (Config/Memory/Skills/Agents)"),
        help_line("a", "Add item (Settings/MCP)"),
        help_line("d", "Delete item (Settings)"),
        help_line("D", "Add deny permission (Settings)"),
        help_line("t", "Toggle server (MCP)"),
        help_line("s", "Search registry (MCP)"),
    ];

    let paragraph = Paragraph::new(help_text).block(block);
    frame.render_widget(paragraph, area);
}

fn help_line<'a>(key: &'a str, desc: &'a str) -> Line<'a> {
    Line::from(vec![
        Span::styled(
            format!("  {:<10}", key),
            Style::default().fg(Color::Yellow),
        ),
        Span::styled(desc, Style::default().fg(Color::White)),
    ])
}
