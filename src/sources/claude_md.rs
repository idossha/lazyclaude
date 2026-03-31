use crate::config::Paths;
use crate::sources::{ClaudeMdFile, Scope};
use std::path::PathBuf;

pub fn load(paths: &Paths) -> Vec<ClaudeMdFile> {
    let mut files = Vec::new();

    // Project-level CLAUDE.md
    check_file(
        &mut files,
        paths.project_root.join("CLAUDE.md"),
        Scope::Project,
        "claude_md",
    );

    // Project-level .claude/CLAUDE.md (alternate location)
    check_file(
        &mut files,
        paths.project_root.join(".claude").join("CLAUDE.md"),
        Scope::Project,
        "claude_md",
    );

    // Project-level .claude/rules/*.md
    let rules_dir = paths.project_root.join(".claude").join("rules");
    scan_rules_dir(&mut files, &rules_dir, Scope::Project);

    // User-level CLAUDE.md (~/.claude/CLAUDE.md)
    check_file(
        &mut files,
        paths.claude_dir.join("CLAUDE.md"),
        Scope::User,
        "claude_md",
    );

    // User-level rules (~/.claude/rules/*.md)
    let user_rules_dir = paths.claude_dir.join("rules");
    scan_rules_dir(&mut files, &user_rules_dir, Scope::User);

    files
}

fn scan_rules_dir(files: &mut Vec<ClaudeMdFile>, dir: &PathBuf, scope: Scope) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            // Recurse into subdirectories (e.g. rules/frontend/, rules/backend/)
            scan_rules_dir(files, &path, scope);
        } else if path.extension().map(|e| e == "md").unwrap_or(false) {
            check_file(files, path, scope, "rule");
        }
    }
}

fn check_file(files: &mut Vec<ClaudeMdFile>, path: PathBuf, scope: Scope, file_type: &str) {
    if !path.exists() {
        return;
    }
    let name = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
    let content = std::fs::read_to_string(&path).unwrap_or_default();

    files.push(ClaudeMdFile {
        path,
        name,
        scope,
        file_type: file_type.to_string(),
        content,
        size,
    });
}
