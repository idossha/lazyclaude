use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::app::App;
use lazyclaude::sources::stats::{self, StatsData, StatsPeriod};

use super::theme::THEME;

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
                Style::default().fg(THEME.text_secondary),
            )),
            Line::from(Span::styled(
                "  Use Claude Code to generate activity.",
                Style::default().fg(THEME.text_secondary),
            )),
        ]);
        frame.render_widget(msg, area);
        return;
    }

    let has_tokens_chart = !app.data.stats.daily_model_tokens.is_empty();
    let has_hourly = app.data.stats.hour_counts.iter().any(|&c| c > 0);
    let top_h = (app.data.stats.model_usage.len() as u16 + 2).max(7);
    let heatmap_h: u16 = 11;

    let chunks = Layout::vertical([
        Constraint::Length(1),         // period tabs
        Constraint::Length(top_h),     // summary (overview + models)
        Constraint::Length(heatmap_h), // heatmap
        Constraint::Fill(1),           // tokens chart or hourly fallback
    ])
    .split(area);

    render_period_tabs(frame, app.stats_period, chunks[0]);
    render_summary(frame, app, chunks[1]);
    render_heatmap(frame, app, chunks[2]);
    if has_tokens_chart {
        render_tokens_chart(frame, app, chunks[3]);
    } else if has_hourly {
        render_hourly(frame, &app.data.stats, chunks[3]);
    }
}

// ── Period tabs ─────────────────────────────────────────────────────

fn render_period_tabs(frame: &mut Frame, period: StatsPeriod, area: Rect) {
    let periods = [
        StatsPeriod::AllTime,
        StatsPeriod::Last7Days,
        StatsPeriod::Last30Days,
    ];
    let active = Style::default()
        .fg(THEME.text_accent)
        .add_modifier(Modifier::BOLD);
    let dim = Style::default().fg(THEME.text_secondary);

    let mut spans: Vec<Span> = vec![Span::styled("  ", Style::default())];
    for (i, &p) in periods.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled("  ", Style::default()));
        }
        let style = if p == period { active } else { dim };
        spans.push(Span::styled(p.label(), style));
    }

    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

// ── Summary: Overview (left) + Models (right) ───────────────────────

fn render_summary(frame: &mut Frame, app: &mut App, area: Rect) {
    let stats = &app.data.stats;
    let (start, end) = app.stats_period.date_range();
    let summary = stats::compute_summary(stats, start.as_deref(), end.as_deref());

    if stats.model_usage.is_empty() {
        render_overview(frame, stats, &summary, area);
        return;
    }
    let cols =
        Layout::horizontal([Constraint::Percentage(55), Constraint::Percentage(45)]).split(area);
    render_overview(frame, stats, &summary, cols[0]);
    render_models(frame, stats, &summary, cols[1]);
}

fn render_overview(
    frame: &mut Frame,
    stats: &StatsData,
    summary: &stats::StatsSummary,
    area: Rect,
) {
    let bold = Style::default()
        .fg(THEME.text_primary)
        .add_modifier(Modifier::BOLD);
    let dim = Style::default().fg(THEME.text_secondary);
    let yellow = Style::default().fg(THEME.text_emphasis);

    let mut lines = vec![
        Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled(format_number(stats.total_sessions), bold),
            Span::styled(" sessions  ", dim),
            Span::styled(
                format!("{}/{}", summary.active_days, summary.total_days),
                bold,
            ),
            Span::styled(" active days", dim),
        ]),
        Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled(format_number(stats.total_messages), bold),
            Span::styled(" msgs  ", dim),
            Span::styled("Streak: ", dim),
            Span::styled(format!("{}", summary.current_streak), yellow),
            Span::styled(" days", dim),
        ]),
        Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled(format_number(stats.total_tool_calls), bold),
            Span::styled(" tool calls  ", dim),
            Span::styled("Best: ", dim),
            Span::styled(format!("{}", summary.longest_streak), yellow),
            Span::styled(" days", dim),
        ]),
        Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled(format!("{:.0}%", stats.cache_hit_rate * 100.0), bold),
            Span::styled(" cache hit  ", dim),
            Span::styled(
                pretty_date(&summary.most_active_day),
                Style::default().fg(THEME.text_accent),
            ),
            Span::styled(
                format!(" ({})", format_number(summary.most_active_day_msgs)),
                dim,
            ),
        ]),
    ];

    if let Some(ref longest) = stats.longest_session {
        let dur = format_duration(longest.duration_ms);
        lines.push(Line::from(vec![
            Span::styled("  Longest: ", dim),
            Span::styled(format_number(longest.message_count), yellow),
            Span::styled(format!(" msgs ({dur})",), dim),
        ]));
    }

    let fav = shorten_model_name(&summary.favorite_model);
    lines.push(Line::from(vec![
        Span::styled("  Fav: ", dim),
        Span::styled(fav, Style::default().fg(THEME.text_accent)),
        Span::styled("  Total: ", dim),
        Span::styled(format_tokens(summary.total_tokens), bold),
        Span::styled(" tokens", dim),
    ]));

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(THEME.border_unfocused))
        .title(Span::styled(" Overview ", Style::default().fg(THEME.text_accent)));
    frame.render_widget(Paragraph::new(lines).block(block), area);
}

