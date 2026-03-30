use crate::config::Paths;
use crate::sources::{parse_frontmatter, MemoryData, MemoryFile};

pub fn load(paths: &Paths) -> MemoryData {
    let mem_dir = paths.memory_dir();
    let project = paths
        .project_root
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    let mut files = Vec::new();

    if let Ok(entries) = std::fs::read_dir(&mem_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map(|e| e == "md").unwrap_or(false) {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    let filename = path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string();
                    let (fm, body) = parse_frontmatter(&content);
                    files.push(MemoryFile {
                        path: path.clone(),
                        name: fm
                            .get("name")
                            .cloned()
                            .unwrap_or_else(|| filename.trim_end_matches(".md").to_string()),
                        description: fm.get("description").cloned().unwrap_or_default(),
                        mem_type: fm
                            .get("type")
                            .cloned()
                            .unwrap_or_else(|| "user".to_string()),
                        body,
                        filename,
                    });
                }
            }
        }
    }

    files.sort_by(|a, b| a.name.cmp(&b.name));

    MemoryData {
        files,
        project,
        dir: mem_dir,
    }
}
