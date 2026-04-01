use std::collections::HashMap;

use crate::config::Paths;

#[derive(Default, Clone, serde::Serialize)]
pub struct StatsData {
    pub total_sessions: u64,
    pub total_messages: u64,
    pub total_tool_calls: u64,
    pub cache_hit_rate: f64,
    pub first_session_date: String,
    pub last_computed_date: String,
    pub daily_activity: Vec<DailyActivity>,
    pub daily_lookup: HashMap<String, usize>, // date -> index into daily_activity
    pub model_usage: Vec<ModelUsageEntry>,
    pub longest_session: Option<LongestSession>,
    pub hour_counts: [u64; 24],
    pub daily_model_tokens: Vec<DailyModelTokens>,
    pub all_models: Vec<String>, // deduplicated, sorted by total usage desc
}

#[derive(Default, Clone, serde::Serialize)]
pub struct DailyActivity {
    pub date: String,
    pub messages: u64,
    pub sessions: u64,
    pub tool_calls: u64,
}

#[derive(Default, Clone, serde::Serialize)]
pub struct ModelUsageEntry {
    pub model: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_tokens: u64,
    pub cache_creation_tokens: u64,
    pub total_tokens: u64,
}

#[derive(Default, Clone, serde::Serialize)]
pub struct LongestSession {
    pub session_id: String,
    pub duration_ms: u64,
    pub message_count: u64,
}

#[derive(Default, Clone, serde::Serialize)]
pub struct DailyModelTokens {
    pub date: String,
    pub tokens_by_model: HashMap<String, u64>,
}

#[derive(Default, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum StatsPeriod {
    #[default]
    AllTime,
    Last7Days,
    Last30Days,
}

#[derive(Default, Clone, serde::Serialize)]
pub struct StatsSummary {
    pub active_days: usize,
    pub total_days: usize,
    pub current_streak: usize,
    pub longest_streak: usize,
    pub most_active_day: String,
    pub most_active_day_msgs: u64,
    pub favorite_model: String,
    pub total_tokens: u64,
}

