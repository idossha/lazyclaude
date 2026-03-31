use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::app::App;
use lazyclaude::sources::stats::{self, StatsData};

// ── Heat-map color palette (5 levels, GitHub-style) ─────────────────

const HEAT_COLORS: [Color; 5] = [
    Color::Indexed(236), // 0: no activity — dark gray
    Color::Indexed(22),  // 1: low — dark green
    Color::Indexed(28),  // 2: medium — green
    Color::Indexed(34),  // 3: high — bright green
    Color::Indexed(40),  // 4: very high — vivid green
];

/// Compute quartile thresholds from non-zero daily message counts.
fn quartile_thresholds(stats: &StatsData) -> [u64; 4] {
    let mut counts: Vec<u64> = stats
        .daily_activity
        .iter()
        .map(|d| d.messages)
        .filter(|&m| m > 0)
        .collect();
    counts.sort_unstable();
    if counts.is_empty() {
        return [1, 2, 3, 4];
    }
    let n = counts.len();
    [
        counts[n / 4],
        counts[n / 2],
        counts[3 * n / 4],
        *counts.last().unwrap(),
    ]
}

fn heat_level(messages: u64, q: &[u64; 4]) -> usize {
    if messages == 0 {
        return 0;
    }
    if messages <= q[0] {
        return 1;
    }
    if messages <= q[1] {
        return 2;
    }
    if messages <= q[2] {
        return 3;
    }
    4
}

const MONTH_ABBR: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];
const DAY_LABELS: [&str; 7] = ["Su", "Mo", "Tu", "We", "Th", "Fr", "Sa"];

// ── Main entry point ────────────────────────────────────────────────

pub(crate) fn render_stats_dashboard(frame: &mut Frame, app: &mut App, area: Rect) {
    if app.data.stats.total_sessions == 0
        && app.data.stats.total_messages == 0
        && app.data.stats.daily_activity.is_empty()
    {
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

    let has_hourly = app.data.stats.hour_counts.iter().any(|&c| c > 0);
    let top_h = (app.data.stats.model_usage.len() as u16 + 2).max(5);
    // Heatmap: 2 borders + 1 month labels + 7 grid rows + 1 legend = 11
    let heatmap_h: u16 = 11;

    let chunks = Layout::vertical([
        Constraint::Length(top_h),
        Constraint::Length(heatmap_h),
        Constraint::Fill(1),
    ])
    .split(area);

    render_top_row(frame, &app.data.stats, chunks[0]);
    render_heatmap(frame, app, chunks[1]);
    if has_hourly {
        render_hourly(frame, &app.data.stats, chunks[2]);
    }
}

// ── Top row: Overview (left) + Tokens by Model (right) ──────────────

fn render_top_row(frame: &mut Frame, stats: &StatsData, area: Rect) {
    if stats.model_usage.is_empty() {
        render_overview(frame, stats, area);
        return;
    }
    let cols =
        Layout::horizontal([Constraint::Percentage(55), Constraint::Percentage(45)]).split(area);
    render_overview(frame, stats, cols[0]);
    render_models(frame, stats, cols[1]);
}

fn render_overview(frame: &mut Frame, stats: &StatsData, area: Rect) {
    let bold = Style::default()
        .fg(Color::White)
        .add_modifier(Modifier::BOLD);
    let dim = Style::default().fg(Color::DarkGray);

    let mut lines = vec![
        Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled(format_number(stats.total_sessions), bold),
            Span::styled(" sessions  ", dim),
            Span::styled(format_number(stats.total_messages), bold),
            Span::styled(" msgs", dim),
        ]),
        Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled(format_number(stats.total_tool_calls), bold),
            Span::styled(" tool calls  ", dim),
            Span::styled(format!("{:.0}%", stats.cache_hit_rate * 100.0), bold),
            Span::styled(" cache hit", dim),
        ]),
    ];

    if let Some(ref longest) = stats.longest_session {
        let dur = format_duration(longest.duration_ms);
        lines.push(Line::from(vec![
            Span::styled("  Longest: ", dim),
            Span::styled(
                format_number(longest.message_count),
                Style::default().fg(Color::Yellow),
            ),
            Span::styled(format!(" msgs ({dur})"), dim),
        ]));
    }

    let mut date_spans = vec![
        Span::styled("  Since ", dim),
        Span::styled(
            stats.first_session_date.clone(),
            Style::default().fg(Color::White),
        ),
    ];
    if !stats.last_computed_date.is_empty() {
        date_spans.push(Span::styled(" \u{00b7} ", dim));
        date_spans.push(Span::styled(
            stats.last_computed_date.clone(),
            Style::default().fg(Color::DarkGray),
        ));
    }
    lines.push(Line::from(date_spans));

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(Span::styled(" Overview ", Style::default().fg(Color::Cyan)));
    frame.render_widget(Paragraph::new(lines).block(block), area);
}

