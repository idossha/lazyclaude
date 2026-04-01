use crate::config::Paths;
use crate::sources::{parse_frontmatter, Command, Scope};

pub fn load(paths: &Paths) -> Vec<Command> {
    let mut commands = Vec::new();

    scan_dir(&mut commands, &paths.user_commands_dir(), Scope::User);
    scan_dir(&mut commands, &paths.project_commands_dir(), Scope::Project);

    commands.sort_by(|a, b| a.name.cmp(&b.name));
    commands
}

/// Scan for flat .md files in the commands directory.
/// Commands are stored as `<name>.md` files directly in the commands directory.
/// These are legacy slash commands (prefer skills for new commands).
fn scan_dir(commands: &mut Vec<Command>, dir: &std::path::Path, scope: Scope) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) => {
            if e.kind() != std::io::ErrorKind::NotFound {
                tracing::warn!("Failed to read commands dir {}: {}", dir.display(), e);
            }
            return;
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            continue;
        }
        if path.extension().map(|e| e == "md").unwrap_or(false) {
            let content = match std::fs::read_to_string(&path) {
                Ok(c) => c,
                Err(e) => {
                    tracing::warn!("Failed to read command file {}: {}", path.display(), e);
                    continue;
                }
            };
            let file_stem = path
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            let (fm, body) = parse_frontmatter(&content);
            commands.push(Command {
                path,
                name: fm.get("name").cloned().unwrap_or_else(|| file_stem.clone()),
                description: fm.get("description").cloned().unwrap_or_default(),
                body,
                file_name: file_stem,
                scope,
            });
        }
    }
}

/// Delete a command by removing its .md file.
pub fn remove(command_file: &std::path::Path) -> anyhow::Result<()> {
    std::fs::remove_file(command_file)?;
    Ok(())
}
