use crate::config::Paths;

#[derive(Default, Clone, serde::Serialize)]
pub struct StatsData {
    pub total_sessions: u64,
    pub total_messages: u64,
    pub first_session_date: String,
    pub last_computed_date: String,
    pub daily_activity: Vec<DailyActivity>,
    pub model_usage: Vec<ModelUsageEntry>,
    pub longest_session: Option<LongestSession>,
    pub hour_counts: [u64; 24],
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
    let last_computed_date = json["lastComputedDate"]
        .as_str()
        .unwrap_or("")
        .to_string();

    let daily_activity = json["dailyActivity"]
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

    StatsData {
        total_sessions,
        total_messages,
        first_session_date,
        last_computed_date,
        daily_activity,
        model_usage,
        longest_session,
        hour_counts,
    }
}
