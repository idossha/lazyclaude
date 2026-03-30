use crate::config::Paths;
use crate::sources::{read_json, Keybinding};

pub fn load(paths: &Paths) -> Vec<Keybinding> {
    let data = read_json(&paths.keybindings_path());
    let mut bindings = Vec::new();

    // Handle both flat array format and nested { bindings: [...] } format
    let items = data
        .as_array()
        .or_else(|| {
            data.get("bindings")
                .and_then(|v| v.as_array())
        });

    if let Some(arr) = items {
        for item in arr {
            // Nested context format: { context: "Chat", bindings: { "key": "cmd" } }
            if let Some(ctx) = item.get("context").and_then(|v| v.as_str()) {
                if let Some(ctx_bindings) = item.get("bindings").and_then(|v| v.as_object()) {
                    for (key, cmd) in ctx_bindings {
                        let command = match cmd {
                            serde_json::Value::String(s) => s.clone(),
                            serde_json::Value::Null => "(unbound)".to_string(),
                            other => other.to_string(),
                        };
                        bindings.push(Keybinding {
                            key: key.clone(),
                            command,
                            context: ctx.to_string(),
                        });
                    }
                }
            } else {
                // Flat format: { key, command, context }
                bindings.push(Keybinding {
                    key: item
                        .get("key")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    command: item
                        .get("command")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    context: item
                        .get("context")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                });
            }
        }
    }

    bindings
}
