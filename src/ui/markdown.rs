use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

pub(crate) fn render_markdown(content: &str) -> Vec<Line<'static>> {
    let mut lines = Vec::new();

    for raw_line in content.lines() {
        let trimmed = raw_line.trim_start();

        if let Some(rest) = trimmed.strip_prefix("# ") {
            lines.push(Line::from(Span::styled(
                format!(" {}", rest),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )));
        } else if let Some(rest) = trimmed.strip_prefix("## ") {
            lines.push(Line::from(Span::styled(
                format!(" {}", rest),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )));
        } else if let Some(rest) = trimmed.strip_prefix("### ") {
            lines.push(Line::from(Span::styled(
                format!(" {}", rest),
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
                Span::styled(trimmed[2..].to_string(), Style::default().fg(Color::White)),
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
