use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::app::{App, Pane, View, SECTIONS};
use ccm::sources::SourceData;

pub fn render(frame: &mut Frame, app: &mut App) {
    let area = frame.area();

    // Outer block
    let outer = Block::default()
        .borders(Borders::ALL)
        .title(" ccm — Claude Code Manager ")
        .title_bottom(Line::from(vec![
            Span::styled(" q", Style::default().fg(Color::Yellow)),
            Span::styled("=Quit ", Style::default().fg(Color::DarkGray)),
            Span::styled("?", Style::default().fg(Color::Yellow)),
            Span::styled("=Help ", Style::default().fg(Color::DarkGray)),
            Span::styled("/", Style::default().fg(Color::Yellow)),
            Span::styled("=Filter ", Style::default().fg(Color::DarkGray)),
            Span::styled("Tab", Style::default().fg(Color::Yellow)),
            Span::styled("=Pane ", Style::default().fg(Color::DarkGray)),
            Span::styled("Enter", Style::default().fg(Color::Yellow)),
            Span::styled("=Zoom ", Style::default().fg(Color::DarkGray)),
        ]));
    let inner = outer.inner(area);
    frame.render_widget(outer, area);

    // Split into left (30%) and right (70%)
    let chunks =
        Layout::horizontal([Constraint::Percentage(30), Constraint::Percentage(70)]).split(inner);

    render_left(frame, app, chunks[0]);
    if app.show_help {
        render_help(frame, chunks[1]);
    } else {
        render_right(frame, app, chunks[1]);
    }
}

fn render_left(frame: &mut Frame, app: &mut App, area: Rect) {
    let border_color = match app.focused_pane {
        Pane::Left => Color::Cyan,
        Pane::Right => Color::DarkGray,
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(" Sections ");

    let items: Vec<ListItem> = SECTIONS
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let count = s.count(&app.data);
            let label = format!("  {} ({})", s.label, count);
            let style = if i == app.section_index {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            ListItem::new(label).style(style)
        })
        .collect();

    let list = List::new(items)
        .block(block)
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(" > ");

    frame.render_stateful_widget(list, area, &mut app.list_state);
}

fn render_right(frame: &mut Frame, app: &App, area: Rect) {
    let border_color = match app.focused_pane {
        Pane::Left => Color::DarkGray,
        Pane::Right => Color::Cyan,
    };
    let section = &SECTIONS[app.section_index];
    let view = section.view;

    // Sections with dual scope get a vertical split
    let has_dual_scope = matches!(
        view,
        View::Skills | View::Agents | View::ClaudeMd | View::Mcp | View::Hooks | View::Settings
    );

    if has_dual_scope {
        render_right_split(frame, app, area, border_color, section.label, view);
    } else {
        render_right_single(frame, app, area, border_color, section);
    }
}

/// Single-scope right pane (Memory, Keybindings)
fn render_right_single(
    frame: &mut Frame,
    app: &App,
    area: Rect,
    border_color: Color,
    section: &crate::app::SectionDef,
) {
    let count = section.count(&app.data);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(format!(" {} — {} items ", section.label, count));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let lines = section.preview(&app.data, &app.filter, inner.width as usize);
    let paragraph = Paragraph::new(lines).scroll((app.scroll as u16, 0));
    frame.render_widget(paragraph, inner);
}

/// Dual-scope right pane: top = User, bottom = Project
fn render_right_split(
    frame: &mut Frame,
    app: &App,
    area: Rect,
    border_color: Color,
    label: &str,
    view: View,
) {
    let fl = app.filter.to_lowercase();
    let matches = |s: &str| app.filter.is_empty() || s.to_lowercase().contains(&fl);

    let (user_lines, project_lines) = build_scope_lines(&app.data, view, &matches);

    let user_count = user_lines.len();
    let proj_count = project_lines.len();
    let total = user_count + proj_count;
    let title = if app.filter.is_empty() {
        format!(" {label} — {total} items ")
    } else {
        format!(" {label} — filter: \"{}\" ", app.filter)
    };

    let outer = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(title);
    let inner = outer.inner(area);
    frame.render_widget(outer, area);

    // Split vertically: project on top, user on bottom
    let chunks =
        Layout::vertical([Constraint::Percentage(50), Constraint::Percentage(50)]).split(inner);

    // Project pane (top)
    let proj_block = Block::default()
        .borders(Borders::BOTTOM)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(Span::styled(
            format!(" Project ({proj_count}) "),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ));
    let proj_inner = proj_block.inner(chunks[0]);
    frame.render_widget(proj_block, chunks[0]);
    let proj_para = Paragraph::new(project_lines).scroll((app.scroll as u16, 0));
    frame.render_widget(proj_para, proj_inner);

    // User pane (bottom)
    let user_block = Block::default().title(Span::styled(
        format!(" User ({user_count}) "),
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    ));
    let user_inner = user_block.inner(chunks[1]);
    frame.render_widget(user_block, chunks[1]);
    let user_para = Paragraph::new(user_lines);
    frame.render_widget(user_para, user_inner);
}

