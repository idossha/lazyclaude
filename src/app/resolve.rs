use super::*;

/// Parse a scope string into a `Scope` enum value.
fn parse_scope(s: &str) -> Option<Scope> {
    match s {
        "user" => Some(Scope::User),
        "project" => Some(Scope::Project),
        "local" => Some(Scope::Local),
        _ => None,
    }
}

impl App {
    /// Resolve current cursor to (kind, scope, rule_index, rule_text) for Settings permissions.
    /// Uses synthetic paths stored by build_detail_items (format: "perm:kind:scope:index").
    /// Returns None if cursor is on a header or non-permission item.
    pub(crate) fn resolve_permission(&self) -> Option<(String, Scope, usize, String)> {
        let idx = self.panel_offset();
        let p = self.item_paths.get(idx).and_then(|p| p.as_ref())?;
        let s = p.to_string_lossy();
        let parts: Vec<&str> = s.splitn(4, ':').collect();
        if parts.len() < 4 || parts[0] != "perm" {
            return None;
        }
        let kind = parts[1].to_string();
        let scope = parse_scope(parts[2])?;
        let rule_index: usize = parts[3].parse().ok()?;
        let perms = &self.data.settings.permissions;
        let rules = if kind == "allow" { &perms.allow } else { &perms.deny };
        let rule_text = rules.get(rule_index).map(|r| r.rule.clone())?;
        Some((kind, scope, rule_index, rule_text))
    }

    /// Resolve the current panel offset to a (scope, server_name) for MCP.
    /// Returns None if the cursor is on a header or "none" hint.
    pub(crate) fn resolve_mcp_server(&self) -> Option<(Scope, String)> {
        let idx = self.panel_offset();
        self.item_paths.get(idx)
            .and_then(|p| p.as_ref())
            .and_then(|p| {
                let s = p.to_string_lossy();
                s.split_once(':').and_then(|(scope_str, name)| {
                    parse_scope(scope_str).map(|scope| (scope, name.to_string()))
                })
            })
    }

    /// Resolve current cursor to (kind, name) for the Plugins panel.
    /// Uses synthetic paths stored by build_detail_items (format: "plugin:kind:name").
    /// kind is "installed", "blocked", or "marketplace". Returns None on headers.
    pub(crate) fn resolve_plugin(&self) -> Option<(String, String)> {
        let idx = self.panel_offset();
        self.item_paths.get(idx)
            .and_then(|p| p.as_ref())
            .and_then(|p| {
                let s = p.to_string_lossy();
                let parts: Vec<&str> = s.splitn(3, ':').collect();
                if parts.len() == 3 && parts[0] == "plugin" {
                    Some((parts[1].to_string(), parts[2].to_string()))
                } else {
                    None
                }
            })
    }
}