fn render_models(frame: &mut Frame, stats: &StatsData, area: Rect) {
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

    let name_w = 14.min(inner.width as usize / 3);
    let token_w = 7;
    let bar_max = (inner.width as usize).saturating_sub(name_w + token_w + 4);

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
                format!(" {:<width$}", short, width = name_w),
                Style::default().fg(color),
            ),
            Span::styled(bar, Style::default().fg(color)),
            Span::styled(
                format!(" {token_str}"),
                Style::default().fg(Color::DarkGray),
            ),
        ]);
        frame.render_widget(Paragraph::new(line), row);
    }
}

// ── Activity heatmap (GitHub-style, past 365 days) ──────────────────

fn render_heatmap(frame: &mut Frame, app: &mut App, area: Rect) {
    let stats = &app.data.stats;
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(Span::styled(
            " Activity ",
            Style::default().fg(Color::Cyan),
        ));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.height < 9 || inner.width < 20 {
        return;
    }

    let (all_dates, today_str, total_weeks) = stats::year_heatmap_dates();
    if all_dates.is_empty() {
        return;
    }

    let qthresh = quartile_thresholds(stats);
    let label_col_w: u16 = 3; // "Mo " etc.

    // Distribute full width across columns — some get an extra pixel
    let grid_space = inner.width.saturating_sub(label_col_w) as usize;
    let n_cols = total_weeks.min(grid_space); // at least 1 char per col
    if n_cols == 0 {
        return;
    }
    let base_w = grid_space / n_cols;
    let extra_cols = grid_space % n_cols; // first `extra_cols` columns get base_w + 1

    // Show the rightmost (most recent) n_cols weeks
    let skip_weeks = total_weeks.saturating_sub(n_cols);

    // Layout rows inside the block inner area:
    //  row 0:   month labels
    //  row 1-7: grid (Sun–Sat)
    //  row 8:   legend + selected-day info
    let month_y = inner.y;
    let grid_y = inner.y + 1;
    let legend_y = inner.y + 8;
    let grid_x = inner.x + label_col_w;

    // Pre-compute x-offset for each column
    let mut col_x: Vec<u16> = Vec::with_capacity(n_cols);
    let mut col_w: Vec<u16> = Vec::with_capacity(n_cols);
    let mut cx = grid_x;
    for c in 0..n_cols {
        let w = (if c < extra_cols { base_w + 1 } else { base_w }) as u16;
        col_x.push(cx);
        col_w.push(w);
        cx += w;
    }

    // ── Month labels ────────────────────────────────────────────────
    let buf = frame.buffer_mut();
    let label_style = Style::default().fg(Color::DarkGray);
    let mut label_end_x = grid_x; // tracks rightmost x written to avoid overlap
    let mut prev_month = 0u32;

    for (col, &x) in col_x.iter().enumerate() {
        let date_idx = (skip_weeks + col) * 7; // Sunday of this week
        let date = match all_dates.get(date_idx) {
            Some(d) => d.as_str(),
            None => continue,
        };
        if let Some((_, m, _)) = stats::parse_date(date) {
            if m != prev_month {
                prev_month = m;
                if x >= label_end_x {
                    let name = MONTH_ABBR[m as usize - 1];
                    for (i, ch) in name.chars().enumerate() {
                        if let Some(cell) = buf.cell_mut((x + i as u16, month_y)) {
                            cell.set_char(ch);
                            cell.set_style(label_style);
                        }
                    }
                    label_end_x = x + name.len() as u16 + 1;
                }
            }
        }
    }

    // ── Day-of-week labels (show Mon, Wed, Fri only — rows 1, 3, 5) ─
    for row in [1usize, 3, 5] {
        let y = grid_y + row as u16;
        if y >= inner.y + inner.height {
            break;
        }
        let lbl = DAY_LABELS[row];
        for (i, ch) in lbl.chars().enumerate() {
            if let Some(cell) = buf.cell_mut((inner.x + i as u16, y)) {
                cell.set_char(ch);
                cell.set_style(label_style);
            }
        }
    }

    // ── Grid cells ──────────────────────────────────────────────────
    // Store row-major for mouse hit-testing: heatmap_grid[row * n_cols + col]
    let mut heatmap_grid: Vec<String> = vec![String::new(); 7 * n_cols];

    for row in 0..7u16 {
        let y = grid_y + row;
        if y >= inner.y + inner.height {
            break;
        }
        for col in 0..n_cols {
            let date_idx = (skip_weeks + col) * 7 + row as usize;
            let date = match all_dates.get(date_idx) {
                Some(d) => d.as_str(),
                None => continue,
            };

            // Don't render future cells
            if date > today_str.as_str() {
                continue;
            }

            heatmap_grid[row as usize * n_cols + col] = date.to_string();

            let msgs = stats
                .daily_lookup
                .get(date)
                .map(|&i| stats.daily_activity[i].messages)
                .unwrap_or(0);

            let is_selected = date == app.stats_selected_date;
            let level = heat_level(msgs, &qthresh);
            let fg = if is_selected {
                Color::White
            } else {
                HEAT_COLORS[level]
            };

            let x = col_x[col];
            let w = col_w[col];
            for dx in 0..w {
                if let Some(cell) = buf.cell_mut((x + dx, y)) {
                    cell.set_char('\u{2588}');
                    cell.set_style(Style::default().fg(fg));
                }
            }
        }
    }

    // Store geometry for mouse handler
    app.stats_heatmap_grid = heatmap_grid;
    app.stats_heatmap_origin = (grid_x, grid_y);
    app.stats_heatmap_cols = n_cols as u16;
    app.stats_heatmap_base_w = base_w as u16;
    app.stats_heatmap_extra = extra_cols as u16;

    // ── Legend + selected day info ───────────────────────────────────
    if legend_y >= inner.y + inner.height {
        return;
    }

    let dim = Style::default().fg(Color::DarkGray);
    let white = Style::default().fg(Color::White);
    let mut spans: Vec<Span> = vec![Span::styled(" Less ", dim)];
    for &c in &HEAT_COLORS {
        spans.push(Span::styled("\u{2588}", Style::default().fg(c)));
    }
    spans.push(Span::styled(" More", dim));

    // Append selected-day info on the same line
    let selected = &app.stats_selected_date;
    if !selected.is_empty() {
        let pretty = pretty_date(selected);
        spans.push(Span::styled("   ", Style::default()));
        spans.push(Span::styled(pretty, Style::default().fg(Color::Cyan)));

        if let Some(&idx) = app.data.stats.daily_lookup.get(selected) {
            let day = &app.data.stats.daily_activity[idx];
            spans.push(Span::styled(
                format!(
                    "  {} msgs  {} sess  {} tc",
                    format_number(day.messages),
                    format_number(day.sessions),
                    format_number(day.tool_calls),
                ),
                white,
            ));
        } else {
            spans.push(Span::styled("  No activity", dim));
        }
    }

    frame.render_widget(
        Paragraph::new(Line::from(spans)),
        Rect {
            x: inner.x,
            y: legend_y,
            width: inner.width,
            height: 1,
        },
    );
}

