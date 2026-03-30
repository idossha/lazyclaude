use crate::config::Paths;
use crate::sources::{read_json, McpData, McpServer};
use std::collections::HashMap;
use std::path::PathBuf;

pub fn load(paths: &Paths) -> McpData {
    McpData {
        user: parse_scope(&paths.mcp_path("user")),
        project: parse_scope(&paths.mcp_path("project")),
    }
}

fn parse_scope(path: &PathBuf) -> Vec<McpServer> {
    let data = read_json(path);
    let mut servers = Vec::new();

    if let Some(obj) = data.get("mcpServers").and_then(|v| v.as_object()) {
        for (name, config) in obj {
            let disabled = config
                .get("disabled")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let command = config
                .get("command")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let args: Vec<String> = config
                .get("args")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();
            let env: HashMap<String, String> = config
                .get("env")
                .and_then(|v| v.as_object())
                .map(|obj| {
                    obj.iter()
                        .map(|(k, v)| (k.clone(), v.as_str().unwrap_or("").to_string()))
                        .collect()
                })
                .unwrap_or_default();

            servers.push(McpServer {
                name: name.clone(),
                command,
                args,
                env,
                disabled,
            });
        }
    }

    servers.sort_by(|a, b| a.name.cmp(&b.name));
    servers
}

/// Add an MCP server to the specified scope
pub fn add(
    paths: &Paths,
    scope: &str,
    name: &str,
    command: &str,
    args: &[String],
) -> anyhow::Result<()> {
    use crate::sources::write_json;
    let path = paths.mcp_path(scope);
    let mut data = read_json(&path);
    if data.is_null() {
        data = serde_json::json!({});
    }
    let servers = data
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("invalid mcp.json"))?
        .entry("mcpServers")
        .or_insert(serde_json::json!({}));
    if let Some(obj) = servers.as_object_mut() {
        obj.insert(
            name.to_string(),
            serde_json::json!({
                "command": command,
                "args": args,
            }),
        );
    }
    write_json(&path, &data)
}

/// Remove an MCP server from the specified scope
pub fn remove(paths: &Paths, scope: &str, name: &str) -> anyhow::Result<()> {
    use crate::sources::write_json;
    let path = paths.mcp_path(scope);
    let mut data = read_json(&path);
    if let Some(servers) = data.get_mut("mcpServers").and_then(|v| v.as_object_mut()) {
        servers.remove(name);
    }
    write_json(&path, &data)
}

/// Toggle disabled state of an MCP server
pub fn toggle(paths: &Paths, scope: &str, name: &str) -> anyhow::Result<()> {
    use crate::sources::write_json;
    let path = paths.mcp_path(scope);
    let mut data = read_json(&path);
    if let Some(server) = data
        .get_mut("mcpServers")
        .and_then(|v| v.as_object_mut())
        .and_then(|o| o.get_mut(name))
    {
        let current = server
            .get("disabled")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        if let Some(obj) = server.as_object_mut() {
            if current {
                obj.remove("disabled");
            } else {
                obj.insert("disabled".to_string(), serde_json::json!(true));
            }
        }
    }
    write_json(&path, &data)
}
