use crate::config::Paths;

#[derive(Default, Clone, serde::Serialize)]
pub struct TodoItem {
    pub id: String,
    pub content: String,
    pub status: String,
    pub session_file: String,
}

pub fn load(paths: &Paths) -> Vec<TodoItem> {
    let dir = paths.claude_dir.join("todos");
    let mut items = Vec::new();

    let entries = match std::fs::read_dir(&dir) {
        Ok(e) => e,
        Err(e) => {
            if e.kind() != std::io::ErrorKind::NotFound {
                tracing::warn!("Failed to read todos dir {}: {}", dir.display(), e);
            }
            return items;
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().map(|e| e == "json").unwrap_or(false) {
            let session_name = path
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();

            let raw = match std::fs::read_to_string(&path) {
                Ok(c) => c,
                Err(e) => {
                    tracing::warn!("Failed to read todo file {}: {}", path.display(), e);
                    continue;
                }
            };

            let arr: Vec<serde_json::Value> = match serde_json::from_str(&raw) {
                Ok(v) => v,
                Err(e) => {
                    tracing::warn!("Failed to parse todo JSON {}: {}", path.display(), e);
                    continue;
                }
            };

            for val in arr {
                let id = val
                    .get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let text = val
                    .get("content")
                    .or_else(|| val.get("subject"))
                    .or_else(|| val.get("description"))
                    .or_else(|| val.get("text"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let status = val
                    .get("status")
                    .and_then(|v| v.as_str())
                    .unwrap_or("pending")
                    .to_string();

                if !id.is_empty() || !text.is_empty() {
                    items.push(TodoItem {
                        id: if id.is_empty() {
                            format!("{}", items.len())
                        } else {
                            id
                        },
                        content: text,
                        status,
                        session_file: session_name.clone(),
                    });
                }
            }
        }
    }

    items
}
