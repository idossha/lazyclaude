use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub(crate) fn render_help(frame: &mut Frame, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(" Help ");

    let help_text = vec![
        Line::from(""),
        help_line("1-0", "Switch panel"),
        help_line("j/k", "Navigate items (scroll in preview)"),
        help_line("J/K", "Scroll preview (fast)"),
        help_line("Enter", "Select / enter preview"),
        help_line("l", "Focus right: panels > detail > preview"),
        help_line("h/BS", "Focus left: preview > detail > panels"),
        help_line("Tab", "Cycle focus"),
        help_line("/", "Filter items"),
        help_line("?", "Toggle help"),
        help_line("R", "Refresh data"),
        help_line("q", "Quit"),
        Line::from(""),
        Line::from(Span::styled(
            "  Panel actions:",
            Style::default().fg(Color::DarkGray),
        )),
        help_line("e", "Edit in $EDITOR"),
        help_line("a", "Add item"),
        help_line("d", "Delete / uninstall / unblock"),
        help_line("u", "Undo last delete"),
        help_line("D", "Add deny permission"),
        help_line("t", "Toggle enable/disable (MCP)"),
        help_line("s", "Search registry (Skills/MCP/Plugins)"),
        help_line("x", "Export panel to clipboard (JSON)"),
        help_line("y", "Copy to clipboard"),
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