// ── Activity by Hour ────────────────────────────────────────────────

const EIGHTH_BLOCKS: [char; 9] = [
    ' ',
    '\u{2581}',
    '\u{2582}',
    '\u{2583}',
    '\u{2584}',
    '\u{2585}',
    '\u{2586}',
    '\u{2587}',
    '\u{2588}',
];

fn render_hourly(frame: &mut Frame, stats: &StatsData, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(Span::styled(
            " Activity by Hour ",
            Style::default().fg(Color::Cyan),
        ));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.height < 3 || inner.width < 24 {
        return;
    }

    let max_val = stats.hour_counts.iter().copied().max().unwrap_or(1).max(1);
    let bar_rows = inner.height.saturating_sub(1);
    let label_y = inner.y + bar_rows;

    let base_w = inner.width as usize / 24;
    let extra = inner.width as usize % 24;

    let bar_style = Style::default().fg(Color::Cyan);
    let label_style = Style::default().fg(Color::DarkGray);

    let buf = frame.buffer_mut();
    let mut x = inner.x;

    for h in 0..24usize {
        let w = (if h < extra { base_w + 1 } else { base_w }) as u16;
        if w == 0 {
            continue;
        }

        let val = stats.hour_counts[h];
        let scaled = if max_val > 0 {
            (val as f64 / max_val as f64 * bar_rows as f64 * 8.0).round() as u16
        } else {
            0
        };
        let full = scaled / 8;
        let frac = (scaled % 8) as usize;

        for row in 0..full.min(bar_rows) {
            let y = inner.y + bar_rows - 1 - row;
            for dx in 0..w {
                if let Some(cell) = buf.cell_mut((x + dx, y)) {
                    cell.set_char(EIGHTH_BLOCKS[8]);
                    cell.set_style(bar_style);
                }
            }
        }

        if frac > 0 && full < bar_rows {
            let y = inner.y + bar_rows - 1 - full;
            for dx in 0..w {
                if let Some(cell) = buf.cell_mut((x + dx, y)) {
                    cell.set_char(EIGHTH_BLOCKS[frac]);
                    cell.set_style(bar_style);
                }
            }
        }

        let label = format!("{h:>2}");
        if w >= 2 {
            let lx = x + (w - 2) / 2;
            for (i, ch) in label.chars().enumerate() {
                if let Some(cell) = buf.cell_mut((lx + i as u16, label_y)) {
                    cell.set_char(ch);
                    cell.set_style(label_style);
                }
            }
        }

        x += w;
    }
}

// ── Formatting helpers ──────────────────────────────────────────────

fn pretty_date(date: &str) -> String {
    if let Some((y, m, d)) = stats::parse_date(date) {
        let month = MONTH_ABBR.get(m as usize - 1).unwrap_or(&"???");
        format!("{month} {d}, {y}")
    } else {
        date.to_string()
    }
}

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
    if let Some(pos) = stripped.rfind('-') {
        let suffix = &stripped[pos + 1..];
        if suffix.len() >= 8 && suffix.chars().all(|c| c.is_ascii_digit()) {
            return stripped[..pos].to_string();
        }
    }
    stripped.to_string()
}