pub fn load(paths: &Paths) -> StatsData {
    let path = paths.claude_dir.join("stats-cache.json");
    let json = super::read_json(&path);

    if json.is_null() {
        return StatsData::default();
    }

    let total_sessions = json["totalSessions"].as_u64().unwrap_or(0);
    let total_messages = json["totalMessages"].as_u64().unwrap_or(0);

    let raw_first = json["firstSessionDate"].as_str().unwrap_or("");
    let first_session_date = if raw_first.len() >= 10 {
        raw_first[..10].to_string()
    } else {
        raw_first.to_string()
    };
    let last_computed_date = json["lastComputedDate"].as_str().unwrap_or("").to_string();

    let daily_activity: Vec<DailyActivity> = json["dailyActivity"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .map(|v| DailyActivity {
                    date: v["date"].as_str().unwrap_or("").to_string(),
                    messages: v["messageCount"].as_u64().unwrap_or(0),
                    sessions: v["sessionCount"].as_u64().unwrap_or(0),
                    tool_calls: v["toolCallCount"].as_u64().unwrap_or(0),
                })
                .collect()
        })
        .unwrap_or_default();

    let mut model_usage: Vec<ModelUsageEntry> = json["modelUsage"]
        .as_object()
        .map(|obj| {
            obj.iter()
                .map(|(model, v)| {
                    let input = v["inputTokens"].as_u64().unwrap_or(0);
                    let output = v["outputTokens"].as_u64().unwrap_or(0);
                    let cache_read = v["cacheReadInputTokens"].as_u64().unwrap_or(0);
                    let cache_creation = v["cacheCreationInputTokens"].as_u64().unwrap_or(0);
                    ModelUsageEntry {
                        model: model.clone(),
                        input_tokens: input,
                        output_tokens: output,
                        cache_read_tokens: cache_read,
                        cache_creation_tokens: cache_creation,
                        total_tokens: input + output + cache_read + cache_creation,
                    }
                })
                .collect()
        })
        .unwrap_or_default();
    model_usage.sort_by(|a, b| b.total_tokens.cmp(&a.total_tokens));

    let longest_session = if !json["longestSession"].is_null() {
        let ls = &json["longestSession"];
        Some(LongestSession {
            session_id: ls["sessionId"].as_str().unwrap_or("").to_string(),
            duration_ms: ls["duration"].as_u64().unwrap_or(0),
            message_count: ls["messageCount"].as_u64().unwrap_or(0),
        })
    } else {
        None
    };

    let total_tool_calls: u64 = daily_activity.iter().map(|d| d.tool_calls).sum();

    let daily_lookup: HashMap<String, usize> = daily_activity
        .iter()
        .enumerate()
        .map(|(i, d)| (d.date.clone(), i))
        .collect();

    // Cache hit rate: cache_read / (cache_read + input) across all models
    let total_input: u64 = model_usage.iter().map(|m| m.input_tokens).sum();
    let total_cache_read: u64 = model_usage.iter().map(|m| m.cache_read_tokens).sum();
    let cache_hit_rate = if total_cache_read + total_input > 0 {
        total_cache_read as f64 / (total_cache_read + total_input) as f64
    } else {
        0.0
    };

    let mut hour_counts = [0u64; 24];
    if let Some(obj) = json["hourCounts"].as_object() {
        for (key, val) in obj {
            if let Ok(h) = key.parse::<usize>() {
                if h < 24 {
                    hour_counts[h] = val.as_u64().unwrap_or(0);
                }
            }
        }
    }

    let daily_model_tokens: Vec<DailyModelTokens> = json["dailyModelTokens"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .map(|v| {
                    let date = v["date"].as_str().unwrap_or("").to_string();
                    let tokens_by_model = v["tokensByModel"]
                        .as_object()
                        .map(|obj| {
                            obj.iter()
                                .map(|(k, v)| (k.clone(), v.as_u64().unwrap_or(0)))
                                .collect()
                        })
                        .unwrap_or_default();
                    DailyModelTokens {
                        date,
                        tokens_by_model,
                    }
                })
                .collect()
        })
        .unwrap_or_default();

    // Deduplicated model names sorted by total usage desc (matches model_usage order)
    let all_models: Vec<String> = model_usage.iter().map(|m| m.model.clone()).collect();

    StatsData {
        total_sessions,
        total_messages,
        total_tool_calls,
        cache_hit_rate,
        first_session_date,
        last_computed_date,
        daily_activity,
        daily_lookup,
        model_usage,
        longest_session,
        hour_counts,
        daily_model_tokens,
        all_models,
    }
}

// ── Date utilities ──────────────────────────────────────────────────────

/// Day of week: 0=Sunday, 1=Monday, …, 6=Saturday (GitHub convention).
pub fn day_of_week(y: i32, m: u32, d: u32) -> usize {
    static T: [i32; 12] = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
    let y = if m < 3 { y - 1 } else { y };
    ((y + y / 4 - y / 100 + y / 400 + T[m as usize - 1] + d as i32) % 7) as usize
}

/// Today's date from the system clock.
pub fn today() -> (i32, u32, u32) {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    civil_from_days(secs as i64 / 86400)
}

/// Howard Hinnant's civil-date-from-days algorithm (days since 1970-01-01).
fn civil_from_days(z: i64) -> (i32, u32, u32) {
    let z = z + 719468;
    let era = z.div_euclid(146097);
    let doe = (z - era * 146097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y as i32, m, d)
}

pub fn parse_date(s: &str) -> Option<(i32, u32, u32)> {
    if s.len() < 10 {
        return None;
    }
    let y = s[0..4].parse().ok()?;
    let m = s[5..7].parse().ok()?;
    let d = s[8..10].parse().ok()?;
    Some((y, m, d))
}

pub fn format_date(y: i32, m: u32, d: u32) -> String {
    format!("{y:04}-{m:02}-{d:02}")
}

fn days_in_month(y: i32, m: u32) -> u32 {
    match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if y % 4 == 0 && (y % 100 != 0 || y % 400 == 0) {
                29
            } else {
                28
            }
        }
        _ => 30,
    }
}

