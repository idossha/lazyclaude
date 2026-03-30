use crate::config::Paths;
use crate::sources::{read_json, write_json, PermissionRule, Permissions, Scope, SettingsData, SettingsScopes};

pub fn load(paths: &Paths) -> SettingsData {
    let user = read_json(&paths.settings_path("user"));
    let project = read_json(&paths.settings_path("project"));
    let local_ = read_json(&paths.settings_path("local"));

    let effective = merge_settings(&user, &project, &local_);
    let permissions = build_permissions(&user, &project, &local_);

    SettingsData {
        permissions,
        effective,
        scopes: SettingsScopes {
            user,
            project,
            local_,
        },
    }
}

fn merge_settings(
    user: &serde_json::Value,
    project: &serde_json::Value,
    local_: &serde_json::Value,
) -> serde_json::Value {
    let mut result = user.clone();
    deep_merge(&mut result, project);
    deep_merge(&mut result, local_);
    result
}

fn deep_merge(base: &mut serde_json::Value, overlay: &serde_json::Value) {
    if let (Some(base_obj), Some(overlay_obj)) = (base.as_object_mut(), overlay.as_object()) {
        for (key, value) in overlay_obj {
            if value.is_object()
                && base_obj
                    .get(key)
                    .map(|v| v.is_object())
                    .unwrap_or(false)
            {
                deep_merge(base_obj.get_mut(key).unwrap(), value);
            } else {
                base_obj.insert(key.clone(), value.clone());
            }
        }
    }
}

fn build_permissions(
    user: &serde_json::Value,
    project: &serde_json::Value,
    local_: &serde_json::Value,
) -> Permissions {
    let mut allow = Vec::new();
    let mut deny = Vec::new();

    let mut collect = |scope: Scope, data: &serde_json::Value| {
        if let Some(perms) = data.get("permissions").and_then(|v| v.as_object()) {
            if let Some(allow_arr) = perms.get("allow").and_then(|v| v.as_array()) {
                for rule in allow_arr {
                    let rule_str = match rule {
                        serde_json::Value::String(s) => s.clone(),
                        _ => rule.to_string(),
                    };
                    allow.push(PermissionRule {
                        rule: rule_str,
                        scope,
                    });
                }
            }
            if let Some(deny_arr) = perms.get("deny").and_then(|v| v.as_array()) {
                for rule in deny_arr {
                    let rule_str = match rule {
                        serde_json::Value::String(s) => s.clone(),
                        _ => rule.to_string(),
                    };
                    deny.push(PermissionRule {
                        rule: rule_str,
                        scope,
                    });
                }
            }
        }
    };

    collect(Scope::User, user);
    collect(Scope::Project, project);
    collect(Scope::Local, local_);

    Permissions { allow, deny }
}

/// Add a permission rule to a scope
pub fn add_permission(paths: &Paths, scope: &str, kind: &str, rule: &str) -> anyhow::Result<()> {
    let path = paths.settings_path(scope);
    let mut data = read_json(&path);
    if data.is_null() {
        data = serde_json::json!({});
    }

    let perms = data
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("invalid settings.json"))?
        .entry("permissions")
        .or_insert(serde_json::json!({}));

    let list = perms
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("invalid permissions"))?
        .entry(kind)
        .or_insert(serde_json::json!([]));

    if let Some(arr) = list.as_array_mut() {
        arr.push(serde_json::json!(rule));
    }

    write_json(&path, &data)
}

/// Remove a permission rule by index from a scope
pub fn remove_permission(paths: &Paths, scope: &str, kind: &str, index: usize) -> anyhow::Result<()> {
    let path = paths.settings_path(scope);
    let mut data = read_json(&path);

    if let Some(arr) = data
        .get_mut("permissions")
        .and_then(|v| v.get_mut(kind))
        .and_then(|v| v.as_array_mut())
    {
        if index < arr.len() {
            arr.remove(index);
        }
    }

    write_json(&path, &data)
}
