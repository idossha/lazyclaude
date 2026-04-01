use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

use crate::app::{App, Focus, Panel, PANELS};

use super::detail::build_detail_items;
use super::help::render_help;
use super::markdown::render_markdown;
use super::search_view::render_search_overlay;
use super::stats_view::render_stats_dashboard;

pub fn render(frame: &mut Frame, app: &mut App) {
    let area = frame.area();

    // Split area into main content + status bar
    let main_chunks = Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).split(area);

    // Outer block (no title_bottom — hints moved to status bar)
    let outer = Block::default()
        .borders(Borders::ALL)
        .title(" lazyclaude — Claude Code Manager ");
    let inner = outer.inner(main_chunks[0]);
    frame.render_widget(outer, main_chunks[0]);

    // Split into left (30%) and right (70%)
    let chunks =
        Layout::horizontal([Constraint::Percentage(30), Constraint::Percentage(70)]).split(inner);

    render_panels(frame, app, chunks[0]);
    if app.show_help {
        render_help(frame, app.active_panel, app.detail_scroll, chunks[1]);
    } else {
        render_detail(frame, app, chunks[1]);
    }

    // Render persistent status bar
    render_status_bar(frame, app, main_chunks[1]);
}

fn render_panels(frame: &mut Frame, app: &mut App, area: Rect) {
    let border_color = match app.focus {
        Focus::Panels => Color::Cyan,
        _ => Color::DarkGray,
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
            let count = panel.item_count(app);
            let key = panel.key_label();
            let label = if *panel == Panel::Stats {
                format!("  {} {}", key, panel.label())
            } else {
                format!("  {} {} ({})", key, panel.label(), count)
            };
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
    // Search overlay takes over the detail pane
    if let Some(ref overlay) = app.search_overlay {
        render_search_overlay(frame, overlay, area);
        return;
    }

    // Show loading indicator while background search is in progress
    if app.search_receiver.is_some() {
        let source_label = app
            .search_source_pending
            .map(|s| s.label())
            .unwrap_or("...");
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .title(format!(" Loading {} ", source_label));
        let loading = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                "  Fetching from remote registry...",
                Style::default().fg(Color::Yellow),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "  Please wait.",
                Style::default().fg(Color::DarkGray),
            )),
        ])
        .block(block);
        frame.render_widget(loading, area);
        return;
    }

    let detail_border = match app.focus {
        Focus::Detail => Color::Cyan,
        _ => Color::DarkGray,
    };

    let panel = app.active_panel;

    // Stats uses a custom dashboard renderer
    if panel == Panel::Stats {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(detail_border))
            .title(Span::styled(" Stats ", Style::default().fg(Color::Cyan)));
        let inner = block.inner(area);
        frame.render_widget(block, area);
        render_stats_dashboard(frame, app, inner);
        return;
    }

    let item_count = panel.item_count(app);
    let title = if app.filter.is_empty() {
        format!(" {} — {} items ", panel.label(), item_count)
    } else {
        format!(" {} — filter: \"{}\" ", panel.label(), app.filter)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(detail_border))
        .title(title);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let (items, paths, bodies) = build_detail_items(app);
    app.item_paths = paths;
    app.item_bodies = bodies;

    // Clamp cursor so it never exceeds the (possibly filtered) item count
    let idx = app.active_panel.index();
    if !items.is_empty() {
        app.panel_offsets[idx] = app.panel_offsets[idx].min(items.len().saturating_sub(1));
    } else {
        app.panel_offsets[idx] = 0;
    }

    // For panels with content preview, split horizontally
    let has_preview = app.has_preview();

    if has_preview && !items.is_empty() {
        let chunks = Layout::horizontal([Constraint::Percentage(40), Constraint::Percentage(60)])
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

        let preview_focused = app.focus == Focus::Preview;
        let preview_border_color = if preview_focused {
            Color::Cyan
        } else {
            Color::DarkGray
        };
        let preview_title_style = if preview_focused {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        let preview_title = if preview_focused {
            " Preview (j/k scroll) "
        } else {
            " Preview "
        };

        let preview_block = Block::default()
            .borders(Borders::LEFT)
            .border_style(Style::default().fg(preview_border_color))
            .title(Span::styled(preview_title, preview_title_style));
        let preview_inner = preview_block.inner(chunks[1]);
        frame.render_widget(preview_block, chunks[1]);

        let lines = render_markdown(content);
        let paragraph = Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .scroll((app.detail_scroll as u16, 0));
        frame.render_widget(paragraph, preview_inner);
    } else {
        // If user was in Preview focus but panel doesn't have preview, snap back
        if app.focus == Focus::Preview {
            app.focus = Focus::Detail;
        }

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

fn render_status_bar(frame: &mut Frame, app: &App, area: Rect) {
    let panel = app.active_panel;

    // Left side: project + panel + filter
    let project_name = if app.selected_project == 0 {
        "Global".to_string()
    } else {
        app.projects
            .get(app.selected_project - 1)
            .map(|p| p.short_name.clone())
            .unwrap_or("?".to_string())
    };

    let mut left_spans = vec![
        Span::styled(
            format!(" {} ", project_name),
            Style::default().fg(Color::Black).bg(Color::Cyan),
        ),
        Span::styled(
            format!(" {} ", panel.label()),
            Style::default().fg(Color::Cyan),
        ),
    ];

    if !app.filter.is_empty() {
        left_spans.push(Span::styled(
            format!(" /{} ", app.filter),
            Style::default().fg(Color::Yellow),
        ));
    }

    // Right side: context-specific key hints
    let hints = match panel {
        Panel::Skills => "s=Search a=Create e=Edit d=Delete y=Copy",
        Panel::Agents => "a=Create e=Edit d=Delete y=Copy",
        Panel::Mcp => "s=Search a=Add t=Toggle d=Delete y=Copy",
        Panel::Settings => "a=Allow D=Deny d=Delete y=Copy",
        Panel::Plugins => "s=Search d=Remove y=Copy",
        Panel::Memory => "e=Edit d=Delete y=Copy",
        Panel::Config => "e=Edit y=Copy",
        Panel::Projects => "Enter=Select",
        Panel::Sessions => "x=Export y=Copy",
        Panel::Stats => "",
        Panel::Todos => "",
    };

    let right_span = Span::styled(format!("{} ", hints), Style::default().fg(Color::DarkGray));

    // Render: left-aligned spans + right-aligned hints
    let left_line = Line::from(left_spans);
    let right_line = Line::from(right_span);

    // Render left part with background
    frame.render_widget(
        Paragraph::new(left_line).style(Style::default().bg(Color::DarkGray)),
        area,
    );

    // Overlay right-aligned hints
    let hints_width = hints.len() as u16 + 1;
    if area.width > hints_width {
        let right_area = Rect {
            x: area.x + area.width - hints_width,
            y: area.y,
            width: hints_width,
            height: 1,
        };
        frame.render_widget(
            Paragraph::new(right_line).style(Style::default().bg(Color::DarkGray)),
            right_area,
        );
    }
}
