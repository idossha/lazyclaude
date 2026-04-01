use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};

use super::theme::THEME;

pub(crate) fn render_markdown(content: &str) -> Vec<Line<'static>> {
    let mut lines = Vec::new();

    for raw_line in content.lines() {
        let trimmed = raw_line.trim_start();

        if let Some(rest) = trimmed.strip_prefix("# ") {
            lines.push(Line::from(Span::styled(
                format!(" {}", rest),
                Style::default()
                    .fg(THEME.text_emphasis)
                    .add_modifier(Modifier::BOLD),
            )));
        } else if let Some(rest) = trimmed.strip_prefix("## ") {
            lines.push(Line::from(Span::styled(
                format!(" {}", rest),
                Style::default()
                    .fg(THEME.text_emphasis)
                    .add_modifier(Modifier::BOLD),
            )));
        } else if let Some(rest) = trimmed.strip_prefix("### ") {
            lines.push(Line::from(Span::styled(
                format!(" {}", rest),
                Style::default().fg(THEME.text_emphasis),
            )));
        } else if trimmed.starts_with("---") && trimmed.chars().all(|c| c == '-') {
            lines.push(Line::from(Span::styled(
                " ---",
                Style::default().fg(THEME.text_secondary),
            )));
        } else if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
            lines.push(Line::from(vec![
                Span::styled(" ", Style::default()),
                Span::styled("  ", Style::default().fg(THEME.text_success)),
                Span::styled(trimmed[2..].to_string(), Style::default().fg(THEME.text_primary)),
            ]));
        } else if trimmed.starts_with("```") {
            lines.push(Line::from(Span::styled(
                format!(" {trimmed}"),
                Style::default().fg(THEME.text_secondary),
            )));
        } else if trimmed.contains(':') && !trimmed.starts_with(' ') && lines.len() < 20 {
            if let Some((key, val)) = trimmed.split_once(':') {
                lines.push(Line::from(vec![
                    Span::styled(format!(" {}", key.trim()), Style::default().fg(THEME.text_accent)),
                    Span::styled(": ", Style::default().fg(THEME.text_secondary)),
                    Span::styled(val.trim().to_string(), Style::default().fg(THEME.text_primary)),
                ]));
            } else {
                lines.push(Line::from(Span::styled(
                    format!(" {raw_line}"),
                    Style::default().fg(THEME.text_primary),
                )));
            }
        } else if trimmed.is_empty() {
            lines.push(Line::from(""));
        } else {
            lines.push(Line::from(Span::styled(
                format!(" {raw_line}"),
                Style::default().fg(THEME.text_primary),
            )));
        }
    }

    if lines.is_empty() {
        lines.push(Line::from(Span::styled(
            " (empty)",
            Style::default().fg(THEME.text_secondary),
        )));
    }

    lines
}
