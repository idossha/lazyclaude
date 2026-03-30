use std::path::PathBuf;

use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};

use crate::app::{App, View, SECTIONS};

/// Views that show a content preview pane on the right.
fn has_preview(view: View) -> bool {
    matches!(
        view,
        View::Memory | View::Skills | View::Agents | View::ClaudeMd
    )
}

pub fn render(frame: &mut Frame, app: &mut App) {
    let area = frame.area();

    if app.view == View::McpSearch {
        render_mcp_search(frame, app, area);
        return;
    }

    let section_idx = match app.view {
        View::Memory => 0,
        View::Skills => 1,
        View::Mcp => 2,
        View::Settings => 3,
        View::Hooks => 4,
        View::ClaudeMd => 5,
        View::Keybindings => 6,
        View::Agents => 7,
        _ => return,
    };
    let section = &SECTIONS[section_idx];

    let hints = match app.view {
        View::Settings => " a=Allow  D=Deny  d=Delete  /=Filter  ?=Help  BS=Back ",
        View::Mcp => " a=Add  d=Remove  t=Toggle  s=Search  /=Filter  ?=Help  BS=Back ",
        View::Memory | View::Skills | View::Agents | View::ClaudeMd => {
            " e=Edit  J/K=Scroll preview  /=Filter  ?=Help  BS=Back "
        }
        _ => " /=Filter  ?=Help  BS=Back ",
    };

    let count = section.count(&app.data);
    let title = if app.filter.is_empty() {
        format!(" {} — {} items ", section.label, count)
    } else {
        format!(" {} — filter: \"{}\" ", section.label, app.filter)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .title_bottom(Line::from(Span::styled(
            hints,
            Style::default().fg(Color::DarkGray),
        )));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if app.show_help {
        render_view_help(frame, inner, &app.view);
        return;
    }

    let (items, paths, bodies) = build_items(app);
    app.item_paths = paths;
    app.item_bodies = bodies;

    if has_preview(app.view) {
        render_with_preview(frame, app, inner, items);
    } else {
        let list = List::new(items)
            .highlight_style(highlight_style())
            .highlight_symbol(" > ");
        frame.render_stateful_widget(list, inner, &mut app.list_state);
    }
}

fn highlight_style() -> Style {
    Style::default()
        .bg(Color::DarkGray)
        .fg(Color::White)
        .add_modifier(Modifier::BOLD)
}

/// Render a horizontal split: file list on left, content preview on right.
fn render_with_preview(frame: &mut Frame, app: &mut App, area: Rect, items: Vec<ListItem<'static>>) {
    let chunks =
        Layout::horizontal([Constraint::Percentage(40), Constraint::Percentage(60)]).split(area);

    // Left: file list
    let list = List::new(items)
        .highlight_style(highlight_style())
        .highlight_symbol(" > ");
    frame.render_stateful_widget(list, chunks[0], &mut app.list_state);

    // Right: content preview
    let idx = app.list_state.selected().unwrap_or(0);
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

    let lines = render_markdown(content, preview_inner.width as usize);
    let paragraph = Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .scroll((app.preview_scroll as u16, 0));
    frame.render_widget(paragraph, preview_inner);
}

/// Simple markdown-aware line rendering for the preview pane.
fn render_markdown(content: &str, _width: usize) -> Vec<Line<'static>> {
    let mut lines = Vec::new();

    for raw_line in content.lines() {
        let trimmed = raw_line.trim_start();

        if trimmed.starts_with("# ") {
            // H1
            lines.push(Line::from(Span::styled(
                format!(" {}", &trimmed[2..]),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )));
        } else if trimmed.starts_with("## ") {
            // H2
            lines.push(Line::from(Span::styled(
                format!(" {}", &trimmed[3..]),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )));
        } else if trimmed.starts_with("### ") {
            // H3
            lines.push(Line::from(Span::styled(
                format!(" {}", &trimmed[4..]),
                Style::default().fg(Color::Yellow),
            )));
        } else if trimmed.starts_with("---") && trimmed.chars().all(|c| c == '-') {
            // Frontmatter delimiter or horizontal rule
            lines.push(Line::from(Span::styled(
                " ---",
                Style::default().fg(Color::DarkGray),
            )));
        } else if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
            // List item
            lines.push(Line::from(vec![
                Span::styled(" ", Style::default()),
                Span::styled("  ", Style::default().fg(Color::Green)),
                Span::styled(
                    trimmed[2..].to_string(),
                    Style::default().fg(Color::White),
                ),
            ]));
        } else if trimmed.starts_with("```") {
            // Code fence
            lines.push(Line::from(Span::styled(
                format!(" {trimmed}"),
                Style::default().fg(Color::DarkGray),
            )));
        } else if trimmed.contains(':') && !trimmed.starts_with(' ') && lines.len() < 20 {
            // Frontmatter-like key: value (early in file)
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

// ── Shared helpers ─────────────────────────────────────────────────────

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

/// Push a header + items for one scope.
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

// ── Build items + path map + body map ──────────────────────────────────

type ItemRow = (ListItem<'static>, Option<PathBuf>, Option<String>);

fn build_items(app: &App) -> (Vec<ListItem<'static>>, Vec<Option<PathBuf>>, Vec<Option<String>>) {
    let fl = app.filter.to_lowercase();
    let matches = |s: &str| app.filter.is_empty() || s.to_lowercase().contains(&fl);

    let mut items = Vec::new();
    let mut paths = Vec::new();
    let mut bodies = Vec::new();

    match app.view {
        View::Memory => {
            for f in &app.data.memory.files {
                if !matches(&f.name) && !matches(&f.description) { continue; }
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

        View::Skills => {
            let proj: Vec<_> = app.data.skills.iter().filter(|s| s.scope == "project").collect();
            let user: Vec<_> = app.data.skills.iter().filter(|s| s.scope == "user").collect();
            let proj_entries: Vec<ItemRow> = proj.iter()
                .filter(|s| matches(&s.name) || matches(&s.description))
                .map(|s| (skill_item(s), Some(s.path.clone()), Some(s.body.clone())))
                .collect();
            let user_entries: Vec<ItemRow> = user.iter()
                .filter(|s| matches(&s.name) || matches(&s.description))
                .map(|s| (skill_item(s), Some(s.path.clone()), Some(s.body.clone())))
                .collect();
            push_scope_group(&mut items, &mut paths, &mut bodies, &format!("Project ({})", proj.len()), proj_entries);
            push_scope_group(&mut items, &mut paths, &mut bodies, &format!("User ({})", user.len()), user_entries);
        }

        View::Mcp => {
            let proj_entries: Vec<ItemRow> = app.data.mcp.project.iter()
                .filter(|s| matches(&s.name))
                .map(|s| (mcp_item(s), None, None))
                .collect();
            let user_entries: Vec<ItemRow> = app.data.mcp.user.iter()
                .filter(|s| matches(&s.name))
                .map(|s| (mcp_item(s), None, None))
                .collect();
            push_scope_group(&mut items, &mut paths, &mut bodies, &format!("Project ({})", app.data.mcp.project.len()), proj_entries);
            push_scope_group(&mut items, &mut paths, &mut bodies, &format!("User ({})", app.data.mcp.user.len()), user_entries);
        }

        View::Settings => {
            let perms = &app.data.settings.permissions;
            if !perms.allow.is_empty() {
                items.push(ListItem::new(Line::from(Span::styled(
                    "  Allow", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
                ))));
                paths.push(None); bodies.push(None);
                for rule in &perms.allow {
                    if !matches(&rule.rule) { continue; }
                    items.push(ListItem::new(Line::from(vec![
                        Span::styled("    ", Style::default()),
                        Span::styled(rule.rule.clone(), Style::default().fg(Color::White)),
                        Span::styled(format!("  [{}]", rule.scope), Style::default().fg(Color::Cyan)),
                    ])));
                    paths.push(None); bodies.push(None);
                }
            }
            if !perms.deny.is_empty() {
                items.push(ListItem::new(Line::from(Span::styled(
                    "  Deny", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                ))));
                paths.push(None); bodies.push(None);
                for rule in &perms.deny {
                    if !matches(&rule.rule) { continue; }
                    items.push(ListItem::new(Line::from(vec![
                        Span::styled("    ", Style::default()),
                        Span::styled(rule.rule.clone(), Style::default().fg(Color::White)),
                        Span::styled(format!("  [{}]", rule.scope), Style::default().fg(Color::Red)),
                    ])));
                    paths.push(None); bodies.push(None);
                }
            }
            if let Some(obj) = app.data.settings.effective.as_object() {
                let general: Vec<_> = obj.iter()
                    .filter(|(k, _)| *k != "permissions" && *k != "hooks")
                    .collect();
                if !general.is_empty() {
                    items.push(ListItem::new(Line::from(Span::styled(
                        "  General", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                    ))));
                    paths.push(None); bodies.push(None);
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
                        paths.push(None); bodies.push(None);
                    }
                }
            }
        }

        View::Hooks => {
            let proj_hooks: Vec<_> = app.data.hooks.iter().filter(|h| h.scope != "user").collect();
            let user_hooks: Vec<_> = app.data.hooks.iter().filter(|h| h.scope == "user").collect();
            items.push(scope_header(&format!("Project ({})", proj_hooks.len())));
            paths.push(None); bodies.push(None);
            if proj_hooks.is_empty() { items.push(empty_hint("none")); paths.push(None); bodies.push(None); }
            push_hook_items(&mut items, &mut paths, &mut bodies, &proj_hooks, &matches);
            items.push(scope_header(&format!("User ({})", user_hooks.len())));
            paths.push(None); bodies.push(None);
            if user_hooks.is_empty() { items.push(empty_hint("none")); paths.push(None); bodies.push(None); }
            push_hook_items(&mut items, &mut paths, &mut bodies, &user_hooks, &matches);
        }

        View::ClaudeMd => {
            let proj: Vec<_> = app.data.claude_md.iter().filter(|f| f.scope == "project").collect();
            let user: Vec<_> = app.data.claude_md.iter().filter(|f| f.scope == "user").collect();
            let proj_entries: Vec<ItemRow> = proj.iter()
                .filter(|f| matches(&f.name))
                .map(|f| (claude_md_item(f), Some(f.path.clone()), Some(f.content.clone())))
                .collect();
            let user_entries: Vec<ItemRow> = user.iter()
                .filter(|f| matches(&f.name))
                .map(|f| (claude_md_item(f), Some(f.path.clone()), Some(f.content.clone())))
                .collect();
            push_scope_group(&mut items, &mut paths, &mut bodies, &format!("Project ({})", proj.len()), proj_entries);
            push_scope_group(&mut items, &mut paths, &mut bodies, &format!("User ({})", user.len()), user_entries);
        }

        View::Keybindings => {
            for b in &app.data.keybindings {
                if !matches(&b.key) && !matches(&b.command) { continue; }
                let mut spans = vec![
                    Span::styled("  ", Style::default()),
                    Span::styled(b.key.clone(), Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                    Span::styled(" -> ", Style::default().fg(Color::DarkGray)),
                    Span::styled(b.command.clone(), Style::default().fg(Color::White)),
                ];
                if !b.context.is_empty() {
                    spans.push(Span::styled(format!("  [{}]", b.context), Style::default().fg(Color::Cyan)));
                }
                items.push(ListItem::new(Line::from(spans)));
                paths.push(None); bodies.push(None);
            }
        }

        View::Agents => {
            let proj: Vec<_> = app.data.agents.iter().filter(|a| a.scope == "project").collect();
            let user: Vec<_> = app.data.agents.iter().filter(|a| a.scope == "user").collect();
            let proj_entries: Vec<ItemRow> = proj.iter()
                .filter(|a| matches(&a.name) || matches(&a.description))
                .map(|a| (agent_item(a), Some(a.path.clone()), Some(a.body.clone())))
                .collect();
            let user_entries: Vec<ItemRow> = user.iter()
                .filter(|a| matches(&a.name) || matches(&a.description))
                .map(|a| (agent_item(a), Some(a.path.clone()), Some(a.body.clone())))
                .collect();
            push_scope_group(&mut items, &mut paths, &mut bodies, &format!("Project ({})", proj.len()), proj_entries);
            push_scope_group(&mut items, &mut paths, &mut bodies, &format!("User ({})", user.len()), user_entries);
        }

        _ => {}
    }

    (items, paths, bodies)
}

// ── Item builders ──────────────────────────────────────────────────────

fn skill_item(s: &ccm::sources::Skill) -> ListItem<'static> {
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

fn agent_item(a: &ccm::sources::Agent) -> ListItem<'static> {
    let mut spans = vec![
        Span::styled("    ", Style::default()),
        Span::styled(a.name.clone(), Style::default().fg(Color::Green)),
    ];
    if !a.model.is_empty() {
        spans.push(Span::styled(format!("  {}", a.model), Style::default().fg(Color::Yellow)));
    }
    ListItem::new(Line::from(spans))
}

fn mcp_item(s: &ccm::sources::McpServer) -> ListItem<'static> {
    let status = if s.disabled { " (disabled)" } else { "" };
    let color = if s.disabled { Color::DarkGray } else { Color::Green };
    let cmd = format!("{} {}", s.command, s.args.join(" "));
    ListItem::new(Line::from(vec![
        Span::styled("    ", Style::default()),
        Span::styled(s.name.clone(), Style::default().fg(color)),
        Span::styled(status.to_string(), Style::default().fg(Color::DarkGray)),
        Span::styled(format!("  {cmd}"), Style::default().fg(Color::DarkGray)),
    ]))
}

fn claude_md_item(f: &ccm::sources::ClaudeMdFile) -> ListItem<'static> {
    let size = if f.size < 1024 {
        format!("{} B", f.size)
    } else {
        format!("{:.1} KB", f.size as f64 / 1024.0)
    };
    let tag = if f.file_type == "rule" { " [rule]" } else { "" };
    ListItem::new(Line::from(vec![
        Span::styled("    ", Style::default()),
        Span::styled(f.name.clone(), Style::default().fg(Color::Green)),
        Span::styled(format!("{tag}  {size}"), Style::default().fg(Color::DarkGray)),
    ]))
}

fn push_hook_items(
    items: &mut Vec<ListItem<'static>>,
    paths: &mut Vec<Option<PathBuf>>,
    bodies: &mut Vec<Option<String>>,
    hooks: &[&ccm::sources::Hook],
    matches: &dyn Fn(&str) -> bool,
) {
    let mut cur_event = String::new();
    for hook in hooks {
        if !matches(&hook.command) && !matches(&hook.event) && !matches(&hook.matcher) { continue; }
        if hook.event != cur_event {
            cur_event.clone_from(&hook.event);
            items.push(ListItem::new(Line::from(Span::styled(
                format!("    {cur_event}"),
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            ))));
            paths.push(None); bodies.push(None);
        }
        items.push(ListItem::new(Line::from(vec![
            Span::styled("      ", Style::default()),
            Span::styled(hook.matcher.clone(), Style::default().fg(Color::Green)),
            Span::styled(" -> ", Style::default().fg(Color::DarkGray)),
            Span::styled(hook.command.clone(), Style::default().fg(Color::White)),
        ])));
        paths.push(None); bodies.push(None);
    }
}

// ── Help ───────────────────────────────────────────────────────────────

fn render_view_help(frame: &mut Frame, area: Rect, view: &View) {
    let help = match view {
        View::Settings => vec![
            "", "  a         Add allow permission",
            "  D         Add deny permission",
            "  d         Delete permission at cursor",
            "  j/k       Navigate",
            "  /         Filter",
            "  BS/h      Back to dashboard",
            "  ?         Close help",
        ],
        View::Mcp => vec![
            "", "  a         Add MCP server",
            "  d         Remove server at cursor",
            "  t         Toggle enabled/disabled",
            "  s         Search npm registry",
            "  j/k       Navigate",
            "  /         Filter",
            "  BS/h      Back to dashboard",
        ],
        View::Memory | View::Skills | View::ClaudeMd | View::Agents => vec![
            "", "  e         Edit in $EDITOR",
            "  J/K       Scroll preview",
            "  j/k       Navigate items",
            "  /         Filter",
            "  BS/h      Back to dashboard",
            "  ?         Close help",
        ],
        _ => vec![
            "", "  j/k       Navigate",
            "  /         Filter",
            "  BS/h      Back to dashboard",
        ],
    };

    let lines: Vec<Line> = help
        .iter()
        .map(|s| Line::from(Span::styled(*s, Style::default().fg(Color::White))))
        .collect();
    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, area);
}

fn render_mcp_search(frame: &mut Frame, app: &mut App, area: Rect) {
    let count = app.registry_results.len();
    let title = format!(" MCP Registry — {count} results ");
    let hints = " Enter=Install  s=New Search  BS=Back ";

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .title_bottom(Line::from(Span::styled(hints, Style::default().fg(Color::DarkGray))));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let items: Vec<ListItem> = app.registry_results.iter().map(|entry| {
        let desc = if entry.description.len() > 60 {
            format!("{}...", &entry.description[..57])
        } else {
            entry.description.clone()
        };
        ListItem::new(Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled(entry.name.clone(), Style::default().fg(Color::Green)),
            Span::styled(format!("  v{}", entry.version), Style::default().fg(Color::Cyan)),
            Span::styled(format!("  {desc}"), Style::default().fg(Color::DarkGray)),
        ]))
    }).collect();

    let list = List::new(items).highlight_style(highlight_style()).highlight_symbol(" > ");
    frame.render_stateful_widget(list, inner, &mut app.list_state);
}
