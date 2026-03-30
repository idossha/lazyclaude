use crate::config::Paths;
use crate::sources::{read_json, Hook, Scope};

pub fn load(paths: &Paths) -> Vec<Hook> {
    let mut results = Vec::new();

    for scope in &[Scope::User, Scope::Project, Scope::Local] {
        let data = read_json(&paths.settings_path(scope.as_str()));
        if let Some(hooks) = data.get("hooks").and_then(|v| v.as_object()) {
            for (event, groups) in hooks {
                if let Some(groups_arr) = groups.as_array() {
                    for group in groups_arr {
                        let matcher = group
                            .get("matcher")
                            .and_then(|v| v.as_str())
                            .unwrap_or("*")
                            .to_string();
                        if let Some(hooks_arr) = group.get("hooks").and_then(|v| v.as_array()) {
                            for hook in hooks_arr {
                                results.push(Hook {
                                    event: event.clone(),
                                    matcher: matcher.clone(),
                                    command: hook
                                        .get("command")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("")
                                        .to_string(),
                                    hook_type: hook
                                        .get("type")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("command")
                                        .to_string(),
                                    scope: *scope,
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    results.sort_by(|a, b| a.event.cmp(&b.event).then(a.matcher.cmp(&b.matcher)));
    results
}