/// Add `n` days to a date (n can be negative).
pub fn add_days(y: i32, m: u32, d: u32, n: i32) -> (i32, u32, u32) {
    let (mut y, mut m, mut d) = (y, m, d as i32);
    d += n;
    while d < 1 {
        m -= 1;
        if m == 0 {
            m = 12;
            y -= 1;
        }
        d += days_in_month(y, m) as i32;
    }
    loop {
        let dim = days_in_month(y, m) as i32;
        if d <= dim {
            break;
        }
        d -= dim;
        m += 1;
        if m > 12 {
            m = 1;
            y += 1;
        }
    }
    (y, m, d as u32)
}

/// Generate the past-year GitHub-style heatmap grid.
/// Returns `(dates, today_str, n_weeks)`.
/// `dates` is in chronological order (Sun W1, Mon W1, … Sat W1, Sun W2, …).
/// Access as `dates[col * 7 + row]` where row 0=Sun … 6=Sat.
pub fn year_heatmap_dates() -> (Vec<String>, String, usize) {
    let (ty, tm, td) = today();
    let today_str = format_date(ty, tm, td);

    // ~365 days back
    let (sy, sm, sd) = add_days(ty, tm, td, -364);
    // Rewind to Sunday
    let dow = day_of_week(sy, sm, sd) as i32;
    let (sy, sm, sd) = add_days(sy, sm, sd, -dow);

    // Fill through Saturday of today's week
    let dow_end = day_of_week(ty, tm, td);
    let (ey, em, ed) = add_days(ty, tm, td, 6 - dow_end as i32);
    let end_str = format_date(ey, em, ed);

    let mut dates = Vec::with_capacity(371);
    let (mut cy, mut cm, mut cd) = (sy, sm, sd);
    loop {
        let s = format_date(cy, cm, cd);
        dates.push(s.clone());
        if s == end_str {
            break;
        }
        let (ny, nm, nd) = add_days(cy, cm, cd, 1);
        cy = ny;
        cm = nm;
        cd = nd;
        if dates.len() > 400 {
            break;
        }
    }

    let n_weeks = dates.len().div_ceil(7);
    (dates, today_str, n_weeks)
}

// ── Epoch conversion ────────────────────────────────────────────────────

/// Convert a civil date to days since 1970-01-01 (inverse of `civil_from_days`).
pub fn days_since_epoch(y: i32, m: u32, d: u32) -> i64 {
    let y = y as i64;
    let (y, m) = if m <= 2 { (y - 1, m + 9) } else { (y, m - 3) };
    let era = y.div_euclid(400);
    let yoe = (y - era * 400) as u32;
    let doy = (153 * m + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe as i64 - 719468
}

// ── Period helpers ──────────────────────────────────────────────────────

impl StatsPeriod {
    pub fn label(self) -> &'static str {
        match self {
            Self::AllTime => "All time",
            Self::Last7Days => "Last 7 days",
            Self::Last30Days => "Last 30 days",
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::AllTime => Self::Last7Days,
            Self::Last7Days => Self::Last30Days,
            Self::Last30Days => Self::AllTime,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Self::AllTime => Self::Last30Days,
            Self::Last30Days => Self::Last7Days,
            Self::Last7Days => Self::AllTime,
        }
    }

    /// Returns `(start_date, end_date)` as "YYYY-MM-DD" strings.
    /// `AllTime` returns `(None, None)`.
    pub fn date_range(self) -> (Option<String>, Option<String>) {
        match self {
            Self::AllTime => (None, None),
            Self::Last7Days => {
                let (y, m, d) = today();
                let end = format_date(y, m, d);
                let (sy, sm, sd) = add_days(y, m, d, -6);
                (Some(format_date(sy, sm, sd)), Some(end))
            }
            Self::Last30Days => {
                let (y, m, d) = today();
                let end = format_date(y, m, d);
                let (sy, sm, sd) = add_days(y, m, d, -29);
                (Some(format_date(sy, sm, sd)), Some(end))
            }
        }
    }
}

// ── Summary computation ─────────────────────────────────────────────────

