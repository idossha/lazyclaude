use crate::config::Paths;

#[derive(Default, Clone, serde::Serialize)]
pub struct PluginsData {
    pub installed: Vec<InstalledPlugin>,
    pub blocked: Vec<BlockedPlugin>,
    pub marketplaces: Vec<Marketplace>,
}

#[derive(Default, Clone, serde::Serialize)]
pub struct InstalledPlugin {
    pub name: String,
    pub version: String,
    pub scope: crate::sources::Scope,
    pub installed_at: String,
}

impl InstalledPlugin {
    pub fn preview_body(&self) -> String {
        let mut lines = vec![format!("# {}", self.name), String::new()];
        if !self.version.is_empty() {
            lines.push(format!("Version: {}", self.version));
        }
        lines.push(format!("Scope: {}", self.scope));
        if !self.installed_at.is_empty() {
            lines.push(format!("Installed: {}", self.installed_at));
        }
        lines.join("\n")
    }
}

#[derive(Default, Clone, serde::Serialize)]
pub struct BlockedPlugin {
    pub name: String,
    pub reason: String,
    pub text: String,
}

impl BlockedPlugin {
    pub fn preview_body(&self) -> String {
        let mut lines = vec![format!("# {}", self.name), String::new()];
        lines.push(format!("Status: Blocked"));
        if !self.reason.is_empty() {
            lines.push(format!("Reason: {}", self.reason));
        }
        if !self.text.is_empty() {
            lines.push(String::new());
            lines.push(self.text.clone());
        }
        lines.join("\n")
    }
}

#[derive(Default, Clone, serde::Serialize)]
pub struct Marketplace {
    pub name: String,
    pub source_type: String,
    pub repo: String,
}

impl Marketplace {
    pub fn preview_body(&self) -> String {
        let mut lines = vec![format!("# {}", self.name), String::new()];
        lines.push(format!("Source: {}", self.source_type));
        if !self.repo.is_empty() {
            lines.push(format!("Repository: {}", self.repo));
        }
        lines.join("\n")
    }
}

pub fn load(paths: &Paths) -> PluginsData {
    let dir = paths.claude_dir.join("plugins");
    PluginsData {
        installed: load_installed(&dir),
        blocked: load_blocked(&dir),
        marketplaces: load_marketplaces(&dir),
    }
}

fn load_installed(dir: &std::path::Path) -> Vec<InstalledPlugin> {
    let path = dir.join("installed_plugins.json");
    let json = super::read_json(&path.to_path_buf());

    let mut plugins = Vec::new();
    if let Some(obj) = json["plugins"].as_object() {
        for (name, installations) in obj {
            if let Some(arr) = installations.as_array() {
                for inst in arr {
                    plugins.push(InstalledPlugin {
                        name: name.clone(),
                        version: inst["version"].as_str().unwrap_or("").to_string(),
                        scope: match inst["scope"].as_str().unwrap_or("user") {
                            "project" => crate::sources::Scope::Project,
                            "local" => crate::sources::Scope::Local,
                            _ => crate::sources::Scope::User,
                        },
                        installed_at: inst["installedAt"]
                            .as_str()
                            .map(|s| {
                                if s.len() >= 10 {
                                    s[..10].to_string()
                                } else {
                                    s.to_string()
                                }
                            })
                            .unwrap_or_default(),
                    });
                }
            }
        }
    }
    plugins
}

fn load_blocked(dir: &std::path::Path) -> Vec<BlockedPlugin> {
    let path = dir.join("blocklist.json");
    let json = super::read_json(&path.to_path_buf());

    json["plugins"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .map(|v| BlockedPlugin {
                    name: v["plugin"].as_str().unwrap_or("").to_string(),
                    reason: v["reason"].as_str().unwrap_or("").to_string(),
                    text: v["text"].as_str().unwrap_or("").to_string(),
                })
                .collect()
        })
        .unwrap_or_default()
}

fn load_marketplaces(dir: &std::path::Path) -> Vec<Marketplace> {
    let path = dir.join("known_marketplaces.json");
    let json = super::read_json(&path.to_path_buf());

    let mut marketplaces = Vec::new();
    if let Some(obj) = json.as_object() {
        for (name, val) in obj {
            marketplaces.push(Marketplace {
                name: name.clone(),
                source_type: val["source"]["source"].as_str().unwrap_or("").to_string(),
                repo: val["source"]["repo"].as_str().unwrap_or("").to_string(),
            });
        }
    }
    marketplaces
}

// ── Helpers ──────────────────────────────────────────────────────────────

/// Convert current epoch time to an ISO date string (YYYY-MM-DD).
/// Uses Howard Hinnant's civil calendar algorithm — correct for all dates.
fn epoch_to_date_string() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let days = (secs / 86400) as i64;
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02}")
}

// ── CRUD operations ─────────────────────────────────────────────────────

/// Remove an installed plugin by name.
pub fn remove(paths: &Paths, name: &str) -> anyhow::Result<()> {
    let path = paths.claude_dir.join("plugins").join("installed_plugins.json");
    let mut data = super::read_json(&path);
    if let Some(obj) = data.get_mut("plugins").and_then(|v| v.as_object_mut()) {
        obj.remove(name);
    }
    super::write_json(&path, &data)
}

/// Install a plugin by recording it in installed_plugins.json.
pub fn install(paths: &Paths, name: &str, version: &str, marketplace: &str) -> anyhow::Result<()> {
    let path = paths.claude_dir.join("plugins").join("installed_plugins.json");
    let mut data = super::read_json(&path);

    // Ensure top-level structure exists
    if !data.is_object() {
        data = serde_json::json!({"plugins": {}});
    }
    if data.get("plugins").is_none() {
        data["plugins"] = serde_json::json!({});
    }

    let now = epoch_to_date_string();

    let entry = serde_json::json!({
        "version": version,
        "scope": "user",
        "installedAt": now,
        "marketplace": marketplace
    });

    if let Some(obj) = data.get_mut("plugins").and_then(|v| v.as_object_mut()) {
        if let Some(arr) = obj.get_mut(name).and_then(|v| v.as_array_mut()) {
            // Avoid duplicates — replace existing user-scope entry
            arr.retain(|v| v["scope"].as_str() != Some("user"));
            arr.push(entry);
        } else {
            obj.insert(name.to_string(), serde_json::json!([entry]));
        }
    }

    super::write_json(&path, &data)
}

/// Remove a plugin from the blocklist by name.
pub fn unblock(paths: &Paths, name: &str) -> anyhow::Result<()> {
    let path = paths.claude_dir.join("plugins").join("blocklist.json");
    let mut data = super::read_json(&path);
    if let Some(arr) = data.get_mut("plugins").and_then(|v| v.as_array_mut()) {
        arr.retain(|v| v["plugin"].as_str() != Some(name));
    }
    super::write_json(&path, &data)
}

