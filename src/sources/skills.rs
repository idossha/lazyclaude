use crate::config::Paths;
use crate::sources::{parse_frontmatter, Skill};

pub fn load(paths: &Paths) -> Vec<Skill> {
    let mut skills = Vec::new();

    scan_dir(&mut skills, &paths.user_skills_dir(), "user");
    scan_dir(&mut skills, &paths.project_skills_dir(), "project");

    skills.sort_by(|a, b| a.name.cmp(&b.name));
    skills
}

fn scan_dir(skills: &mut Vec<Skill>, dir: &std::path::Path, scope: &str) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let dir_path = entry.path();
        if !dir_path.is_dir() {
            continue;
        }
        let skill_file = dir_path.join("SKILL.md");
        if let Ok(content) = std::fs::read_to_string(&skill_file) {
            let dir_name = dir_path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            let (fm, body) = parse_frontmatter(&content);
            skills.push(Skill {
                path: skill_file,
                name: fm
                    .get("name")
                    .cloned()
                    .unwrap_or_else(|| dir_name.clone()),
                description: fm.get("description").cloned().unwrap_or_default(),
                user_invocable: fm
                    .get("user_invocable")
                    .or_else(|| fm.get("user-invocable"))
                    .map(|v| v == "true")
                    .unwrap_or(false),
                body,
                dir_name,
                scope: scope.to_string(),
            });
        }
    }
}