/// Build separate User / Project line lists for the dashboard preview.
fn build_scope_lines<'a>(
    data: &'a SourceData,
    view: View,
    matches: &dyn Fn(&str) -> bool,
) -> (Vec<Line<'a>>, Vec<Line<'a>>) {
    use ratatui::style::Style;

    let mut user = Vec::new();
    let mut project = Vec::new();

    match view {
        View::Skills => {
            for s in &data.skills {
                if !matches(&s.name) {
                    continue;
                }
                let (badge, color) = if s.user_invocable {
                    ("[inv]", Color::Green)
                } else {
                    ("[int]", Color::DarkGray)
                };
                let line = Line::from(vec![
                    Span::styled("  ", Style::default()),
                    Span::styled(s.name.as_str(), Style::default().fg(color)),
                    Span::styled(format!("  {badge}"), Style::default().fg(color)),
                ]);
                if s.scope == "user" {
                    user.push(line);
                } else {
                    project.push(line);
                }
            }
        }
        View::Agents => {
            for a in &data.agents {
                if !matches(&a.name) {
                    continue;
                }
                let mut spans = vec![
                    Span::styled("  ", Style::default()),
                    Span::styled(a.name.as_str(), Style::default().fg(Color::Green)),
                ];
                if !a.model.is_empty() {
                    spans.push(Span::styled(
                        format!("  {}", a.model),
                        Style::default().fg(Color::DarkGray),
                    ));
                }
                let line = Line::from(spans);
                if a.scope == "user" {
                    user.push(line);
                } else {
                    project.push(line);
                }
            }
        }
        View::ClaudeMd => {
            for f in &data.claude_md {
                if !matches(&f.name) {
                    continue;
                }
                let size = if f.size < 1024 {
                    format!("{} B", f.size)
                } else {
                    format!("{:.1} KB", f.size as f64 / 1024.0)
                };
                let tag = if f.file_type == "rule" { " [rule]" } else { "" };
                let line = Line::from(vec![
                    Span::styled("  ", Style::default()),
                    Span::styled(f.name.as_str(), Style::default().fg(Color::Green)),
                    Span::styled(
                        format!("{tag}  {size}"),
                        Style::default().fg(Color::DarkGray),
                    ),
                ]);
                if f.scope == "user" {
                    user.push(line);
                } else {
                    project.push(line);
                }
            }
        }
        View::Mcp => {
            for s in &data.mcp.user {
                if !matches(&s.name) {
                    continue;
                }
                let color = if s.disabled {
                    Color::DarkGray
                } else {
                    Color::Green
                };
                user.push(Line::from(vec![
                    Span::styled("  ", Style::default()),
                    Span::styled(s.name.as_str(), Style::default().fg(color)),
                    if s.disabled {
                        Span::styled(" (disabled)", Style::default().fg(Color::DarkGray))
                    } else {
                        Span::styled("", Style::default())
                    },
                ]));
            }
            for s in &data.mcp.project {
                if !matches(&s.name) {
                    continue;
                }
                let color = if s.disabled {
                    Color::DarkGray
                } else {
                    Color::Green
                };
                project.push(Line::from(vec![
                    Span::styled("  ", Style::default()),
                    Span::styled(s.name.as_str(), Style::default().fg(color)),
                    if s.disabled {
                        Span::styled(" (disabled)", Style::default().fg(Color::DarkGray))
                    } else {
                        Span::styled("", Style::default())
                    },
                ]));
            }
        }
        View::Hooks => {
            let mut user_event = String::new();
            let mut proj_event = String::new();
            for h in &data.hooks {
                if !matches(&h.command) && !matches(&h.event) {
                    continue;
                }
                let (target, cur_event) = if h.scope == "user" {
                    (&mut user, &mut user_event)
                } else {
                    (&mut project, &mut proj_event)
                };
                if h.event != *cur_event {
                    *cur_event = h.event.clone();
                    target.push(Line::from(Span::styled(
                        format!("  {}", cur_event),
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    )));
                }
                target.push(Line::from(vec![
                    Span::styled("    ", Style::default()),
                    Span::styled(h.matcher.as_str(), Style::default().fg(Color::Green)),
                    Span::styled(" -> ", Style::default().fg(Color::DarkGray)),
                    Span::styled(h.command.as_str(), Style::default().fg(Color::White)),
                ]));
            }
        }
        View::Settings => {
            let p = &data.settings.permissions;
            for r in &p.allow {
                if !matches(&r.rule) {
                    continue;
                }
                let line = Line::from(vec![
                    Span::styled("  ", Style::default()),
                    Span::styled("+ ", Style::default().fg(Color::Green)),
                    Span::styled(r.rule.as_str(), Style::default().fg(Color::White)),
                ]);
                if r.scope == "user" {
                    user.push(line);
                } else {
                    project.push(line);
                }
            }
            for r in &p.deny {
                if !matches(&r.rule) {
                    continue;
                }
                let line = Line::from(vec![
                    Span::styled("  ", Style::default()),
                    Span::styled("- ", Style::default().fg(Color::Red)),
                    Span::styled(r.rule.as_str(), Style::default().fg(Color::White)),
                ]);
                if r.scope == "user" {
                    user.push(line);
                } else {
                    project.push(line);
                }
            }
        }
        _ => {}
    }

    (user, project)
}

fn render_help(frame: &mut Frame, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(" Help ");

    let help_text = vec![
        Line::from(""),
        help_line("j/k", "Navigate sections"),
        help_line("Enter/l", "Zoom into section"),
        help_line("h/BS", "Go back"),
        help_line("Tab", "Switch pane"),
        help_line("/", "Filter items"),
        help_line("?", "Toggle help"),
        help_line("R", "Refresh data"),
        help_line("q", "Quit"),
        Line::from(""),
        Line::from(Span::styled(
            "  In zoomed views:",
            Style::default().fg(Color::DarkGray),
        )),
        help_line("a", "Add item"),
        help_line("d", "Delete item at cursor"),
        help_line("D", "Add deny permission"),
        help_line("e", "Edit in $EDITOR"),
        help_line("t", "Toggle (MCP)"),
        help_line("s", "Search MCP registry"),
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