fn render_models(frame: &mut Frame, stats: &StatsData, _summary: &stats::StatsSummary, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(THEME.border_unfocused))
        .title(Span::styled(" Models ", Style::default().fg(THEME.text_accent)));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if stats.model_usage.is_empty() {
        return;
    }

    let grand_total: u64 = stats.model_usage.iter().map(|m| m.total_tokens).sum();
    let dim = Style::default().fg(THEME.text_secondary);

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
        let pct = if grand_total > 0 {
            model.total_tokens as f64 / grand_total as f64 * 100.0
        } else {
            0.0
        };
        let color = THEME.model_colors[i % THEME.model_colors.len()];

        let line = Line::from(vec![
            Span::styled(format!(" {:<14}", short), Style::default().fg(color)),
            Span::styled(format!("{:>5.1}%", pct), Style::default().fg(THEME.text_primary)),
            Span::styled(
                format!(
                    "  {} in / {} out",
                    format_tokens(
                        model.input_tokens + model.cache_read_tokens + model.cache_creation_tokens
                    ),
                    format_tokens(model.output_tokens)
                ),
                dim,
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
        .border_style(Style::default().fg(THEME.border_unfocused))
        .title(Span::styled(" Activity ", Style::default().fg(THEME.text_accent)));
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
    let label_style = Style::default().fg(THEME.chart_label);
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

            let level = heat_level(msgs, &qthresh);
            let fg = THEME.heat_colors[level];

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

    let dim = Style::default().fg(THEME.text_secondary);
    let white = Style::default().fg(THEME.text_primary);
    let mut spans: Vec<Span> = vec![Span::styled(" Less ", dim)];
    for &c in &THEME.heat_colors {
        spans.push(Span::styled("\u{2588}", Style::default().fg(c)));
    }
    spans.push(Span::styled(" More", dim));

    // Append selected-day info on the same line
    let selected = &app.stats_selected_date;
    if !selected.is_empty() {
        let pretty = pretty_date(selected);
        spans.push(Span::styled("   ", Style::default()));
        spans.push(Span::styled(pretty, Style::default().fg(THEME.text_accent)));

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

// ── Tokens per Day (line chart) ─────────────────────────────────────

fn render_tokens_chart(frame: &mut Frame, app: &mut App, area: Rect) {
    let stats = &app.data.stats;
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(THEME.border_unfocused))
        .title(Span::styled(
            " Tokens per Day ",
            Style::default().fg(THEME.text_accent),
        ));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.height < 4 || inner.width < 10 {
        return;
    }

    // Filter daily_model_tokens to the active period
    let (start, end) = app.stats_period.date_range();
    let filtered: Vec<&stats::DailyModelTokens> = stats
        .daily_model_tokens
        .iter()
        .filter(|d| {
            if let Some(ref s) = start {
                if d.date.as_str() < s.as_str() {
                    return false;
                }
            }
            if let Some(ref e) = end {
                if d.date.as_str() > e.as_str() {
                    return false;
                }
            }
            true
        })
        .collect();

    if filtered.is_empty() {
        let dim = Style::default().fg(THEME.text_secondary);
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled("  No token data", dim))),
            Rect {
                x: inner.x,
                y: inner.y,
                width: inner.width,
                height: 1,
            },
        );
        return;
    }

    // Compute per-day totals for each model
    let models = &stats.all_models;
    let n_days = filtered.len();

    // Per-day total (across all models) for Y-axis scaling
    let day_totals: Vec<u64> = filtered
        .iter()
        .map(|d| d.tokens_by_model.values().sum::<u64>())
        .collect();
    let max_total = day_totals.iter().copied().max().unwrap_or(1).max(1);

    // Reserve 1 row for date labels, 1 row for legend
    let chart_rows = inner.height.saturating_sub(2) as usize;
    if chart_rows == 0 {
        return;
    }

    // Y-axis label width (e.g. "691k ")
    let y_label_w: u16 = 6;
    let chart_x = inner.x + y_label_w;
    let chart_w = inner.width.saturating_sub(y_label_w) as usize;
    if chart_w < 2 {
        return;
    }

    // Column distribution: one column per day
    let base_col_w = chart_w / n_days;
    let extra_cols = chart_w % n_days;

    let buf = frame.buffer_mut();
    let dim_style = Style::default().fg(THEME.chart_label);

    // ── Y-axis labels (top, middle, bottom) ─────────────────────────
    let label_positions = [0usize, chart_rows / 2, chart_rows.saturating_sub(1)];
    for &row in &label_positions {
        let val = max_total as f64 * (1.0 - row as f64 / (chart_rows.max(1) - 1).max(1) as f64);
        let label = format_tokens(val as u64);
        let y = inner.y + row as u16;
        for (i, ch) in label.chars().enumerate() {
            if i as u16 + inner.x < chart_x {
                if let Some(cell) = buf.cell_mut((inner.x + i as u16, y)) {
                    cell.set_char(ch);
                    cell.set_style(dim_style);
                }
            }
        }
    }

    // ── Build per-model series (lowest usage first → dominant draws last) ─
    let baseline_y = inner.y + chart_rows as u16 - 1;

    let mut model_series: Vec<(usize, Vec<u64>)> = models
        .iter()
        .enumerate()
        .map(|(mi, model_name)| {
            let values: Vec<u64> = filtered
                .iter()
                .map(|d| d.tokens_by_model.get(model_name).copied().unwrap_or(0))
                .collect();
            let total: u64 = values.iter().sum();
            (mi, values, total)
        })
        .filter(|(_, _, total)| *total > 0)
        .map(|(mi, values, _total)| (mi, values))
        .collect::<Vec<_>>();
    model_series.sort_by_key(|(mi, values)| {
        let total: u64 = values.iter().sum();
        (total, *mi)
    });

    // Pre-compute x positions for each day column
    let mut col_positions: Vec<(u16, u16)> = Vec::with_capacity(n_days);
    {
        let mut cx = chart_x;
        for d in 0..n_days {
            let w = (if d < extra_cols {
                base_col_w + 1
            } else {
                base_col_w
            }) as u16;
            col_positions.push((cx, w));
            cx += w;
        }
    }

    // ── Draw each model's line ──────────────────────────────────────
    for &(mi, ref values) in &model_series {
        let color = THEME.model_colors[mi % THEME.model_colors.len()];
        let style = Style::default().fg(color);

        // Map every day to a Y position — zero values go to baseline
        let y_positions: Vec<u16> = values
            .iter()
            .map(|&v| {
                if v == 0 || chart_rows <= 1 {
                    baseline_y
                } else {
                    let frac = v as f64 / max_total as f64;
                    let row = ((1.0 - frac) * (chart_rows as f64 - 1.0)).round() as u16;
                    inner.y + row
                }
            })
            .collect();

        // Draw connecting lines between ALL consecutive days
        for d in 0..n_days.saturating_sub(1) {
            let y = y_positions[d];
            let y_next = y_positions[d + 1];
            let (cx, cw) = col_positions[d];
            let (cx_next, cw_next) = col_positions[d + 1];
            let mid_x = cx + cw / 2;
            let mid_x_next = cx_next + cw_next / 2;

            if y == y_next {
                // Same row — horizontal line
                for x in (mid_x + 1)..mid_x_next {
                    if let Some(cell) = buf.cell_mut((x, y)) {
                        cell.set_char('\u{2500}'); // ─
                        cell.set_style(style);
                    }
                }
            } else {
                // Different rows — stepped connection with smooth corners
                let step_x = (mid_x + mid_x_next) / 2;
                // Horizontal from current toward step
                for x in (mid_x + 1)..step_x {
                    if let Some(cell) = buf.cell_mut((x, y)) {
                        cell.set_char('\u{2500}'); // ─
                        cell.set_style(style);
                    }
                }
                // Corners + vertical
                let (y_min, y_max) = if y < y_next { (y, y_next) } else { (y_next, y) };
                for vy in (y_min + 1)..y_max {
                    if let Some(cell) = buf.cell_mut((step_x, vy)) {
                        cell.set_char('\u{2502}'); // │
                        cell.set_style(style);
                    }
                }
                if y < y_next {
                    // Going down: ╮ at top, ╰ at bottom
                    if let Some(cell) = buf.cell_mut((step_x, y)) {
                        cell.set_char('\u{256E}'); // ╮
                        cell.set_style(style);
                    }
                    if let Some(cell) = buf.cell_mut((step_x, y_next)) {
                        cell.set_char('\u{2570}'); // ╰
                        cell.set_style(style);
                    }
                } else {
                    // Going up: ╯ at bottom, ╭ at top
                    if let Some(cell) = buf.cell_mut((step_x, y)) {
                        cell.set_char('\u{256F}'); // ╯
                        cell.set_style(style);
                    }
                    if let Some(cell) = buf.cell_mut((step_x, y_next)) {
                        cell.set_char('\u{256D}'); // ╭
                        cell.set_style(style);
                    }
                }
                // Horizontal from step toward next
                for x in (step_x + 1)..mid_x_next {
                    if let Some(cell) = buf.cell_mut((x, y_next)) {
                        cell.set_char('\u{2500}'); // ─
                        cell.set_style(style);
                    }
                }
            }
        }

        // Draw bold markers only for non-zero days (on top of lines)
        for d in 0..n_days {
            if values[d] == 0 {
                continue;
            }
            let y = y_positions[d];
            let (cx, cw) = col_positions[d];
            let mid_x = cx + cw / 2;
            if let Some(cell) = buf.cell_mut((mid_x, y)) {
                cell.set_char('\u{25CF}'); // ● filled circle
                cell.set_style(style);
            }
        }
    }

    // ── Date labels along bottom ────────────────────────────────────
    let label_y = inner.y + chart_rows as u16;
    if label_y < inner.y + inner.height {
        // Show ~5 evenly spaced date labels
        let n_labels = 5.min(n_days);
        if n_labels > 0 {
            let step = if n_labels > 1 {
                (n_days - 1) / (n_labels - 1)
            } else {
                1
            };
            for li in 0..n_labels {
                let d = (li * step).min(n_days - 1);
                let date_str = &filtered[d].date;
                let short = if let Some((_, m, day)) = stats::parse_date(date_str) {
                    format!("{} {day}", MONTH_ABBR[m as usize - 1])
                } else {
                    continue;
                };
                let (cx, _) = col_positions[d];
                for (i, ch) in short.chars().enumerate() {
                    let x = cx + i as u16;
                    if x < inner.x + inner.width {
                        if let Some(cell) = buf.cell_mut((x, label_y)) {
                            cell.set_char(ch);
                            cell.set_style(dim_style);
                        }
                    }
                }
            }
        }
    }

    // ── Legend ───────────────────────────────────────────────────────
    let legend_y = inner.y + chart_rows as u16 + 1;
    if legend_y < inner.y + inner.height {
        let mut spans: Vec<Span> = vec![Span::styled("  ", Style::default())];
        for &(mi, _) in model_series.iter().rev() {
            let color = THEME.model_colors[mi % THEME.model_colors.len()];
            let name = shorten_model_name(&models[mi]);
            spans.push(Span::styled("\u{25CF} ", Style::default().fg(color)));
            spans.push(Span::styled(format!("{name}  "), dim_style));
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
}

// ── Activity by Hour (fallback when no dailyModelTokens) ────────────

const EIGHTH_BLOCKS: [char; 9] = [
    ' ', '\u{2581}', '\u{2582}', '\u{2583}', '\u{2584}', '\u{2585}', '\u{2586}', '\u{2587}',
    '\u{2588}',
];

fn render_hourly(frame: &mut Frame, stats: &StatsData, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(THEME.border_unfocused))
        .title(Span::styled(
            " Activity by Hour ",
            Style::default().fg(THEME.text_accent),
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

    let bar_style = Style::default().fg(THEME.chart_bar);
    let label_style = Style::default().fg(THEME.chart_label);

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
    let days = hours / 24;
    if days > 0 {
        format!("{}d {}h {}m", days, hours % 24, mins % 60)
    } else if hours > 0 {
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
