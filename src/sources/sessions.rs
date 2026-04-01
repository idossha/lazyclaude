use std::fs;
use std::io::BufRead;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[derive(Clone, Debug, serde::Serialize)]
pub struct Session {
    pub id: String,
    pub path: PathBuf,
    pub modified: Option<SystemTime>,
    pub size: u64,
    pub summary: Option<String>,
}

pub fn load_sessions(project_dir: &Path) -> Vec<Session> {
    let mut sessions = Vec::new();

    let entries = match fs::read_dir(project_dir) {
        Ok(e) => e,
        Err(e) => {
            if e.kind() != std::io::ErrorKind::NotFound {
                tracing::warn!(
                    "Failed to read sessions dir {}: {}",
                    project_dir.display(),
                    e
                );
            }
            return sessions;
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().map(|e| e == "jsonl").unwrap_or(false) {
            let id = path
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();

            let (size, modified) = match fs::metadata(&path) {
                Ok(meta) => (meta.len(), meta.modified().ok()),
                Err(_) => continue,
            };

            let summary = extract_summary(&path);

            sessions.push(Session {
                id,
                path,
                modified,
                size,
                summary,
            });
        }
    }

    sessions.sort_by(|a, b| b.modified.cmp(&a.modified));
    sessions
}

/// Read up to the first 5 lines of a JSONL file and return the content of the
/// first message with `"role": "user"`, truncated to 100 characters.
fn extract_summary(path: &Path) -> Option<String> {
    let file = fs::File::open(path).ok()?;
    let reader = std::io::BufReader::new(file);

    for line in reader.lines().take(5) {
        let line = match line {
            Ok(l) => l,
            Err(_) => continue,
        };

        let val: serde_json::Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(_) => continue,
        };

        if val.get("role").and_then(|r| r.as_str()) != Some("user") {
            continue;
        }

        let content = val.get("content")?;
        let text = extract_text_from_content(content)?;
        let truncated = truncate(&text, 100);
        return Some(truncated);
    }

    None
}

/// Extract a plain-text string from a `"content"` field.
///
/// If content is a string, return it directly.
/// If content is an array, find the first object with `"type": "text"` and
/// return its `"text"` field.
fn extract_text_from_content(content: &serde_json::Value) -> Option<String> {
    if let Some(s) = content.as_str() {
        return Some(s.to_string());
    }

    if let Some(arr) = content.as_array() {
        for item in arr {
            if item.get("type").and_then(|t| t.as_str()) == Some("text") {
                if let Some(text) = item.get("text").and_then(|t| t.as_str()) {
                    return Some(text.to_string());
                }
            }
        }
    }

    None
}

/// Truncate a string to at most `max_chars` characters, appending "..." if
/// truncation occurred.
fn truncate(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_chars).collect();
        format!("{truncated}...")
    }
}
