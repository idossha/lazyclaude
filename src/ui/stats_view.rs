use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Bar, BarChart, BarGroup, Block, Borders, Paragraph, Sparkline},
    Frame,
};

pub(crate) fn render_stats_dashboard(
    frame: &mut Frame,
    stats: &lazyclaude::sources::stats::StatsData,
    area: Rect,
) {
    if stats.total_sessions == 0 && stats.total_messages == 0 && stats.daily_activity.is_empty() {
        let msg = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                "  No stats data available.",
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(Span::styled(
                "  Use Claude Code to generate activity.",
                Style::default().fg(Color::DarkGray),
            )),
        ]);
        frame.render_widget(msg, area);
        return;
    }

    let model_rows = (stats.model_usage.len().max(1) + 2) as u16;
    let chunks = Layout::vertical([
        Constraint::Length(4),
        Constraint::Length(5),
        Constraint::Length(model_rows),
        Constraint::Min(6),
    ])
    .split(area);

    render_stats_overview(frame, stats, chunks[0]);
    render_stats_sparkline(frame, stats, chunks[1]);
    render_stats_models(frame, stats, chunks[2]);
    render_stats_hourly(frame, stats, chunks[3]);
}

fn render_stats_overview(
    frame: &mut Frame,
    stats: &lazyclaude::sources::stats::StatsData,
    area: Rect,
) {
    let bold_white = Style::default()
        .fg(Color::White)
        .add_modifier(Modifier::BOLD);
    let dim = Style::default().fg(Color::DarkGray);

    let mut lines = vec![Line::from(vec![
        Span::styled("  ", Style::default()),
        Span::styled(format_number(stats.total_sessions), bold_white),
        Span::styled(" sessions   ", dim),
        Span::styled(format_number(stats.total_messages), bold_white),
        Span::styled(" messages   ", dim),
        Span::styled(format!("since {}", stats.first_session_date), dim),
    ])];

    if let Some(ref longest) = stats.longest_session {
        let dur = format_duration(longest.duration_ms);
        lines.push(Line::from(vec![
            Span::styled("  Longest: ", dim),
            Span::styled(
                format!("{} messages", format_number(longest.message_count)),
                Style::default().fg(Color::Yellow),
            ),
            Span::styled(format!(" ({dur})"), dim),
        ]));
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(Span::styled(" Overview ", Style::default().fg(Color::Cyan)));
    frame.render_widget(Paragraph::new(lines).block(block), area);
}

fn render_stats_sparkline(
    frame: &mut Frame,
    stats: &lazyclaude::sources::stats::StatsData,
    area: Rect,
) {
    let data: Vec<u64> = stats.daily_activity.iter().map(|d| d.messages).collect();
    if data.is_empty() {
        return;
    }

    let total: u64 = data.iter().sum();
    let avg = total / data.len().max(1) as u64;

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(Span::styled(
            format!(" Messages / Day  avg {} ", format_number(avg)),
            Style::default().fg(Color::Cyan),
        ));

    let sparkline = Sparkline::default()
        .block(block)
        .data(&data)
        .style(Style::default().fg(Color::Green));
    frame.render_widget(sparkline, area);
}

fn render_stats_models(
    frame: &mut Frame,
    stats: &lazyclaude::sources::stats::StatsData,
    area: Rect,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(Span::styled(
            " Tokens by Model ",
            Style::default().fg(Color::Cyan),
        ));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if stats.model_usage.is_empty() {
        return;
    }

    let max_tokens = stats
        .model_usage
        .iter()
        .map(|m| m.total_tokens)
        .max()
        .unwrap_or(1);
    let colors = [
        Color::Cyan,
        Color::Green,
        Color::Yellow,
        Color::Magenta,
        Color::Blue,
        Color::Red,
    ];
    let name_width: usize = 16;
    let token_width: usize = 10;
    let bar_max = (inner.width as usize).saturating_sub(name_width + token_width + 6);

    for (i, model) in stats.model_usage.iter().enumerate() {
        if i as u16 >= inner.height {
            break;
        }
        let row = Rect {
            x: inner.x,
            y: inner.y + i as u16,
            width: inner.width,
            height: 1,
        };

        let short = shorten_model_name(&model.model);
        let bar_len = if max_tokens > 0 {
            ((model.total_tokens as f64 / max_tokens as f64) * bar_max as f64) as usize
        } else {
            0
        }
        .max(1);
        let bar = "\u{2588}".repeat(bar_len);
        let token_str = format_tokens(model.total_tokens);
        let color = colors[i % colors.len()];

        let line = Line::from(vec![
            Span::styled(
                format!("  {:<width$}", short, width = name_width),
                Style::default().fg(color),
            ),
            Span::styled(bar, Style::default().fg(color)),
            Span::styled(
                format!("  {token_str}"),
                Style::default().fg(Color::DarkGray),
            ),
        ]);
        frame.render_widget(Paragraph::new(line), row);
    }
}

fn render_stats_hourly(
    frame: &mut Frame,
    stats: &lazyclaude::sources::stats::StatsData,
    area: Rect,
) {
    let bars: Vec<Bar> = (0..24)
        .map(|h: usize| {
            let label = if h.is_multiple_of(6) {
                format!("{h:>2}")
            } else {
                String::new()
            };
            Bar::default()
                .value(stats.hour_counts[h])
                .label(Line::from(label))
                .style(Style::default().fg(Color::Cyan))
        })
        .collect();

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(Span::styled(
            " Activity by Hour ",
            Style::default().fg(Color::Cyan),
        ));

    let chart = BarChart::default()
        .block(block)
        .bar_width(2)
        .bar_gap(0)
        .bar_style(Style::default().fg(Color::Cyan))
        .data(BarGroup::default().bars(&bars));
    frame.render_widget(chart, area);
}

// -- Stats helpers --------------------------------------------------------

fn format_number(n: u64) -> String {
    let s = n.to_string();
    let mut result = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    result.chars().rev().collect()
}

fn format_tokens(n: u64) -> String {
    if n >= 1_000_000_000 {
        format!("{:.1}B", n as f64 / 1_000_000_000.0)
    } else if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}K", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}

fn format_duration(ms: u64) -> String {
    let secs = ms / 1000;
    let mins = secs / 60;
    let hours = mins / 60;
    if hours > 0 {
        format!("{}h {}m", hours, mins % 60)
    } else {
        format!("{}m", mins)
    }
}

fn shorten_model_name(name: &str) -> String {
    let stripped = name.strip_prefix("claude-").unwrap_or(name);
    // Remove trailing date suffix (8+ digit number)
    if let Some(pos) = stripped.rfind('-') {
        let suffix = &stripped[pos + 1..];
        if suffix.len() >= 8 && suffix.chars().all(|c| c.is_ascii_digit()) {
            return stripped[..pos].to_string();
        }
    }
    stripped.to_string()
}