pub fn compute_summary(stats: &StatsData, start: Option<&str>, end: Option<&str>) -> StatsSummary {
    // Filter daily_activity to the date range
    let filtered: Vec<&DailyActivity> = stats
        .daily_activity
        .iter()
        .filter(|d| {
            if let Some(s) = start {
                if d.date.as_str() < s {
                    return false;
                }
            }
            if let Some(e) = end {
                if d.date.as_str() > e {
                    return false;
                }
            }
            true
        })
        .collect();

    let active_days = filtered.iter().filter(|d| d.messages > 0).count();

    // Total days span
    let total_days = if filtered.is_empty() {
        0
    } else {
        let first = filtered.first().unwrap().date.as_str();
        let last_date = end.unwrap_or_else(|| {
            let (y, m, d) = today();
            // leak a tiny string — this runs at most once per render
            Box::leak(format_date(y, m, d).into_boxed_str())
        });
        if let (Some(f), Some(l)) = (parse_date(first), parse_date(last_date)) {
            (days_since_epoch(l.0, l.1, l.2) - days_since_epoch(f.0, f.1, f.2) + 1).max(0) as usize
        } else {
            0
        }
    };

    // Most active day
    let (most_active_day, most_active_day_msgs) = filtered
        .iter()
        .max_by_key(|d| d.messages)
        .map(|d| (d.date.clone(), d.messages))
        .unwrap_or_default();

    // Streaks — need a set of active dates for O(1) lookup
    let active_set: std::collections::HashSet<&str> = filtered
        .iter()
        .filter(|d| d.messages > 0)
        .map(|d| d.date.as_str())
        .collect();

    // Current streak: walk backwards from today
    let (ty, tm, td) = today();
    let today_str = format_date(ty, tm, td);
    let mut current_streak = 0usize;
    {
        let (mut cy, mut cm, mut cd) = (ty, tm, td);
        // If today has no activity yet, start from yesterday
        if !active_set.contains(today_str.as_str()) {
            let (ny, nm, nd) = add_days(cy, cm, cd, -1);
            cy = ny;
            cm = nm;
            cd = nd;
        }
        loop {
            let ds = format_date(cy, cm, cd);
            if !active_set.contains(ds.as_str()) {
                break;
            }
            current_streak += 1;
            let (ny, nm, nd) = add_days(cy, cm, cd, -1);
            cy = ny;
            cm = nm;
            cd = nd;
        }
    }

    // Longest streak: scan all active dates forward
    let mut longest_streak = 0usize;
    {
        let mut sorted_dates: Vec<i64> = active_set
            .iter()
            .filter_map(|s| parse_date(s).map(|(y, m, d)| days_since_epoch(y, m, d)))
            .collect();
        sorted_dates.sort_unstable();
        let mut run = 0usize;
        for i in 0..sorted_dates.len() {
            if i == 0 || sorted_dates[i] == sorted_dates[i - 1] + 1 {
                run += 1;
            } else {
                run = 1;
            }
            longest_streak = longest_streak.max(run);
        }
    }

    // Favorite model + total tokens
    let (favorite_model, total_tokens) = if start.is_none() {
        // AllTime — use pre-aggregated model_usage
        let fav = stats
            .model_usage
            .first()
            .map(|m| m.model.clone())
            .unwrap_or_default();
        let total: u64 = stats.model_usage.iter().map(|m| m.total_tokens).sum();
        (fav, total)
    } else {
        // Filtered period — re-aggregate from daily_model_tokens
        let mut totals: HashMap<String, u64> = HashMap::new();
        for dmt in &stats.daily_model_tokens {
            if let Some(s) = start {
                if dmt.date.as_str() < s {
                    continue;
                }
            }
            if let Some(e) = end {
                if dmt.date.as_str() > e {
                    continue;
                }
            }
            for (model, &tokens) in &dmt.tokens_by_model {
                *totals.entry(model.clone()).or_default() += tokens;
            }
        }
        let total: u64 = totals.values().sum();
        let fav = totals
            .iter()
            .max_by_key(|(_, &v)| v)
            .map(|(k, _)| k.clone())
            .unwrap_or_default();
        (fav, total)
    };

    StatsSummary {
        active_days,
        total_days,
        current_streak,
        longest_streak,
        most_active_day,
        most_active_day_msgs,
        favorite_model,
        total_tokens,
    }
}
