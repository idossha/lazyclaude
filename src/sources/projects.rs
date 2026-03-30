use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[derive(Clone, Debug, serde::Serialize)]
pub struct Project {
    pub name: String,
    pub short_name: String,
    pub encoded: String,
    pub dir: PathBuf,
    pub last_active: Option<SystemTime>,
    pub exists: bool,
}

/// Discover all Claude Code projects by scanning `claude_dir/projects/`.
///
/// Each subdirectory name is an encoded absolute path where `/` was replaced
/// with `-`, so `-Users-foo-bar` represents `/Users/foo/bar`.
pub fn discover(claude_dir: &Path) -> Vec<Project> {
    let projects_dir = claude_dir.join("projects");

    let entries = match std::fs::read_dir(&projects_dir) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };

    let mut projects = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let encoded = match path.file_name() {
            Some(n) => n.to_string_lossy().to_string(),
            None => continue,
        };

        let decoded = decode_project_name(&encoded);
        let short_name = Path::new(&decoded)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| decoded.clone());

        let exists = Path::new(&decoded).exists();
        let last_active = most_recent_mtime(&path);

        projects.push(Project {
            name: decoded,
            short_name,
            encoded,
            dir: path,
            last_active,
            exists,
        });
    }

    // Sort by last_active descending (most recent first), None values last.
    projects.sort_by(|a, b| match (&b.last_active, &a.last_active) {
        (Some(tb), Some(ta)) => tb.cmp(ta),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    });

    projects
}

/// Decode an encoded project directory name back to an absolute path.
///
/// The encoding replaces every `/` in the absolute path with `-`, so
/// `/Users/foo/bar` becomes `-Users-foo-bar`. To reverse this we replace
/// every `-` with `/`.
fn decode_project_name(encoded: &str) -> String {
    encoded.replace('-', "/")
}

/// Find the most recently modified file (top-level only) in `dir`.
fn most_recent_mtime(dir: &Path) -> Option<SystemTime> {
    let entries = std::fs::read_dir(dir).ok()?;
    let mut latest: Option<SystemTime> = None;

    for entry in entries.flatten() {
        let meta = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };

        if let Ok(mtime) = meta.modified() {
            latest = Some(match latest {
                Some(current) if current >= mtime => current,
                _ => mtime,
            });
        }
    }

    latest
}
