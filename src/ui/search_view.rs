use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

use crate::app::SearchSource;

use super::markdown::render_markdown;
use super::theme::THEME;

pub(crate) fn render_search_overlay(
    frame: &mut Frame,
    overlay: &crate::app::SearchOverlay,
    area: Rect,
) {
    let source_label = match overlay.source {
        SearchSource::Skills => "Skills Registry (anthropics/skills)",
        SearchSource::Mcp => "MCP Registry (npm)",
        SearchSource::Plugins => "Plugin Marketplace",
    };

    // Split: filter bar (3 rows with border) + content area
    let chunks = Layout::vertical([Constraint::Length(3), Constraint::Min(1)]).split(area);

    // Filter bar
    let filter_display = if overlay.filter.is_empty() {
        "Type to filter... (Up/Down=navigate, Enter=install, Tab=preview, Esc=close)".to_string()
    } else {
        overlay.filter.clone()
    };
    let filter_style = if overlay.filter.is_empty() {
        Style::default().fg(THEME.text_secondary)
    } else {
        Style::default().fg(THEME.text_primary)
    };
    let filter_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(THEME.border_focused))
        .title(format!(" Search {} ", source_label));
    let filter_paragraph = Paragraph::new(Line::from(vec![
        Span::styled("  > ", Style::default().fg(THEME.text_accent)),
        Span::styled(filter_display, filter_style),
    ]))
    .block(filter_block);
    frame.render_widget(filter_paragraph, chunks[0]);

    // Items + Preview split
    let content_chunks =
        Layout::horizontal([Constraint::Percentage(40), Constraint::Percentage(60)])
            .split(chunks[1]);

    // Build filtered item list
    let filtered = overlay.filtered_indices();
    let list_items: Vec<ListItem> = filtered
        .iter()
        .map(|&i| {
            let item = &overlay.all_items[i];
            let installed_marker = if item.installed { " \u{2714}" } else { "" };
            let name_color = if item.installed {
                THEME.text_secondary
            } else {
                THEME.text_success
            };
            let mut spans = vec![
                Span::styled("  ", Style::default()),
                Span::styled(item.name.clone(), Style::default().fg(name_color)),
            ];
            if !item.extra.is_empty() {
                spans.push(Span::styled(
                    format!("  {}", item.extra),
                    Style::default().fg(THEME.text_accent),
                ));
            }
            if !installed_marker.is_empty() {
                spans.push(Span::styled(
                    installed_marker.to_string(),
                    Style::default().fg(THEME.text_emphasis),
                ));
            }
            ListItem::new(Line::from(spans))
        })
        .collect();

    let mut list_state = ListState::default();
    if !filtered.is_empty() {
        list_state.select(Some(overlay.selected.min(filtered.len().saturating_sub(1))));
    }

    let list_border = if !overlay.preview_focused {
        THEME.border_focused
    } else {
        THEME.border_unfocused
    };
    let list = List::new(list_items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(list_border))
                .title(format!(" {} items ", filtered.len())),
        )
        .highlight_style(THEME.highlight_style())
        .highlight_symbol(" > ");
    frame.render_stateful_widget(list, content_chunks[0], &mut list_state);

    // Preview pane — use same clamped index as the list selection
    let clamped = overlay.selected.min(filtered.len().saturating_sub(1));
    let preview_content = filtered
        .get(clamped)
        .and_then(|&i| overlay.all_items.get(i))
        .map(|item| item.preview.as_str())
        .unwrap_or("");

    let preview_border = if overlay.preview_focused {
        THEME.border_focused
    } else {
        THEME.border_unfocused
    };
    let preview_title = if overlay.preview_focused {
        " Preview (j/k scroll) "
    } else {
        " Preview "
    };
    let preview_block = Block::default()
        .borders(Borders::LEFT)
        .border_style(Style::default().fg(preview_border))
        .title(Span::styled(
            preview_title,
            Style::default().fg(preview_border),
        ));
    let preview_inner = preview_block.inner(content_chunks[1]);
    frame.render_widget(preview_block, content_chunks[1]);

    let lines = render_markdown(preview_content);
    let paragraph = Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .scroll((overlay.preview_scroll as u16, 0));
    frame.render_widget(paragraph, preview_inner);
}
