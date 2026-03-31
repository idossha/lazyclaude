//! Integration tests for lazyclaude configuration source discovery.
//!
//! Each test creates a mock filesystem using `tempfile::TempDir` and verifies
//! that the loaders correctly discover files at both user and project scopes.

use std::fs;
use tempfile::TempDir;

use lazyclaude::config::Paths;
use lazyclaude::sources;
use lazyclaude::sources::Scope;

// ── Helpers ──────────────────────────────────────────────────────────────

/// Build `Paths` from two `TempDir` references.
fn make_paths(claude_dir: &TempDir, project_root: &TempDir) -> Paths {
    Paths::new(
        claude_dir.path().to_path_buf(),
        project_root.path().to_path_buf(),
    )
}

/// Build `Paths` where claude_dir is nested inside a wrapper dir.
/// This gives us control over claude_dir's parent (for user-level CLAUDE.md tests).
/// Returns (Paths, claude_dir_path) where claude_dir_path = wrapper/.claude
fn make_nested_paths(wrapper: &TempDir, project_root: &TempDir) -> (Paths, std::path::PathBuf) {
    let claude_dir = wrapper.path().join(".claude");
    fs::create_dir_all(&claude_dir).expect("create nested claude dir");
    let paths = Paths::new(claude_dir.clone(), project_root.path().to_path_buf());
    (paths, claude_dir)
}

/// Write a file, creating all intermediate directories.
fn write_fixture(base: &std::path::Path, rel: &str, content: &str) {
    let full = base.join(rel);
    if let Some(parent) = full.parent() {
        fs::create_dir_all(parent).expect("create parent dirs");
    }
    fs::write(&full, content).expect("write fixture file");
}

// ── Skills ───────────────────────────────────────────────────────────────

#[test]
fn test_skills_dual_scope() {
    let claude_dir = TempDir::new().unwrap();
    let project_root = TempDir::new().unwrap();
    let paths = make_paths(&claude_dir, &project_root);

    // User-level skill: ~/.claude/skills/my-skill/SKILL.md
    write_fixture(
        claude_dir.path(),
        "skills/my-skill/SKILL.md",
        "---\nname: My Skill\ndescription: A user skill\nuser_invocable: true\n---\nDo things.",
    );

    // Project-level skill: <project>/.claude/skills/proj-skill/SKILL.md
    write_fixture(
        project_root.path(),
        ".claude/skills/proj-skill/SKILL.md",
        "---\nname: Proj Skill\ndescription: A project skill\nuser_invocable: false\n---\nProject specific.",
    );

    let skills = sources::skills::load(&paths);

    assert_eq!(skills.len(), 2, "expected 2 skills, got {}", skills.len());

    let user_skill = skills
        .iter()
        .find(|s| s.scope == Scope::User)
        .expect("user skill");
    assert_eq!(user_skill.name, "My Skill");
    assert_eq!(user_skill.description, "A user skill");
    assert!(user_skill.user_invocable);
    assert_eq!(user_skill.dir_name, "my-skill");
    assert_eq!(user_skill.body, "Do things.");

    let proj_skill = skills
        .iter()
        .find(|s| s.scope == Scope::Project)
        .expect("project skill");
    assert_eq!(proj_skill.name, "Proj Skill");
    assert_eq!(proj_skill.description, "A project skill");
    assert!(!proj_skill.user_invocable);
    assert_eq!(proj_skill.dir_name, "proj-skill");
}

#[test]
fn test_skills_user_invocable_hyphenated_key() {
    let claude_dir = TempDir::new().unwrap();
    let project_root = TempDir::new().unwrap();
    let paths = make_paths(&claude_dir, &project_root);

    // The loader supports both "user_invocable" and "user-invocable"
    write_fixture(
        claude_dir.path(),
        "skills/hyphen-skill/SKILL.md",
        "---\nname: Hyphen Skill\ndescription: test\nuser-invocable: true\n---\nBody.",
    );

    let skills = sources::skills::load(&paths);
    assert_eq!(skills.len(), 1);
    assert!(skills[0].user_invocable);
}

#[test]
fn test_skills_name_defaults_to_dir_name() {
    let claude_dir = TempDir::new().unwrap();
    let project_root = TempDir::new().unwrap();
    let paths = make_paths(&claude_dir, &project_root);

    // Frontmatter has no "name" key
    write_fixture(
        claude_dir.path(),
        "skills/fallback-name/SKILL.md",
        "---\ndescription: no name key\n---\nBody.",
    );

    let skills = sources::skills::load(&paths);
    assert_eq!(skills.len(), 1);
    assert_eq!(skills[0].name, "fallback-name");
}

// ── Agents ───────────────────────────────────────────────────────────────

#[test]
fn test_agents_dual_scope() {
    let claude_dir = TempDir::new().unwrap();
    let project_root = TempDir::new().unwrap();
    let paths = make_paths(&claude_dir, &project_root);

    // User-level agent: ~/.claude/agents/my-agent.md (flat file)
    write_fixture(
        claude_dir.path(),
        "agents/my-agent.md",
        "---\nname: My Agent\ndescription: A user agent\nmodel: opus\n---\nAgent instructions.",
    );

    // Project-level agent: <project>/.claude/agents/proj-agent.md (flat file)
    write_fixture(
        project_root.path(),
        ".claude/agents/proj-agent.md",
        "---\nname: Proj Agent\ndescription: A project agent\nmodel: sonnet\n---\nProject agent.",
    );

    let agents = sources::agents::load(&paths);

    assert_eq!(agents.len(), 2, "expected 2 agents, got {}", agents.len());

    let user_agent = agents
        .iter()
        .find(|a| a.scope == Scope::User)
        .expect("user agent");
    assert_eq!(user_agent.name, "My Agent");
    assert_eq!(user_agent.description, "A user agent");
    assert_eq!(user_agent.model, "opus");
    assert_eq!(user_agent.dir_name, "my-agent");
    assert_eq!(user_agent.body, "Agent instructions.");

    let proj_agent = agents
        .iter()
        .find(|a| a.scope == Scope::Project)
        .expect("project agent");
    assert_eq!(proj_agent.name, "Proj Agent");
    assert_eq!(proj_agent.description, "A project agent");
    assert_eq!(proj_agent.model, "sonnet");
    assert_eq!(proj_agent.dir_name, "proj-agent");
}

#[test]
fn test_agents_name_defaults_to_dir_name() {
    let claude_dir = TempDir::new().unwrap();
    let project_root = TempDir::new().unwrap();
    let paths = make_paths(&claude_dir, &project_root);

    write_fixture(
        claude_dir.path(),
        "agents/fallback-name.md",
        "---\ndescription: no name\nmodel: haiku\n---\nBody.",
    );

    let agents = sources::agents::load(&paths);
    assert_eq!(agents.len(), 1);
    assert_eq!(agents[0].name, "fallback-name");
    assert_eq!(agents[0].model, "haiku");
}

// ── MCP ──────────────────────────────────────────────────────────────────

#[test]
fn test_mcp_dual_scope() {
    let wrapper = TempDir::new().unwrap();
    let project_root = TempDir::new().unwrap();
    let (paths, _claude_dir) = make_nested_paths(&wrapper, &project_root);

    // User-level: ~/.mcp.json (home dir, not inside .claude/)
    write_fixture(
        wrapper.path(),
        ".mcp.json",
        r#"{"mcpServers": {"user-server": {"command": "node", "args": ["user.js"]}}}"#,
    );

    // Project-level: <project>/.mcp.json (project root, not inside .claude/)
    write_fixture(
        project_root.path(),
        ".mcp.json",
        r#"{"mcpServers": {"proj-server": {"command": "python", "args": ["proj.py"]}}}"#,
    );

    let mcp = sources::mcp::load(&paths);

    assert_eq!(mcp.user.len(), 1, "expected 1 user server");
    assert_eq!(mcp.project.len(), 1, "expected 1 project server");

    assert_eq!(mcp.user[0].name, "user-server");
    assert_eq!(mcp.user[0].command, "node");
    assert_eq!(mcp.user[0].args, vec!["user.js"]);
    assert!(!mcp.user[0].disabled);

    assert_eq!(mcp.project[0].name, "proj-server");
    assert_eq!(mcp.project[0].command, "python");
    assert_eq!(mcp.project[0].args, vec!["proj.py"]);
}

#[test]
fn test_mcp_disabled_server() {
    let wrapper = TempDir::new().unwrap();
    let project_root = TempDir::new().unwrap();
    let (paths, _claude_dir) = make_nested_paths(&wrapper, &project_root);

    write_fixture(
        wrapper.path(),
        ".mcp.json",
        r#"{"mcpServers": {"off-server": {"command": "node", "args": [], "disabled": true}}}"#,
    );

    let mcp = sources::mcp::load(&paths);
    assert_eq!(mcp.user.len(), 1);
    assert!(mcp.user[0].disabled);
}

#[test]
fn test_mcp_with_env() {
    let wrapper = TempDir::new().unwrap();
    let project_root = TempDir::new().unwrap();
    let (paths, _claude_dir) = make_nested_paths(&wrapper, &project_root);

    write_fixture(
        wrapper.path(),
        ".mcp.json",
        r#"{"mcpServers": {"env-server": {"command": "node", "args": ["s.js"], "env": {"API_KEY": "secret123"}}}}"#,
    );

    let mcp = sources::mcp::load(&paths);
    assert_eq!(mcp.user.len(), 1);
    assert_eq!(
        mcp.user[0].env.get("API_KEY").map(|s| s.as_str()),
        Some("secret123")
    );
}

// ── Settings ─────────────────────────────────────────────────────────────

#[test]
fn test_settings_three_scopes() {
    let claude_dir = TempDir::new().unwrap();
    let project_root = TempDir::new().unwrap();
    let paths = make_paths(&claude_dir, &project_root);

    // User-level: ~/.claude/settings.json
    write_fixture(
        claude_dir.path(),
        "settings.json",
        r#"{"permissions": {"allow": ["Bash(echo *)"], "deny": []}}"#,
    );

    // Project-level: <project>/.claude/settings.json
    write_fixture(
        project_root.path(),
        ".claude/settings.json",
        r#"{"permissions": {"allow": [], "deny": ["Bash(rm -rf *)"]}}"#,
    );

    // Local-level: <project>/.claude/settings.local.json
    write_fixture(
        project_root.path(),
        ".claude/settings.local.json",
        r#"{"permissions": {"allow": ["Read(*)"], "deny": []}}"#,
    );

    let settings = sources::settings::load(&paths);

    // Check allow rules
    assert_eq!(
        settings.permissions.allow.len(),
        2,
        "expected 2 allow rules, got {}",
        settings.permissions.allow.len()
    );
    let user_allow = settings
        .permissions
        .allow
        .iter()
        .find(|r| r.scope == Scope::User)
        .expect("user allow rule");
    assert_eq!(user_allow.rule, "Bash(echo *)");

    let local_allow = settings
        .permissions
        .allow
        .iter()
        .find(|r| r.scope == Scope::Local)
        .expect("local allow rule");
    assert_eq!(local_allow.rule, "Read(*)");

    // Check deny rules
    assert_eq!(
        settings.permissions.deny.len(),
        1,
        "expected 1 deny rule, got {}",
        settings.permissions.deny.len()
    );
    assert_eq!(settings.permissions.deny[0].scope, Scope::Project);
    assert_eq!(settings.permissions.deny[0].rule, "Bash(rm -rf *)");

    // Check that scopes are stored raw
    assert!(!settings.scopes.user.is_null());
    assert!(!settings.scopes.project.is_null());
    assert!(!settings.scopes.local_.is_null());
}

#[test]
fn test_settings_merge_effective() {
    let claude_dir = TempDir::new().unwrap();
    let project_root = TempDir::new().unwrap();
    let paths = make_paths(&claude_dir, &project_root);

    write_fixture(
        claude_dir.path(),
        "settings.json",
        r#"{"theme": "dark", "nested": {"a": 1}}"#,
    );

    write_fixture(
        project_root.path(),
        ".claude/settings.json",
        r#"{"theme": "light", "nested": {"b": 2}}"#,
    );

    let settings = sources::settings::load(&paths);

    // "theme" should be overwritten by project scope
    assert_eq!(
        settings.effective.get("theme").and_then(|v| v.as_str()),
        Some("light")
    );
    // nested merge: both "a" and "b" should be present
    let nested = settings.effective.get("nested").expect("nested key");
    assert_eq!(nested.get("a").and_then(|v| v.as_i64()), Some(1));
    assert_eq!(nested.get("b").and_then(|v| v.as_i64()), Some(2));
}

#[test]
fn test_settings_missing_files_no_panic() {
    let claude_dir = TempDir::new().unwrap();
    let project_root = TempDir::new().unwrap();
    let paths = make_paths(&claude_dir, &project_root);

    // No settings files at all -- should not panic
    let settings = sources::settings::load(&paths);
    assert!(settings.permissions.allow.is_empty());
    assert!(settings.permissions.deny.is_empty());
    assert!(settings.scopes.user.is_null());
    assert!(settings.scopes.project.is_null());
    assert!(settings.scopes.local_.is_null());
}

// ── Hooks ────────────────────────────────────────────────────────────────

#[test]
fn test_hooks_three_scopes() {
    let claude_dir = TempDir::new().unwrap();
    let project_root = TempDir::new().unwrap();
    let paths = make_paths(&claude_dir, &project_root);

    // User-level hook in settings.json
    write_fixture(
        claude_dir.path(),
        "settings.json",
        r#"{
            "hooks": {
                "PostToolUse": [
                    {
                        "matcher": "Bash",
                        "hooks": [
                            {"command": "echo user-hook", "type": "command"}
                        ]
                    }
                ]
            }
        }"#,
    );

    // Project-level hook in settings.json
    write_fixture(
        project_root.path(),
        ".claude/settings.json",
        r#"{
            "hooks": {
                "PreToolUse": [
                    {
                        "matcher": "Edit",
                        "hooks": [
                            {"command": "echo project-hook", "type": "command"}
                        ]
                    }
                ]
            }
        }"#,
    );

    let hooks = sources::hooks::load(&paths);

    assert_eq!(hooks.len(), 2, "expected 2 hooks, got {}", hooks.len());

    let user_hook = hooks
        .iter()
        .find(|h| h.scope == Scope::User)
        .expect("user hook");
    assert_eq!(user_hook.event, "PostToolUse");
    assert_eq!(user_hook.matcher, "Bash");
    assert_eq!(user_hook.command, "echo user-hook");
    assert_eq!(user_hook.hook_type, "command");

    let proj_hook = hooks
        .iter()
        .find(|h| h.scope == Scope::Project)
        .expect("project hook");
    assert_eq!(proj_hook.event, "PreToolUse");
    assert_eq!(proj_hook.matcher, "Edit");
    assert_eq!(proj_hook.command, "echo project-hook");
}

#[test]
fn test_hooks_default_matcher_and_type() {
    let claude_dir = TempDir::new().unwrap();
    let project_root = TempDir::new().unwrap();
    let paths = make_paths(&claude_dir, &project_root);

    // Hook with no explicit matcher or type
    write_fixture(
        claude_dir.path(),
        "settings.json",
        r#"{
            "hooks": {
                "Notification": [
                    {
                        "hooks": [
                            {"command": "echo notify"}
                        ]
                    }
                ]
            }
        }"#,
    );

    let hooks = sources::hooks::load(&paths);
    assert_eq!(hooks.len(), 1);
    assert_eq!(hooks[0].matcher, "*");
    assert_eq!(hooks[0].hook_type, "command");
}

#[test]
fn test_hooks_local_scope() {
    let claude_dir = TempDir::new().unwrap();
    let project_root = TempDir::new().unwrap();
    let paths = make_paths(&claude_dir, &project_root);

    write_fixture(
        project_root.path(),
        ".claude/settings.local.json",
        r#"{
            "hooks": {
                "PostToolUse": [
                    {
                        "matcher": "*",
                        "hooks": [
                            {"command": "echo local-hook", "type": "command"}
                        ]
                    }
                ]
            }
        }"#,
    );

    let hooks = sources::hooks::load(&paths);
    assert_eq!(hooks.len(), 1);
    assert_eq!(hooks[0].scope, Scope::Local);
}

// ── CLAUDE.md ────────────────────────────────────────────────────────────

#[test]
fn test_claude_md_all_locations() {
    // Use a wrapper dir so claude_dir's parent is controlled (not shared /tmp)
    let wrapper = TempDir::new().unwrap();
    let project_root = TempDir::new().unwrap();
    let (paths, claude_dir) = make_nested_paths(&wrapper, &project_root);

    // 1) Project root CLAUDE.md
    write_fixture(project_root.path(), "CLAUDE.md", "# Project root CLAUDE");

    // 2) Project .claude/CLAUDE.md
    write_fixture(
        project_root.path(),
        ".claude/CLAUDE.md",
        "# Project .claude CLAUDE",
    );

    // 3) Project .claude/rules/test-rule.md
    write_fixture(
        project_root.path(),
        ".claude/rules/test-rule.md",
        "# Test rule",
    );

    // 4) User-level CLAUDE.md (~/.claude/CLAUDE.md)
    write_fixture(&claude_dir, "CLAUDE.md", "# User CLAUDE");

    // 5) User-level rules (~/.claude/rules/user-rule.md)
    write_fixture(&claude_dir, "rules/user-rule.md", "# User rule");

    let files = sources::claude_md::load(&paths);

    assert_eq!(
        files.len(),
        5,
        "expected 5 claude_md files, got {}",
        files.len()
    );

    // Verify scopes
    let project_files: Vec<_> = files.iter().filter(|f| f.scope == Scope::Project).collect();
    let user_files: Vec<_> = files.iter().filter(|f| f.scope == Scope::User).collect();
    assert_eq!(project_files.len(), 3, "expected 3 project-scope files");
    assert_eq!(user_files.len(), 2, "expected 2 user-scope files");

    // Verify file_type tags
    let claude_md_files: Vec<_> = files
        .iter()
        .filter(|f| f.file_type == "claude_md")
        .collect();
    let rule_files: Vec<_> = files.iter().filter(|f| f.file_type == "rule").collect();
    assert_eq!(claude_md_files.len(), 3, "expected 3 claude_md type files");
    assert_eq!(rule_files.len(), 2, "expected 2 rule type files");

    // Verify content is read
    let root_claude = files
        .iter()
        .find(|f| f.content.contains("Project root CLAUDE"))
        .expect("project root CLAUDE.md");
    assert!(root_claude.size > 0);
}

#[test]
fn test_claude_md_missing_files_still_works() {
    // Use nested paths so the parent of claude_dir is controlled
    let wrapper = TempDir::new().unwrap();
    let project_root = TempDir::new().unwrap();
    let (paths, _claude_dir) = make_nested_paths(&wrapper, &project_root);

    // Only create one file
    write_fixture(project_root.path(), "CLAUDE.md", "# Only this one");

    let files = sources::claude_md::load(&paths);
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].scope, Scope::Project);
    assert_eq!(files[0].file_type, "claude_md");
}

// ── Keybindings ──────────────────────────────────────────────────────────

#[test]
fn test_keybindings_flat_array() {
    let claude_dir = TempDir::new().unwrap();
    let project_root = TempDir::new().unwrap();
    let paths = make_paths(&claude_dir, &project_root);

    // Flat format: items WITHOUT a "context" key hit the else branch
    // (items with "context" are treated as nested context format)
    write_fixture(
        claude_dir.path(),
        "keybindings.json",
        r#"[
            {"key": "ctrl+s", "command": "submit"},
            {"key": "ctrl+k", "command": "clear"}
        ]"#,
    );

    let bindings = sources::keybindings::load(&paths);

    assert_eq!(
        bindings.len(),
        2,
        "expected 2 keybindings, got {}",
        bindings.len()
    );

    let submit = bindings.iter().find(|b| b.key == "ctrl+s").expect("ctrl+s");
    assert_eq!(submit.command, "submit");
    assert_eq!(submit.context, ""); // no context in flat format without the field

    let clear = bindings.iter().find(|b| b.key == "ctrl+k").expect("ctrl+k");
    assert_eq!(clear.command, "clear");
}

#[test]
fn test_keybindings_nested_context_format() {
    let claude_dir = TempDir::new().unwrap();
    let project_root = TempDir::new().unwrap();
    let paths = make_paths(&claude_dir, &project_root);

    write_fixture(
        claude_dir.path(),
        "keybindings.json",
        r#"[
            {
                "context": "Chat",
                "bindings": {
                    "ctrl+s": "submit",
                    "ctrl+k": "clear"
                }
            },
            {
                "context": "Editor",
                "bindings": {
                    "ctrl+z": "undo"
                }
            }
        ]"#,
    );

    let bindings = sources::keybindings::load(&paths);
    assert_eq!(
        bindings.len(),
        3,
        "expected 3 keybindings, got {}",
        bindings.len()
    );

    let chat_bindings: Vec<_> = bindings.iter().filter(|b| b.context == "Chat").collect();
    assert_eq!(chat_bindings.len(), 2);

    let editor_bindings: Vec<_> = bindings.iter().filter(|b| b.context == "Editor").collect();
    assert_eq!(editor_bindings.len(), 1);
    assert_eq!(editor_bindings[0].key, "ctrl+z");
    assert_eq!(editor_bindings[0].command, "undo");
}

#[test]
fn test_keybindings_missing_file() {
    let claude_dir = TempDir::new().unwrap();
    let project_root = TempDir::new().unwrap();
    let paths = make_paths(&claude_dir, &project_root);

    // No keybindings.json -- should return empty vec
    let bindings = sources::keybindings::load(&paths);
    assert!(bindings.is_empty());
}

// ── Memory ───────────────────────────────────────────────────────────────

#[test]
fn test_memory() {
    let claude_dir = TempDir::new().unwrap();
    let project_root = TempDir::new().unwrap();
    let paths = make_paths(&claude_dir, &project_root);

    // Memory path: claude_dir/projects/<encoded>/memory/
    let mem_dir = paths.memory_dir();
    fs::create_dir_all(&mem_dir).unwrap();

    // Write memory files with frontmatter
    let mem1 = mem_dir.join("architecture.md");
    fs::write(
        &mem1,
        "---\nname: Architecture\ndescription: System design notes\ntype: context\n---\nThe system uses a layered architecture.",
    )
    .unwrap();

    let mem2 = mem_dir.join("conventions.md");
    fs::write(
        &mem2,
        "---\nname: Conventions\ndescription: Code conventions\ntype: user\n---\nUse snake_case everywhere.",
    )
    .unwrap();

    let memory = sources::memory::load(&paths);

    assert_eq!(
        memory.files.len(),
        2,
        "expected 2 memory files, got {}",
        memory.files.len()
    );
    assert_eq!(memory.dir, mem_dir);

    // Files are sorted by name
    assert_eq!(memory.files[0].name, "Architecture");
    assert_eq!(memory.files[0].description, "System design notes");
    assert_eq!(memory.files[0].mem_type, "context");
    assert!(memory.files[0].body.contains("layered architecture"));
    assert_eq!(memory.files[0].filename, "architecture.md");

    assert_eq!(memory.files[1].name, "Conventions");
    assert_eq!(memory.files[1].mem_type, "user");
}

#[test]
fn test_memory_name_defaults_to_filename() {
    let claude_dir = TempDir::new().unwrap();
    let project_root = TempDir::new().unwrap();
    let paths = make_paths(&claude_dir, &project_root);

    let mem_dir = paths.memory_dir();
    fs::create_dir_all(&mem_dir).unwrap();

    // No "name" in frontmatter
    fs::write(
        mem_dir.join("my-notes.md"),
        "---\ndescription: just notes\n---\nContent here.",
    )
    .unwrap();

    let memory = sources::memory::load(&paths);
    assert_eq!(memory.files.len(), 1);
    assert_eq!(memory.files[0].name, "my-notes");
}

#[test]
fn test_memory_type_defaults_to_user() {
    let claude_dir = TempDir::new().unwrap();
    let project_root = TempDir::new().unwrap();
    let paths = make_paths(&claude_dir, &project_root);

    let mem_dir = paths.memory_dir();
    fs::create_dir_all(&mem_dir).unwrap();

    // No "type" in frontmatter
    fs::write(mem_dir.join("note.md"), "---\nname: Note\n---\nSomething.").unwrap();

    let memory = sources::memory::load(&paths);
    assert_eq!(memory.files.len(), 1);
    assert_eq!(memory.files[0].mem_type, "user");
}

#[test]
fn test_memory_ignores_non_md_files() {
    let claude_dir = TempDir::new().unwrap();
    let project_root = TempDir::new().unwrap();
    let paths = make_paths(&claude_dir, &project_root);

    let mem_dir = paths.memory_dir();
    fs::create_dir_all(&mem_dir).unwrap();

    fs::write(mem_dir.join("valid.md"), "---\nname: Valid\n---\nOk.").unwrap();
    fs::write(mem_dir.join("not-markdown.txt"), "This should be ignored").unwrap();
    fs::write(mem_dir.join("data.json"), "{}").unwrap();

    let memory = sources::memory::load(&paths);
    assert_eq!(memory.files.len(), 1);
    assert_eq!(memory.files[0].name, "Valid");
}

#[test]
fn test_memory_empty_dir() {
    let claude_dir = TempDir::new().unwrap();
    let project_root = TempDir::new().unwrap();
    let paths = make_paths(&claude_dir, &project_root);

    // Don't even create the memory dir
    let memory = sources::memory::load(&paths);
    assert!(memory.files.is_empty());
}

// ── Paths encoding ───────────────────────────────────────────────────────

#[test]
fn test_paths_encode_project_path() {
    assert_eq!(
        Paths::encode_project_path("/Users/foo/myproject"),
        "-Users-foo-myproject"
    );
    assert_eq!(
        Paths::encode_project_path("/home/user/code/app"),
        "-home-user-code-app"
    );
    // Underscores are replaced with hyphens
    assert_eq!(
        Paths::encode_project_path("/Users/idohaber/00_development/lazyclaude"),
        "-Users-idohaber-00-development-lazyclaude"
    );
    // Dots are replaced with hyphens
    assert_eq!(
        Paths::encode_project_path("/Users/idohaber/00_development/cc.nvim"),
        "-Users-idohaber-00-development-cc-nvim"
    );
}

#[test]
fn test_paths_project_config_dir() {
    let claude_dir = TempDir::new().unwrap();
    let project_root = TempDir::new().unwrap();
    let paths = make_paths(&claude_dir, &project_root);

    let encoded = Paths::encode_project_path(&project_root.path().to_string_lossy());
    let expected = claude_dir.path().join("projects").join(&encoded);
    assert_eq!(paths.project_config_dir(), expected);
}

#[test]
fn test_paths_method_consistency() {
    let claude_dir = TempDir::new().unwrap();
    let project_root = TempDir::new().unwrap();
    let paths = make_paths(&claude_dir, &project_root);

    // User skills dir
    assert_eq!(paths.user_skills_dir(), claude_dir.path().join("skills"));
    // Project skills dir
    assert_eq!(
        paths.project_skills_dir(),
        project_root.path().join(".claude").join("skills")
    );
    // User agents dir
    assert_eq!(paths.user_agents_dir(), claude_dir.path().join("agents"));
    // Project agents dir
    assert_eq!(
        paths.project_agents_dir(),
        project_root.path().join(".claude").join("agents")
    );
    // Keybindings
    assert_eq!(
        paths.keybindings_path(),
        claude_dir.path().join("keybindings.json")
    );
    // MCP paths (user-level is ~/.mcp.json, project-level is <project>/.mcp.json)
    assert_eq!(paths.mcp_path("user"), paths.home_dir.join(".mcp.json"));
    assert_eq!(
        paths.mcp_path("project"),
        project_root.path().join(".mcp.json")
    );
    // Settings paths
    assert_eq!(
        paths.settings_path("user"),
        claude_dir.path().join("settings.json")
    );
    assert_eq!(
        paths.settings_path("project"),
        project_root.path().join(".claude").join("settings.json")
    );
    assert_eq!(
        paths.settings_path("local"),
        project_root
            .path()
            .join(".claude")
            .join("settings.local.json")
    );
}

// ── Frontmatter parsing ─────────────────────────────────────────────────

#[test]
fn test_parse_frontmatter_basic() {
    let content = "---\nname: Test\ndescription: A test\n---\nBody text here.";
    let (fm, body) = sources::parse_frontmatter(content);
    assert_eq!(fm.get("name").map(|s| s.as_str()), Some("Test"));
    assert_eq!(fm.get("description").map(|s| s.as_str()), Some("A test"));
    assert_eq!(body, "Body text here.");
}

#[test]
fn test_parse_frontmatter_no_frontmatter() {
    let content = "Just plain body text.";
    let (fm, body) = sources::parse_frontmatter(content);
    assert!(fm.is_empty());
    assert_eq!(body, "Just plain body text.");
}

#[test]
fn test_parse_frontmatter_empty_body() {
    let content = "---\nname: EmptyBody\n---\n";
    let (fm, body) = sources::parse_frontmatter(content);
    assert_eq!(fm.get("name").map(|s| s.as_str()), Some("EmptyBody"));
    assert!(body.is_empty() || body.trim().is_empty());
}

// ── load_all integration ─────────────────────────────────────────────────

#[test]
fn test_load_all_integration() {
    let wrapper = TempDir::new().unwrap();
    let project_root = TempDir::new().unwrap();
    let (paths, claude_dir) = make_nested_paths(&wrapper, &project_root);

    // --- Skills ---
    write_fixture(
        &claude_dir,
        "skills/test-skill/SKILL.md",
        "---\nname: Test Skill\ndescription: test\n---\nBody.",
    );

    // --- Agents ---
    write_fixture(
        &claude_dir,
        "agents/test-agent.md",
        "---\nname: Test Agent\ndescription: test agent\nmodel: opus\n---\nAgent body.",
    );

    // --- MCP (user-level: ~/.mcp.json, lives in home dir) ---
    write_fixture(
        wrapper.path(),
        ".mcp.json",
        r#"{"mcpServers": {"test-server": {"command": "node", "args": ["test.js"]}}}"#,
    );

    // --- Settings ---
    write_fixture(
        &claude_dir,
        "settings.json",
        r#"{"permissions": {"allow": ["Bash(echo *)"]}}"#,
    );

    // --- Hooks (via project settings) ---
    write_fixture(
        project_root.path(),
        ".claude/settings.json",
        r#"{
            "hooks": {
                "PostToolUse": [
                    {
                        "matcher": "*",
                        "hooks": [
                            {"command": "echo done", "type": "command"}
                        ]
                    }
                ]
            }
        }"#,
    );

    // --- CLAUDE.md ---
    write_fixture(project_root.path(), "CLAUDE.md", "# Project instructions");

    // --- Keybindings (use nested context format) ---
    write_fixture(
        &claude_dir,
        "keybindings.json",
        r#"[{"context": "Chat", "bindings": {"ctrl+s": "submit"}}]"#,
    );

    // --- Memory ---
    let mem_dir = paths.memory_dir();
    fs::create_dir_all(&mem_dir).unwrap();
    fs::write(
        mem_dir.join("note.md"),
        "---\nname: Note\n---\nMemory content.",
    )
    .unwrap();

    // --- Load everything ---
    let data = sources::load_all(&paths);

    assert!(!data.skills.is_empty(), "skills should not be empty");
    assert!(!data.agents.is_empty(), "agents should not be empty");
    assert!(!data.mcp.user.is_empty(), "mcp.user should not be empty");
    assert!(
        !data.settings.permissions.allow.is_empty(),
        "settings.permissions.allow should not be empty"
    );
    assert!(!data.hooks.is_empty(), "hooks should not be empty");
    assert!(!data.claude_md.is_empty(), "claude_md should not be empty");
    assert!(
        !data.keybindings.is_empty(),
        "keybindings should not be empty"
    );
    assert!(
        !data.memory.files.is_empty(),
        "memory.files should not be empty"
    );
}

// ── Empty dirs (no panic) ────────────────────────────────────────────────

#[test]
fn test_empty_dirs() {
    // Use nested paths so the parent of claude_dir is a clean wrapper dir
    let wrapper = TempDir::new().unwrap();
    let project_root = TempDir::new().unwrap();
    let (paths, _claude_dir) = make_nested_paths(&wrapper, &project_root);

    // Call load_all on completely empty directories
    let data = sources::load_all(&paths);

    assert!(data.skills.is_empty());
    assert!(data.agents.is_empty());
    assert!(data.mcp.user.is_empty());
    assert!(data.mcp.project.is_empty());
    assert!(data.settings.permissions.allow.is_empty());
    assert!(data.settings.permissions.deny.is_empty());
    assert!(data.hooks.is_empty());
    assert!(data.claude_md.is_empty());
    assert!(data.keybindings.is_empty());
    assert!(data.memory.files.is_empty());
}

// ── MCP mutations ────────────────────────────────────────────────────────

#[test]
fn test_mcp_add_and_remove() {
    let wrapper = TempDir::new().unwrap();
    let project_root = TempDir::new().unwrap();
    let (paths, _claude_dir) = make_nested_paths(&wrapper, &project_root);

    // Start with empty -- add a server
    sources::mcp::add(
        &paths,
        "user",
        "new-server",
        "node",
        &["server.js".to_string()],
    )
    .expect("add server");

    let mcp = sources::mcp::load(&paths);
    assert_eq!(mcp.user.len(), 1);
    assert_eq!(mcp.user[0].name, "new-server");
    assert_eq!(mcp.user[0].command, "node");

    // Remove the server
    sources::mcp::remove(&paths, "user", "new-server").expect("remove server");
    let mcp = sources::mcp::load(&paths);
    assert!(mcp.user.is_empty());
}

#[test]
fn test_mcp_toggle() {
    let wrapper = TempDir::new().unwrap();
    let project_root = TempDir::new().unwrap();
    let (paths, _claude_dir) = make_nested_paths(&wrapper, &project_root);

    write_fixture(
        wrapper.path(),
        ".mcp.json",
        r#"{"mcpServers": {"s1": {"command": "node", "args": []}}}"#,
    );

    // Should not be disabled initially
    let mcp = sources::mcp::load(&paths);
    assert!(!mcp.user[0].disabled);

    // Toggle off
    sources::mcp::toggle(&paths, "user", "s1").expect("toggle off");
    let mcp = sources::mcp::load(&paths);
    assert!(mcp.user[0].disabled);

    // Toggle back on
    sources::mcp::toggle(&paths, "user", "s1").expect("toggle on");
    let mcp = sources::mcp::load(&paths);
    assert!(!mcp.user[0].disabled);
}

// ── Settings mutations ───────────────────────────────────────────────────

#[test]
fn test_settings_add_permission() {
    let claude_dir = TempDir::new().unwrap();
    let project_root = TempDir::new().unwrap();
    let paths = make_paths(&claude_dir, &project_root);

    sources::settings::add_permission(&paths, "user", "allow", "Bash(echo *)")
        .expect("add permission");

    let settings = sources::settings::load(&paths);
    assert_eq!(settings.permissions.allow.len(), 1);
    assert_eq!(settings.permissions.allow[0].rule, "Bash(echo *)");
    assert_eq!(settings.permissions.allow[0].scope, Scope::User);
}

#[test]
fn test_settings_remove_permission() {
    let claude_dir = TempDir::new().unwrap();
    let project_root = TempDir::new().unwrap();
    let paths = make_paths(&claude_dir, &project_root);

    write_fixture(
        claude_dir.path(),
        "settings.json",
        r#"{"permissions": {"allow": ["rule1", "rule2", "rule3"]}}"#,
    );

    // Remove index 1 ("rule2")
    sources::settings::remove_permission(&paths, "user", "allow", 1).expect("remove permission");

    let settings = sources::settings::load(&paths);
    assert_eq!(settings.permissions.allow.len(), 2);
    let rules: Vec<&str> = settings
        .permissions
        .allow
        .iter()
        .map(|r| r.rule.as_str())
        .collect();
    assert!(rules.contains(&"rule1"));
    assert!(rules.contains(&"rule3"));
    assert!(!rules.contains(&"rule2"));
}

// ── Edge cases ───────────────────────────────────────────────────────────

#[test]
fn test_skills_non_directory_entries_ignored() {
    let claude_dir = TempDir::new().unwrap();
    let project_root = TempDir::new().unwrap();
    let paths = make_paths(&claude_dir, &project_root);

    // Create a regular file (not a directory) in the skills dir
    write_fixture(claude_dir.path(), "skills/README.md", "# Not a skill");

    // Create a proper skill directory alongside it
    write_fixture(
        claude_dir.path(),
        "skills/real-skill/SKILL.md",
        "---\nname: Real\n---\nBody.",
    );

    let skills = sources::skills::load(&paths);
    assert_eq!(skills.len(), 1);
    assert_eq!(skills[0].name, "Real");
}

#[test]
fn test_agents_non_md_files_ignored() {
    let claude_dir = TempDir::new().unwrap();
    let project_root = TempDir::new().unwrap();
    let paths = make_paths(&claude_dir, &project_root);

    write_fixture(claude_dir.path(), "agents/stray-file.txt", "Not an agent");
    write_fixture(
        claude_dir.path(),
        "agents/real-agent.md",
        "---\nname: Real Agent\nmodel: opus\n---\nBody.",
    );

    let agents = sources::agents::load(&paths);
    assert_eq!(agents.len(), 1);
    assert_eq!(agents[0].name, "Real Agent");
}

#[test]
fn test_skills_directory_without_skill_md_ignored() {
    let claude_dir = TempDir::new().unwrap();
    let project_root = TempDir::new().unwrap();
    let paths = make_paths(&claude_dir, &project_root);

    // Directory exists but no SKILL.md inside
    fs::create_dir_all(claude_dir.path().join("skills/empty-skill")).unwrap();

    // Proper skill
    write_fixture(
        claude_dir.path(),
        "skills/valid-skill/SKILL.md",
        "---\nname: Valid\n---\nBody.",
    );

    let skills = sources::skills::load(&paths);
    assert_eq!(skills.len(), 1);
    assert_eq!(skills[0].name, "Valid");
}

#[test]
fn test_invalid_json_returns_empty() {
    let claude_dir = TempDir::new().unwrap();
    let project_root = TempDir::new().unwrap();
    let paths = make_paths(&claude_dir, &project_root);

    // Write malformed JSON to settings
    write_fixture(claude_dir.path(), "settings.json", "not valid json {{{");

    let settings = sources::settings::load(&paths);
    assert!(settings.permissions.allow.is_empty());
    assert!(settings.scopes.user.is_null());
}

#[test]
fn test_mcp_invalid_json_returns_empty() {
    let wrapper = TempDir::new().unwrap();
    let project_root = TempDir::new().unwrap();
    let (paths, _claude_dir) = make_nested_paths(&wrapper, &project_root);

    write_fixture(wrapper.path(), ".mcp.json", "broken json!!!");

    let mcp = sources::mcp::load(&paths);
    assert!(mcp.user.is_empty());
}

#[test]
fn test_multiple_hooks_per_event() {
    let claude_dir = TempDir::new().unwrap();
    let project_root = TempDir::new().unwrap();
    let paths = make_paths(&claude_dir, &project_root);

    write_fixture(
        claude_dir.path(),
        "settings.json",
        r#"{
            "hooks": {
                "PostToolUse": [
                    {
                        "matcher": "Bash",
                        "hooks": [
                            {"command": "echo first", "type": "command"},
                            {"command": "echo second", "type": "command"}
                        ]
                    },
                    {
                        "matcher": "Edit",
                        "hooks": [
                            {"command": "echo third", "type": "command"}
                        ]
                    }
                ]
            }
        }"#,
    );

    let hooks = sources::hooks::load(&paths);
    assert_eq!(hooks.len(), 3, "expected 3 hooks, got {}", hooks.len());
}

#[test]
fn test_skills_sorted_by_name() {
    let claude_dir = TempDir::new().unwrap();
    let project_root = TempDir::new().unwrap();
    let paths = make_paths(&claude_dir, &project_root);

    write_fixture(
        claude_dir.path(),
        "skills/zebra/SKILL.md",
        "---\nname: Zebra\n---\nZ.",
    );
    write_fixture(
        claude_dir.path(),
        "skills/alpha/SKILL.md",
        "---\nname: Alpha\n---\nA.",
    );
    write_fixture(
        claude_dir.path(),
        "skills/middle/SKILL.md",
        "---\nname: Middle\n---\nM.",
    );

    let skills = sources::skills::load(&paths);
    assert_eq!(skills.len(), 3);
    assert_eq!(skills[0].name, "Alpha");
    assert_eq!(skills[1].name, "Middle");
    assert_eq!(skills[2].name, "Zebra");
}

#[test]
fn test_agents_sorted_by_name() {
    let claude_dir = TempDir::new().unwrap();
    let project_root = TempDir::new().unwrap();
    let paths = make_paths(&claude_dir, &project_root);

    write_fixture(
        claude_dir.path(),
        "agents/zeta.md",
        "---\nname: Zeta\nmodel: opus\n---\nZ.",
    );
    write_fixture(
        claude_dir.path(),
        "agents/alpha.md",
        "---\nname: Alpha\nmodel: opus\n---\nA.",
    );

    let agents = sources::agents::load(&paths);
    assert_eq!(agents.len(), 2);
    assert_eq!(agents[0].name, "Alpha");
    assert_eq!(agents[1].name, "Zeta");
}

// ══════════════════════════════════════════════════════════════════════════
// NEW TESTS — Plugins module
// ══════════════════════════════════════════════════════════════════════════

#[test]
fn test_plugins_load_installed() {
    let claude_dir = TempDir::new().unwrap();
    let project_root = TempDir::new().unwrap();
    let paths = make_paths(&claude_dir, &project_root);

    write_fixture(
        claude_dir.path(),
        "plugins/installed_plugins.json",
        r#"{
            "plugins": {
                "my-plugin": [
                    {
                        "version": "1.2.3",
                        "scope": "user",
                        "installedAt": "2025-01-15T12:00:00Z"
                    }
                ]
            }
        }"#,
    );

    let data = sources::plugins::load(&paths);
    assert_eq!(data.installed.len(), 1);
    assert_eq!(data.installed[0].name, "my-plugin");
    assert_eq!(data.installed[0].version, "1.2.3");
    assert_eq!(data.installed[0].scope, Scope::User);
    // installedAt is truncated to first 10 chars
    assert_eq!(data.installed[0].installed_at, "2025-01-15");
}

#[test]
fn test_plugins_load_multiple_installations() {
    let claude_dir = TempDir::new().unwrap();
    let project_root = TempDir::new().unwrap();
    let paths = make_paths(&claude_dir, &project_root);

    write_fixture(
        claude_dir.path(),
        "plugins/installed_plugins.json",
        r#"{
            "plugins": {
                "multi-plugin": [
                    {"version": "1.0.0", "scope": "user", "installedAt": "2025-01-10"},
                    {"version": "2.0.0", "scope": "project", "installedAt": "2025-02-20"}
                ]
            }
        }"#,
    );

    let data = sources::plugins::load(&paths);
    assert_eq!(data.installed.len(), 2);
    assert_eq!(data.installed[0].version, "1.0.0");
    assert_eq!(data.installed[1].version, "2.0.0");
}

#[test]
fn test_plugins_load_blocked() {
    let claude_dir = TempDir::new().unwrap();
    let project_root = TempDir::new().unwrap();
    let paths = make_paths(&claude_dir, &project_root);

    write_fixture(
        claude_dir.path(),
        "plugins/blocklist.json",
        r#"{
            "plugins": [
                {"plugin": "bad-plugin", "reason": "security", "text": "Known vulnerability"}
            ]
        }"#,
    );

    let data = sources::plugins::load(&paths);
    assert_eq!(data.blocked.len(), 1);
    assert_eq!(data.blocked[0].name, "bad-plugin");
    assert_eq!(data.blocked[0].reason, "security");
    assert_eq!(data.blocked[0].text, "Known vulnerability");
}

#[test]
fn test_plugins_load_marketplaces() {
    let claude_dir = TempDir::new().unwrap();
    let project_root = TempDir::new().unwrap();
    let paths = make_paths(&claude_dir, &project_root);

    write_fixture(
        claude_dir.path(),
        "plugins/known_marketplaces.json",
        r#"{
            "official": {
                "source": {
                    "source": "github",
                    "repo": "anthropics/claude-plugins"
                }
            }
        }"#,
    );

    let data = sources::plugins::load(&paths);
    assert_eq!(data.marketplaces.len(), 1);
    assert_eq!(data.marketplaces[0].name, "official");
    assert_eq!(data.marketplaces[0].source_type, "github");
    assert_eq!(data.marketplaces[0].repo, "anthropics/claude-plugins");
}

#[test]
fn test_plugins_load_empty_returns_defaults() {
    let claude_dir = TempDir::new().unwrap();
    let project_root = TempDir::new().unwrap();
    let paths = make_paths(&claude_dir, &project_root);

    // No plugin files at all
    let data = sources::plugins::load(&paths);
    assert!(data.installed.is_empty());
    assert!(data.blocked.is_empty());
    assert!(data.marketplaces.is_empty());
}

#[test]
fn test_plugins_install_and_remove() {
    let claude_dir = TempDir::new().unwrap();
    let project_root = TempDir::new().unwrap();
    let paths = make_paths(&claude_dir, &project_root);

    // Install a plugin
    sources::plugins::install(&paths, "new-plugin", "1.0.0", "test-marketplace")
        .expect("install plugin");

    let data = sources::plugins::load(&paths);
    assert_eq!(data.installed.len(), 1);
    assert_eq!(data.installed[0].name, "new-plugin");
    assert_eq!(data.installed[0].version, "1.0.0");
    assert_eq!(data.installed[0].scope, Scope::User);

    // Remove the plugin
    sources::plugins::remove(&paths, "new-plugin").expect("remove plugin");
    let data = sources::plugins::load(&paths);
    assert!(data.installed.is_empty());
}

#[test]
fn test_plugins_install_replaces_existing_user_scope() {
    let claude_dir = TempDir::new().unwrap();
    let project_root = TempDir::new().unwrap();
    let paths = make_paths(&claude_dir, &project_root);

    // Install v1
    sources::plugins::install(&paths, "my-plugin", "1.0.0", "mp1").expect("install v1");

    // Install v2 — should replace, not duplicate
    sources::plugins::install(&paths, "my-plugin", "2.0.0", "mp1").expect("install v2");

    let data = sources::plugins::load(&paths);
    assert_eq!(
        data.installed.len(),
        1,
        "should have exactly 1 installation after replacement"
    );
    assert_eq!(data.installed[0].version, "2.0.0");
}

#[test]
fn test_plugins_install_into_empty_file() {
    let claude_dir = TempDir::new().unwrap();
    let project_root = TempDir::new().unwrap();
    let paths = make_paths(&claude_dir, &project_root);

    // No pre-existing installed_plugins.json
    sources::plugins::install(&paths, "fresh-plugin", "0.1.0", "market")
        .expect("install into empty");

    let data = sources::plugins::load(&paths);
    assert_eq!(data.installed.len(), 1);
    assert_eq!(data.installed[0].name, "fresh-plugin");
}

#[test]
fn test_plugins_unblock() {
    let claude_dir = TempDir::new().unwrap();
    let project_root = TempDir::new().unwrap();
    let paths = make_paths(&claude_dir, &project_root);

    write_fixture(
        claude_dir.path(),
        "plugins/blocklist.json",
        r#"{"plugins": [
            {"plugin": "keep-me", "reason": "ok", "text": ""},
            {"plugin": "unblock-me", "reason": "test", "text": ""}
        ]}"#,
    );

    sources::plugins::unblock(&paths, "unblock-me").expect("unblock");

    let data = sources::plugins::load(&paths);
    assert_eq!(data.blocked.len(), 1);
    assert_eq!(data.blocked[0].name, "keep-me");
}

// ── Plugins preview bodies ──────────────────────────────────────────────

#[test]
fn test_installed_plugin_preview_body() {
    let plugin = sources::plugins::InstalledPlugin {
        name: "test-plugin".to_string(),
        version: "1.0.0".to_string(),
        scope: sources::Scope::User,
        installed_at: "2025-03-01".to_string(),
    };

    let body = plugin.preview_body();
    assert!(body.contains("# test-plugin"));
    assert!(body.contains("Version: 1.0.0"));
    assert!(body.contains("Scope: user"));
    assert!(body.contains("Installed: 2025-03-01"));
}

#[test]
fn test_installed_plugin_preview_body_empty_fields() {
    let plugin = sources::plugins::InstalledPlugin {
        name: "minimal".to_string(),
        version: String::new(),
        scope: sources::Scope::User,
        installed_at: String::new(),
    };

    let body = plugin.preview_body();
    assert!(body.contains("# minimal"));
    assert!(body.contains("Scope: user"));
    // Empty version and installed_at should be omitted
    assert!(!body.contains("Version:"));
    assert!(!body.contains("Installed:"));
}

#[test]
fn test_blocked_plugin_preview_body() {
    let plugin = sources::plugins::BlockedPlugin {
        name: "bad-plugin".to_string(),
        reason: "security".to_string(),
        text: "CVE-2025-1234".to_string(),
    };

    let body = plugin.preview_body();
    assert!(body.contains("# bad-plugin"));
    assert!(body.contains("Status: Blocked"));
    assert!(body.contains("Reason: security"));
    assert!(body.contains("CVE-2025-1234"));
}

#[test]
fn test_blocked_plugin_preview_body_empty_reason() {
    let plugin = sources::plugins::BlockedPlugin {
        name: "blocked".to_string(),
        reason: String::new(),
        text: String::new(),
    };

    let body = plugin.preview_body();
    assert!(body.contains("# blocked"));
    assert!(body.contains("Status: Blocked"));
    assert!(!body.contains("Reason:"));
}

#[test]
fn test_marketplace_preview_body() {
    let mp = sources::plugins::Marketplace {
        name: "official".to_string(),
        source_type: "github".to_string(),
        repo: "anthropics/plugins".to_string(),
    };

    let body = mp.preview_body();
    assert!(body.contains("# official"));
    assert!(body.contains("Source: github"));
    assert!(body.contains("Repository: anthropics/plugins"));
}

// ══════════════════════════════════════════════════════════════════════════
// NEW TESTS — Plugin Registry (local marketplace search)
// ══════════════════════════════════════════════════════════════════════════

#[test]
fn test_plugin_registry_search_local_empty_dir() {
    let tmp = TempDir::new().unwrap();
    let result = sources::plugin_registry::search_local(tmp.path(), "");
    assert!(result.is_ok());
    assert!(result.unwrap().is_empty());
}

#[test]
fn test_plugin_registry_search_local_no_marketplaces_dir() {
    let tmp = TempDir::new().unwrap();
    // plugins dir exists but no marketplaces subdir
    let result = sources::plugin_registry::search_local(tmp.path(), "");
    assert!(result.is_ok());
    assert!(result.unwrap().is_empty());
}

#[test]
fn test_plugin_registry_search_local_finds_plugin() {
    let tmp = TempDir::new().unwrap();
    let base = tmp.path();

    // Create marketplace/plugin structure
    let plugin_dir = base.join("marketplaces/test-mp/plugins/my-plugin");
    let claude_plugin_dir = plugin_dir.join(".claude-plugin");
    fs::create_dir_all(&claude_plugin_dir).unwrap();
    fs::write(
        claude_plugin_dir.join("plugin.json"),
        r#"{"name": "my-plugin", "description": "A test plugin", "version": "1.0.0", "author": {"name": "Test Author"}}"#,
    ).unwrap();

    let results = sources::plugin_registry::search_local(base, "").unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "my-plugin");
    assert_eq!(results[0].description, "A test plugin");
    assert_eq!(results[0].version, "1.0.0");
    assert_eq!(results[0].author, "Test Author");
    assert_eq!(results[0].marketplace, "test-mp");
}

#[test]
fn test_plugin_registry_search_local_filters_by_query() {
    let tmp = TempDir::new().unwrap();
    let base = tmp.path();

    // Plugin 1
    let p1 = base.join("marketplaces/mp/plugins/alpha-plugin/.claude-plugin");
    fs::create_dir_all(&p1).unwrap();
    fs::write(
        p1.join("plugin.json"),
        r#"{"name": "alpha-plugin", "description": "First"}"#,
    )
    .unwrap();

    // Plugin 2
    let p2 = base.join("marketplaces/mp/plugins/beta-tool/.claude-plugin");
    fs::create_dir_all(&p2).unwrap();
    fs::write(
        p2.join("plugin.json"),
        r#"{"name": "beta-tool", "description": "Second"}"#,
    )
    .unwrap();

    // Search for "alpha"
    let results = sources::plugin_registry::search_local(base, "alpha").unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "alpha-plugin");

    // Search for "second" (matches description)
    let results = sources::plugin_registry::search_local(base, "second").unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "beta-tool");

    // Empty query returns all
    let results = sources::plugin_registry::search_local(base, "").unwrap();
    assert_eq!(results.len(), 2);
}

#[test]
fn test_plugin_registry_component_detection() {
    let tmp = TempDir::new().unwrap();
    let base = tmp.path();

    let plugin_dir = base.join("marketplaces/mp/plugins/full-plugin");
    let claude_plugin_dir = plugin_dir.join(".claude-plugin");
    fs::create_dir_all(&claude_plugin_dir).unwrap();
    fs::write(
        claude_plugin_dir.join("plugin.json"),
        r#"{"name": "full-plugin", "description": "Has everything"}"#,
    )
    .unwrap();

    // Create component subdirectories
    fs::create_dir_all(plugin_dir.join("agents")).unwrap();
    fs::create_dir_all(plugin_dir.join("skills")).unwrap();
    fs::create_dir_all(plugin_dir.join("hooks")).unwrap();
    fs::create_dir_all(plugin_dir.join("commands")).unwrap();
    fs::create_dir_all(plugin_dir.join("mcp")).unwrap();

    let results = sources::plugin_registry::search_local(base, "").unwrap();
    assert_eq!(results.len(), 1);
    assert!(results[0].has_agents);
    assert!(results[0].has_skills);
    assert!(results[0].has_hooks);
    assert!(results[0].has_commands);
    assert!(results[0].has_mcp);
}

#[test]
fn test_plugin_registry_mcp_servers_dir() {
    let tmp = TempDir::new().unwrap();
    let base = tmp.path();

    let plugin_dir = base.join("marketplaces/mp/plugins/mcp-plugin");
    let claude_plugin_dir = plugin_dir.join(".claude-plugin");
    fs::create_dir_all(&claude_plugin_dir).unwrap();
    fs::write(
        claude_plugin_dir.join("plugin.json"),
        r#"{"name": "mcp-plugin", "description": "MCP via mcp-servers dir"}"#,
    )
    .unwrap();

    // "mcp-servers" dir also triggers has_mcp
    fs::create_dir_all(plugin_dir.join("mcp-servers")).unwrap();

    let results = sources::plugin_registry::search_local(base, "").unwrap();
    assert!(results[0].has_mcp);
}

#[test]
fn test_plugin_registry_component_tags() {
    let entry = sources::plugin_registry::PluginEntry {
        name: "test".to_string(),
        description: String::new(),
        version: String::new(),
        author: String::new(),
        marketplace: String::new(),
        readme: String::new(),
        has_agents: true,
        has_skills: false,
        has_hooks: true,
        has_commands: false,
        has_mcp: true,
    };

    let tags = entry.component_tags();
    assert_eq!(tags, vec!["Agents", "Hooks", "MCP Servers"]);
}

#[test]
fn test_plugin_registry_component_summary() {
    let entry = sources::plugin_registry::PluginEntry {
        name: "test".to_string(),
        description: String::new(),
        version: String::new(),
        author: String::new(),
        marketplace: String::new(),
        readme: String::new(),
        has_agents: true,
        has_skills: true,
        has_hooks: false,
        has_commands: false,
        has_mcp: false,
    };

    assert_eq!(entry.component_summary(), "agents skills");
}

#[test]
fn test_plugin_registry_component_summary_empty() {
    let entry = sources::plugin_registry::PluginEntry {
        name: "bare".to_string(),
        description: String::new(),
        version: String::new(),
        author: String::new(),
        marketplace: String::new(),
        readme: String::new(),
        has_agents: false,
        has_skills: false,
        has_hooks: false,
        has_commands: false,
        has_mcp: false,
    };

    assert_eq!(entry.component_summary(), "");
    assert!(entry.component_tags().is_empty());
}

#[test]
fn test_plugin_registry_preview_body() {
    let entry = sources::plugin_registry::PluginEntry {
        name: "cool-plugin".to_string(),
        description: "A cool plugin".to_string(),
        version: "2.0.0".to_string(),
        author: "Dev".to_string(),
        marketplace: "official".to_string(),
        readme: "# README\nThis is a plugin.".to_string(),
        has_agents: true,
        has_skills: false,
        has_hooks: false,
        has_commands: false,
        has_mcp: false,
    };

    let body = entry.preview_body();
    assert!(body.contains("# cool-plugin"));
    assert!(body.contains("A cool plugin"));
    assert!(body.contains("Version: 2.0.0"));
    assert!(body.contains("Author: Dev"));
    assert!(body.contains("Marketplace: official"));
    assert!(body.contains("## Components"));
    assert!(body.contains("- Agents"));
    assert!(body.contains("## Install"));
    assert!(body.contains("cool-plugin@official"));
    assert!(body.contains("# README"));
}

#[test]
fn test_plugin_registry_results_sorted_by_name() {
    let tmp = TempDir::new().unwrap();
    let base = tmp.path();

    for name in &["zebra-plugin", "alpha-plugin", "mid-plugin"] {
        let p = base.join(format!("marketplaces/mp/plugins/{name}/.claude-plugin"));
        fs::create_dir_all(&p).unwrap();
        fs::write(
            p.join("plugin.json"),
            format!(r#"{{"name": "{name}", "description": ""}}"#),
        )
        .unwrap();
    }

    let results = sources::plugin_registry::search_local(base, "").unwrap();
    assert_eq!(results.len(), 3);
    assert_eq!(results[0].name, "alpha-plugin");
    assert_eq!(results[1].name, "mid-plugin");
    assert_eq!(results[2].name, "zebra-plugin");
}

// ══════════════════════════════════════════════════════════════════════════
// NEW TESTS — Stats module
// ══════════════════════════════════════════════════════════════════════════

#[test]
fn test_stats_load_full() {
    let claude_dir = TempDir::new().unwrap();
    let project_root = TempDir::new().unwrap();
    let paths = make_paths(&claude_dir, &project_root);

    write_fixture(
        claude_dir.path(),
        "stats-cache.json",
        r#"{
            "totalSessions": 42,
            "totalMessages": 1234,
            "firstSessionDate": "2025-01-01T00:00:00Z",
            "lastComputedDate": "2025-06-15",
            "dailyActivity": [
                {"date": "2025-06-15", "messageCount": 10, "sessionCount": 2, "toolCallCount": 5}
            ],
            "modelUsage": {
                "opus": {"inputTokens": 1000, "outputTokens": 2000, "cacheReadInputTokens": 500, "cacheCreationInputTokens": 100},
                "sonnet": {"inputTokens": 500, "outputTokens": 800, "cacheReadInputTokens": 0, "cacheCreationInputTokens": 0}
            },
            "longestSession": {
                "sessionId": "abc123",
                "duration": 3600000,
                "messageCount": 50
            },
            "hourCounts": {"0": 5, "12": 20, "23": 3}
        }"#,
    );

    let stats = sources::stats::load(&paths);
    assert_eq!(stats.total_sessions, 42);
    assert_eq!(stats.total_messages, 1234);
    assert_eq!(stats.first_session_date, "2025-01-01");
    assert_eq!(stats.last_computed_date, "2025-06-15");

    // Daily activity
    assert_eq!(stats.daily_activity.len(), 1);
    assert_eq!(stats.daily_activity[0].date, "2025-06-15");
    assert_eq!(stats.daily_activity[0].messages, 10);
    assert_eq!(stats.daily_activity[0].sessions, 2);
    assert_eq!(stats.daily_activity[0].tool_calls, 5);

    // Model usage (sorted by total_tokens descending)
    assert_eq!(stats.model_usage.len(), 2);
    assert_eq!(stats.model_usage[0].model, "opus");
    assert_eq!(stats.model_usage[0].total_tokens, 1000 + 2000 + 500 + 100);
    assert_eq!(stats.model_usage[1].model, "sonnet");
    assert_eq!(stats.model_usage[1].total_tokens, 500 + 800);

    // Longest session
    let ls = stats.longest_session.as_ref().unwrap();
    assert_eq!(ls.session_id, "abc123");
    assert_eq!(ls.duration_ms, 3600000);
    assert_eq!(ls.message_count, 50);

    // Hour counts
    assert_eq!(stats.hour_counts[0], 5);
    assert_eq!(stats.hour_counts[12], 20);
    assert_eq!(stats.hour_counts[23], 3);
    assert_eq!(stats.hour_counts[1], 0); // unset hour
}

#[test]
fn test_stats_load_empty_file() {
    let claude_dir = TempDir::new().unwrap();
    let project_root = TempDir::new().unwrap();
    let paths = make_paths(&claude_dir, &project_root);

    // No stats-cache.json
    let stats = sources::stats::load(&paths);
    assert_eq!(stats.total_sessions, 0);
    assert_eq!(stats.total_messages, 0);
    assert!(stats.first_session_date.is_empty());
    assert!(stats.daily_activity.is_empty());
    assert!(stats.model_usage.is_empty());
    assert!(stats.longest_session.is_none());
    assert_eq!(stats.hour_counts, [0u64; 24]);
}

#[test]
fn test_stats_load_invalid_json() {
    let claude_dir = TempDir::new().unwrap();
    let project_root = TempDir::new().unwrap();
    let paths = make_paths(&claude_dir, &project_root);

    write_fixture(claude_dir.path(), "stats-cache.json", "not json at all");
    let stats = sources::stats::load(&paths);
    assert_eq!(stats.total_sessions, 0);
}

#[test]
fn test_stats_hour_counts_ignores_invalid_keys() {
    let claude_dir = TempDir::new().unwrap();
    let project_root = TempDir::new().unwrap();
    let paths = make_paths(&claude_dir, &project_root);

    write_fixture(
        claude_dir.path(),
        "stats-cache.json",
        r#"{"hourCounts": {"0": 10, "24": 999, "abc": 5, "23": 7}}"#,
    );

    let stats = sources::stats::load(&paths);
    assert_eq!(stats.hour_counts[0], 10);
    assert_eq!(stats.hour_counts[23], 7);
    // Key "24" is out of bounds (h < 24 check), should be ignored
    // Key "abc" can't be parsed, should be ignored
}

#[test]
fn test_stats_first_session_date_short_string() {
    let claude_dir = TempDir::new().unwrap();
    let project_root = TempDir::new().unwrap();
    let paths = make_paths(&claude_dir, &project_root);

    write_fixture(
        claude_dir.path(),
        "stats-cache.json",
        r#"{"firstSessionDate": "short"}"#,
    );

    let stats = sources::stats::load(&paths);
    // If string is shorter than 10 chars, use as-is
    assert_eq!(stats.first_session_date, "short");
}

#[test]
fn test_stats_no_longest_session() {
    let claude_dir = TempDir::new().unwrap();
    let project_root = TempDir::new().unwrap();
    let paths = make_paths(&claude_dir, &project_root);

    write_fixture(
        claude_dir.path(),
        "stats-cache.json",
        r#"{"totalSessions": 5}"#,
    );

    let stats = sources::stats::load(&paths);
    assert_eq!(stats.total_sessions, 5);
    assert!(stats.longest_session.is_none());
}

// ══════════════════════════════════════════════════════════════════════════
// NEW TESTS — Todos module
// ══════════════════════════════════════════════════════════════════════════

#[test]
fn test_todos_load_basic() {
    let claude_dir = TempDir::new().unwrap();
    let project_root = TempDir::new().unwrap();
    let paths = make_paths(&claude_dir, &project_root);

    write_fixture(
        claude_dir.path(),
        "todos/session1.json",
        r#"[
            {"id": "t1", "content": "Fix the bug", "status": "pending"},
            {"id": "t2", "content": "Write tests", "status": "done"}
        ]"#,
    );

    let todos = sources::todos::load(&paths);
    assert_eq!(todos.len(), 2);
    assert_eq!(todos[0].id, "t1");
    assert_eq!(todos[0].content, "Fix the bug");
    assert_eq!(todos[0].status, "pending");
    assert_eq!(todos[0].session_file, "session1");
    assert_eq!(todos[1].status, "done");
}

#[test]
fn test_todos_load_empty_dir() {
    let claude_dir = TempDir::new().unwrap();
    let project_root = TempDir::new().unwrap();
    let paths = make_paths(&claude_dir, &project_root);

    // No todos directory
    let todos = sources::todos::load(&paths);
    assert!(todos.is_empty());
}

#[test]
fn test_todos_fallback_content_fields() {
    let claude_dir = TempDir::new().unwrap();
    let project_root = TempDir::new().unwrap();
    let paths = make_paths(&claude_dir, &project_root);

    // Test that "subject", "description", and "text" fallback fields work
    write_fixture(
        claude_dir.path(),
        "todos/session-fallback.json",
        r#"[
            {"id": "a", "subject": "Subject text"},
            {"id": "b", "description": "Description text"},
            {"id": "c", "text": "Text text"}
        ]"#,
    );

    let todos = sources::todos::load(&paths);
    assert_eq!(todos.len(), 3);
    assert_eq!(todos[0].content, "Subject text");
    assert_eq!(todos[1].content, "Description text");
    assert_eq!(todos[2].content, "Text text");
}

#[test]
fn test_todos_default_status_is_pending() {
    let claude_dir = TempDir::new().unwrap();
    let project_root = TempDir::new().unwrap();
    let paths = make_paths(&claude_dir, &project_root);

    write_fixture(
        claude_dir.path(),
        "todos/session.json",
        r#"[{"id": "x", "content": "No status field"}]"#,
    );

    let todos = sources::todos::load(&paths);
    assert_eq!(todos.len(), 1);
    assert_eq!(todos[0].status, "pending");
}

#[test]
fn test_todos_auto_id_when_missing() {
    let claude_dir = TempDir::new().unwrap();
    let project_root = TempDir::new().unwrap();
    let paths = make_paths(&claude_dir, &project_root);

    write_fixture(
        claude_dir.path(),
        "todos/session.json",
        r#"[{"content": "No id"}, {"content": "Also no id"}]"#,
    );

    let todos = sources::todos::load(&paths);
    assert_eq!(todos.len(), 2);
    // Auto-generated IDs should be numeric strings based on position
    assert_eq!(todos[0].id, "0");
    assert_eq!(todos[1].id, "1");
}

#[test]
fn test_todos_skips_empty_entries() {
    let claude_dir = TempDir::new().unwrap();
    let project_root = TempDir::new().unwrap();
    let paths = make_paths(&claude_dir, &project_root);

    // Entry with no id and no content/text fields => both empty => skipped
    write_fixture(
        claude_dir.path(),
        "todos/session.json",
        r#"[{"status": "pending"}, {"id": "valid", "content": "Real todo"}]"#,
    );

    let todos = sources::todos::load(&paths);
    assert_eq!(todos.len(), 1);
    assert_eq!(todos[0].id, "valid");
}

#[test]
fn test_todos_ignores_non_json_files() {
    let claude_dir = TempDir::new().unwrap();
    let project_root = TempDir::new().unwrap();
    let paths = make_paths(&claude_dir, &project_root);

    let todo_dir = claude_dir.path().join("todos");
    fs::create_dir_all(&todo_dir).unwrap();
    fs::write(todo_dir.join("notes.txt"), "not json").unwrap();
    fs::write(
        todo_dir.join("valid.json"),
        r#"[{"id": "1", "content": "ok"}]"#,
    )
    .unwrap();

    let todos = sources::todos::load(&paths);
    assert_eq!(todos.len(), 1);
    assert_eq!(todos[0].content, "ok");
}

#[test]
fn test_todos_invalid_json_file_skipped() {
    let claude_dir = TempDir::new().unwrap();
    let project_root = TempDir::new().unwrap();
    let paths = make_paths(&claude_dir, &project_root);

    let todo_dir = claude_dir.path().join("todos");
    fs::create_dir_all(&todo_dir).unwrap();
    fs::write(todo_dir.join("bad.json"), "not valid json!!!").unwrap();
    fs::write(
        todo_dir.join("good.json"),
        r#"[{"id": "1", "content": "ok"}]"#,
    )
    .unwrap();

    let todos = sources::todos::load(&paths);
    assert_eq!(todos.len(), 1);
}

#[test]
fn test_todos_multiple_session_files() {
    let claude_dir = TempDir::new().unwrap();
    let project_root = TempDir::new().unwrap();
    let paths = make_paths(&claude_dir, &project_root);

    let todo_dir = claude_dir.path().join("todos");
    fs::create_dir_all(&todo_dir).unwrap();
    fs::write(
        todo_dir.join("session-a.json"),
        r#"[{"id": "1", "content": "from A"}]"#,
    )
    .unwrap();
    fs::write(
        todo_dir.join("session-b.json"),
        r#"[{"id": "2", "content": "from B"}]"#,
    )
    .unwrap();

    let todos = sources::todos::load(&paths);
    assert_eq!(todos.len(), 2);

    // Each todo should reference its session file
    let session_files: Vec<&str> = todos.iter().map(|t| t.session_file.as_str()).collect();
    assert!(session_files.contains(&"session-a"));
    assert!(session_files.contains(&"session-b"));
}

// ══════════════════════════════════════════════════════════════════════════
// NEW TESTS — MCP Registry (RegistryEntry methods, no network)
// ══════════════════════════════════════════════════════════════════════════

#[test]
fn test_mcp_registry_popularity_dots() {
    let make_entry = |pop: f64| sources::mcp_registry::RegistryEntry {
        name: String::new(),
        description: String::new(),
        version: String::new(),
        install_command: String::new(),
        install_args: Vec::new(),
        registry: String::new(),
        author: String::new(),
        date: String::new(),
        homepage: String::new(),
        repository: String::new(),
        npm_url: String::new(),
        keywords: Vec::new(),
        score_quality: 0.0,
        score_popularity: pop,
        score_maintenance: 0.0,
    };

    assert_eq!(
        make_entry(0.0).popularity_dots(),
        "\u{25CB}\u{25CB}\u{25CB}\u{25CB}\u{25CB}"
    );
    assert_eq!(
        make_entry(0.15).popularity_dots(),
        "\u{25CF}\u{25CB}\u{25CB}\u{25CB}\u{25CB}"
    );
    assert_eq!(
        make_entry(0.5).popularity_dots(),
        "\u{25CF}\u{25CF}\u{25CF}\u{25CB}\u{25CB}"
    );
    assert_eq!(
        make_entry(1.0).popularity_dots(),
        "\u{25CF}\u{25CF}\u{25CF}\u{25CF}\u{25CF}"
    );
}

#[test]
fn test_mcp_registry_preview_body_content() {
    let entry = sources::mcp_registry::RegistryEntry {
        name: "mcp-server-test".to_string(),
        description: "A test MCP server".to_string(),
        version: "3.0.0".to_string(),
        install_command: "npx".to_string(),
        install_args: vec!["-y".to_string(), "mcp-server-test".to_string()],
        registry: "npm".to_string(),
        author: "TestDev".to_string(),
        date: "2025-06-15T12:00:00".to_string(),
        homepage: "https://example.com".to_string(),
        repository: "https://github.com/test/repo".to_string(),
        npm_url: "https://npmjs.com/package/mcp-server-test".to_string(),
        keywords: vec!["mcp".to_string(), "test".to_string()],
        score_quality: 0.8,
        score_popularity: 0.5,
        score_maintenance: 0.9,
    };

    let body = entry.preview_body();
    assert!(body.contains("# mcp-server-test"));
    assert!(body.contains("A test MCP server"));
    assert!(body.contains("Version: 3.0.0"));
    assert!(body.contains("Author: TestDev"));
    assert!(body.contains("Published: 2025-06-15"));
    assert!(body.contains("Registry: npm"));
    assert!(body.contains("## Quality"));
    assert!(body.contains("## Popularity"));
    assert!(body.contains("## Maintenance"));
    assert!(body.contains("npm: https://npmjs.com/package/mcp-server-test"));
    assert!(body.contains("Homepage: https://example.com"));
    assert!(body.contains("Repository: https://github.com/test/repo"));
    assert!(body.contains("Keywords: mcp, test"));
    assert!(body.contains("## Install"));
    assert!(body.contains("npx -y mcp-server-test"));
}

#[test]
fn test_mcp_registry_preview_body_minimal() {
    let entry = sources::mcp_registry::RegistryEntry {
        name: "minimal".to_string(),
        description: String::new(),
        version: "1.0.0".to_string(),
        install_command: "npx".to_string(),
        install_args: vec!["-y".to_string(), "minimal".to_string()],
        registry: "npm".to_string(),
        author: String::new(),
        date: String::new(),
        homepage: String::new(),
        repository: String::new(),
        npm_url: String::new(),
        keywords: Vec::new(),
        score_quality: 0.0,
        score_popularity: 0.0,
        score_maintenance: 0.0,
    };

    let body = entry.preview_body();
    assert!(body.contains("# minimal"));
    // No author, date, links, or keywords sections
    assert!(!body.contains("Author:"));
    assert!(!body.contains("Published:"));
    assert!(!body.contains("npm:"));
    assert!(!body.contains("Keywords:"));
}

// ══════════════════════════════════════════════════════════════════════════
// NEW TESTS — Skills Registry (SkillEntry methods, no network)
// ══════════════════════════════════════════════════════════════════════════

#[test]
fn test_skills_registry_preview_body_not_installed() {
    let entry = sources::skills_registry::SkillEntry {
        name: "code-review".to_string(),
        description: "Automated code review\nwith AI feedback".to_string(),
        dir_name: "code-review".to_string(),
    };

    let body = entry.preview_body(false);
    assert!(body.contains("# code-review"));
    assert!(body.contains("Automated code review"));
    assert!(body.contains("with AI feedback"));
    assert!(body.contains("Source: github.com/anthropics/skills"));
    assert!(body.contains("Directory: skills/code-review"));
    assert!(body.contains("## Install"));
    assert!(body.contains("~/.claude/skills/code-review/"));
    assert!(!body.contains("Already installed"));
}

#[test]
fn test_skills_registry_preview_body_installed() {
    let entry = sources::skills_registry::SkillEntry {
        name: "my-skill".to_string(),
        description: "A skill".to_string(),
        dir_name: "my-skill".to_string(),
    };

    let body = entry.preview_body(true);
    assert!(body.contains("Status: Already installed"));
}

#[test]
fn test_skills_registry_preview_body_empty_description() {
    let entry = sources::skills_registry::SkillEntry {
        name: "no-desc".to_string(),
        description: String::new(),
        dir_name: "no-desc".to_string(),
    };

    let body = entry.preview_body(false);
    assert!(body.contains("# no-desc"));
    assert!(body.contains("---"));
}

// ══════════════════════════════════════════════════════════════════════════
// NEW TESTS — McpServer preview_body
// ══════════════════════════════════════════════════════════════════════════

#[test]
fn test_mcp_server_preview_body_basic() {
    let server = sources::McpServer {
        name: "my-server".to_string(),
        command: "node".to_string(),
        args: vec![
            "index.js".to_string(),
            "--port".to_string(),
            "3000".to_string(),
        ],
        env: std::collections::HashMap::new(),
        disabled: false,
    };

    let body = server.preview_body("user");
    assert!(body.contains("# my-server"));
    assert!(body.contains("Status: Enabled"));
    assert!(body.contains("Scope: user"));
    assert!(body.contains("Command: node"));
    assert!(body.contains("Args: index.js --port 3000"));
}

#[test]
fn test_mcp_server_preview_body_disabled() {
    let server = sources::McpServer {
        name: "off-server".to_string(),
        command: "python".to_string(),
        args: Vec::new(),
        env: std::collections::HashMap::new(),
        disabled: true,
    };

    let body = server.preview_body("project");
    assert!(body.contains("Status: Disabled"));
    assert!(body.contains("Scope: project"));
    // No "Args:" line when args is empty
    assert!(!body.contains("Args:"));
}

#[test]
fn test_mcp_server_preview_body_truncates_long_env_values() {
    let mut env = std::collections::HashMap::new();
    env.insert("SHORT".to_string(), "abc".to_string());
    env.insert(
        "LONG_KEY".to_string(),
        "this_is_a_very_long_value_that_exceeds_20_chars".to_string(),
    );

    let server = sources::McpServer {
        name: "env-server".to_string(),
        command: "node".to_string(),
        args: Vec::new(),
        env,
        disabled: false,
    };

    let body = server.preview_body("user");
    assert!(body.contains("## Environment"));
    assert!(body.contains("SHORT: abc"));
    // Long value should be truncated to 17 chars + "..."
    assert!(body.contains("LONG_KEY: this_is_a_very_lo..."));
}

// ══════════════════════════════════════════════════════════════════════════
// NEW TESTS — Frontmatter edge cases
// ══════════════════════════════════════════════════════════════════════════

#[test]
fn test_parse_frontmatter_colon_in_value() {
    // Values with colons (e.g. URLs) should preserve everything after first colon
    let content = "---\nurl: https://example.com:8080\n---\nBody.";
    let (fm, body) = sources::parse_frontmatter(content);
    assert_eq!(
        fm.get("url").map(|s| s.as_str()),
        Some("https://example.com:8080")
    );
    assert_eq!(body, "Body.");
}

#[test]
fn test_parse_frontmatter_empty_string() {
    let (fm, body) = sources::parse_frontmatter("");
    assert!(fm.is_empty());
    assert_eq!(body, "");
}

#[test]
fn test_parse_frontmatter_only_dashes_no_close() {
    // Opening --- but no closing --- => treated as no frontmatter
    let content = "---\nname: Test\nno closing dashes";
    let (fm, body) = sources::parse_frontmatter(content);
    assert!(fm.is_empty());
    assert_eq!(body, content);
}

#[test]
fn test_parse_frontmatter_whitespace_trimming() {
    let content = "---\n  name  :  Spaced Out  \n---\nBody.";
    let (fm, body) = sources::parse_frontmatter(content);
    assert_eq!(fm.get("name").map(|s| s.as_str()), Some("Spaced Out"));
    assert_eq!(body, "Body.");
}

#[test]
fn test_parse_frontmatter_empty_value() {
    let content = "---\nname:\n---\nBody.";
    let (fm, body) = sources::parse_frontmatter(content);
    assert_eq!(fm.get("name").map(|s| s.as_str()), Some(""));
    assert_eq!(body, "Body.");
}

#[test]
fn test_parse_frontmatter_multiline_body() {
    let content = "---\nname: Test\n---\nLine 1\nLine 2\nLine 3";
    let (fm, body) = sources::parse_frontmatter(content);
    assert_eq!(fm.get("name").map(|s| s.as_str()), Some("Test"));
    assert_eq!(body, "Line 1\nLine 2\nLine 3");
}

// ══════════════════════════════════════════════════════════════════════════
// NEW TESTS — read_json / write_json helpers
// ══════════════════════════════════════════════════════════════════════════

#[test]
fn test_read_json_missing_file() {
    let val = sources::read_json(&std::path::PathBuf::from("/nonexistent/path.json"));
    assert!(val.is_null());
}

#[test]
fn test_read_json_invalid_content() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("bad.json");
    fs::write(&path, "{{{{not json}}}}").unwrap();
    let val = sources::read_json(&path);
    assert!(val.is_null());
}

#[test]
fn test_write_json_creates_parent_dirs() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("a/b/c/test.json");
    let val = serde_json::json!({"key": "value"});
    sources::write_json(&path, &val).expect("write_json should create dirs");

    let read_back = sources::read_json(&path);
    assert_eq!(read_back["key"].as_str(), Some("value"));
}

#[test]
fn test_write_and_read_json_roundtrip() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("roundtrip.json");
    let original = serde_json::json!({
        "name": "test",
        "count": 42,
        "nested": {"a": [1, 2, 3]}
    });
    sources::write_json(&path, &original).unwrap();

    let read_back = sources::read_json(&path);
    assert_eq!(read_back, original);
}

// ══════════════════════════════════════════════════════════════════════════
// NEW TESTS — load_all integration with new modules
// ══════════════════════════════════════════════════════════════════════════

#[test]
fn test_load_all_includes_stats_plugins_todos() {
    let wrapper = TempDir::new().unwrap();
    let project_root = TempDir::new().unwrap();
    let (paths, claude_dir) = make_nested_paths(&wrapper, &project_root);

    // Stats
    write_fixture(
        &claude_dir,
        "stats-cache.json",
        r#"{"totalSessions": 10, "totalMessages": 100}"#,
    );

    // Plugins
    write_fixture(
        &claude_dir,
        "plugins/installed_plugins.json",
        r#"{"plugins": {"test-plugin": [{"version": "1.0.0", "scope": "user"}]}}"#,
    );

    // Todos
    write_fixture(
        &claude_dir,
        "todos/test.json",
        r#"[{"id": "1", "content": "Test todo"}]"#,
    );

    let data = sources::load_all(&paths);

    assert_eq!(data.stats.total_sessions, 10);
    assert_eq!(data.stats.total_messages, 100);
    assert_eq!(data.plugins.installed.len(), 1);
    assert_eq!(data.plugins.installed[0].name, "test-plugin");
    assert_eq!(data.todos.len(), 1);
    assert_eq!(data.todos[0].content, "Test todo");
}

#[test]
fn test_load_all_empty_includes_default_stats_plugins_todos() {
    let wrapper = TempDir::new().unwrap();
    let project_root = TempDir::new().unwrap();
    let (paths, _claude_dir) = make_nested_paths(&wrapper, &project_root);

    let data = sources::load_all(&paths);

    assert_eq!(data.stats.total_sessions, 0);
    assert!(data.plugins.installed.is_empty());
    assert!(data.plugins.blocked.is_empty());
    assert!(data.plugins.marketplaces.is_empty());
    assert!(data.todos.is_empty());
}

// ══════════════════════════════════════════════════════════════════════════
// NEW TESTS — Plugins installed_at truncation edge cases
// ══════════════════════════════════════════════════════════════════════════

#[test]
fn test_plugins_installed_at_truncation() {
    let claude_dir = TempDir::new().unwrap();
    let project_root = TempDir::new().unwrap();
    let paths = make_paths(&claude_dir, &project_root);

    write_fixture(
        claude_dir.path(),
        "plugins/installed_plugins.json",
        r#"{
            "plugins": {
                "short-date": [{"version": "1.0", "installedAt": "2025"}],
                "exact-date": [{"version": "1.0", "installedAt": "2025-01-01"}],
                "long-date": [{"version": "1.0", "installedAt": "2025-01-01T12:00:00.000Z"}],
                "no-date": [{"version": "1.0"}]
            }
        }"#,
    );

    let data = sources::plugins::load(&paths);
    assert_eq!(data.installed.len(), 4);

    let find_plugin = |name: &str| data.installed.iter().find(|p| p.name == name).unwrap();

    assert_eq!(find_plugin("short-date").installed_at, "2025"); // < 10 chars, use as-is
    assert_eq!(find_plugin("exact-date").installed_at, "2025-01-01"); // exactly 10 chars
    assert_eq!(find_plugin("long-date").installed_at, "2025-01-01"); // truncated
    assert_eq!(find_plugin("no-date").installed_at, ""); // missing field
}

// ══════════════════════════════════════════════════════════════════════════
// NEW TESTS — Plugin registry README truncation
// ══════════════════════════════════════════════════════════════════════════

#[test]
fn test_plugin_registry_preview_truncates_long_readme() {
    // Create a README with more than 40 lines
    let long_readme: String = (0..50)
        .map(|i| format!("Line {i}"))
        .collect::<Vec<_>>()
        .join("\n");

    let entry = sources::plugin_registry::PluginEntry {
        name: "long-readme".to_string(),
        description: String::new(),
        version: String::new(),
        author: String::new(),
        marketplace: "mp".to_string(),
        readme: long_readme,
        has_agents: false,
        has_skills: false,
        has_hooks: false,
        has_commands: false,
        has_mcp: false,
    };

    let body = entry.preview_body();
    assert!(body.contains("Line 0"));
    assert!(body.contains("Line 39")); // Last line shown (0-indexed, 40 lines)
    assert!(body.contains("... (truncated)"));
    assert!(!body.contains("Line 40")); // Not shown
}

#[test]
fn test_plugin_registry_preview_short_readme_no_truncation() {
    let entry = sources::plugin_registry::PluginEntry {
        name: "short-readme".to_string(),
        description: String::new(),
        version: String::new(),
        author: String::new(),
        marketplace: "mp".to_string(),
        readme: "Just one line.".to_string(),
        has_agents: false,
        has_skills: false,
        has_hooks: false,
        has_commands: false,
        has_mcp: false,
    };

    let body = entry.preview_body();
    assert!(body.contains("Just one line."));
    assert!(!body.contains("truncated"));
}

// ══════════════════════════════════════════════════════════════════════════
// NEW TESTS — Plugins with invalid/malformed JSON structures
// ══════════════════════════════════════════════════════════════════════════

#[test]
fn test_plugins_invalid_installed_json() {
    let claude_dir = TempDir::new().unwrap();
    let project_root = TempDir::new().unwrap();
    let paths = make_paths(&claude_dir, &project_root);

    write_fixture(
        claude_dir.path(),
        "plugins/installed_plugins.json",
        "not json at all",
    );

    let data = sources::plugins::load(&paths);
    assert!(data.installed.is_empty());
}

#[test]
fn test_plugins_invalid_blocklist_json() {
    let claude_dir = TempDir::new().unwrap();
    let project_root = TempDir::new().unwrap();
    let paths = make_paths(&claude_dir, &project_root);

    write_fixture(claude_dir.path(), "plugins/blocklist.json", "corrupt");

    let data = sources::plugins::load(&paths);
    assert!(data.blocked.is_empty());
}

#[test]
fn test_plugins_plugins_key_is_not_object() {
    let claude_dir = TempDir::new().unwrap();
    let project_root = TempDir::new().unwrap();
    let paths = make_paths(&claude_dir, &project_root);

    // "plugins" is a string instead of an object
    write_fixture(
        claude_dir.path(),
        "plugins/installed_plugins.json",
        r#"{"plugins": "not an object"}"#,
    );

    let data = sources::plugins::load(&paths);
    assert!(data.installed.is_empty());
}

// ══════════════════════════════════════════════════════════════════════════
// NEW TESTS — Plugin registry skips invalid entries
// ══════════════════════════════════════════════════════════════════════════

#[test]
fn test_plugin_registry_skips_non_directories() {
    let tmp = TempDir::new().unwrap();
    let base = tmp.path();

    // Create a file instead of a directory in plugins/
    let mp_plugins = base.join("marketplaces/mp/plugins");
    fs::create_dir_all(&mp_plugins).unwrap();
    fs::write(mp_plugins.join("not-a-dir.txt"), "just a file").unwrap();

    let results = sources::plugin_registry::search_local(base, "").unwrap();
    assert!(results.is_empty());
}

#[test]
fn test_plugin_registry_skips_dir_without_plugin_json() {
    let tmp = TempDir::new().unwrap();
    let base = tmp.path();

    let plugin_dir = base.join("marketplaces/mp/plugins/incomplete");
    fs::create_dir_all(&plugin_dir).unwrap();
    // No .claude-plugin/plugin.json

    let results = sources::plugin_registry::search_local(base, "").unwrap();
    assert!(results.is_empty());
}

#[test]
fn test_plugin_registry_skips_invalid_json_in_plugin() {
    let tmp = TempDir::new().unwrap();
    let base = tmp.path();

    let cp = base.join("marketplaces/mp/plugins/bad-json/.claude-plugin");
    fs::create_dir_all(&cp).unwrap();
    fs::write(cp.join("plugin.json"), "{{not json}}").unwrap();

    let results = sources::plugin_registry::search_local(base, "").unwrap();
    assert!(results.is_empty());
}

// ══════════════════════════════════════════════════════════════════════════
// NEW TESTS — MCP registry date truncation edge case
// ══════════════════════════════════════════════════════════════════════════

#[test]
fn test_mcp_registry_preview_body_empty_date() {
    let entry = sources::mcp_registry::RegistryEntry {
        name: "test".to_string(),
        description: String::new(),
        version: "1.0.0".to_string(),
        install_command: "npx".to_string(),
        install_args: vec!["-y".to_string(), "test".to_string()],
        registry: "npm".to_string(),
        author: String::new(),
        date: String::new(),
        homepage: String::new(),
        repository: String::new(),
        npm_url: String::new(),
        keywords: Vec::new(),
        score_quality: 0.0,
        score_popularity: 0.0,
        score_maintenance: 0.0,
    };

    let body = entry.preview_body();
    // Empty date should not show "Published:" line
    assert!(!body.contains("Published:"));
}

// ── Commands ────────────────────────────────────────────────────────────

#[test]
fn test_commands_dual_scope() {
    let claude_dir = TempDir::new().unwrap();
    let project_root = TempDir::new().unwrap();
    let paths = make_paths(&claude_dir, &project_root);

    // User-level command: ~/.claude/commands/test.md
    write_fixture(
        claude_dir.path(),
        "commands/test.md",
        "---\nname: test\ndescription: Run tests\n---\nRun the test suite.",
    );

    // Project-level command: <project>/.claude/commands/deploy.md
    write_fixture(
        project_root.path(),
        ".claude/commands/deploy.md",
        "---\nname: deploy\ndescription: Deploy to prod\n---\nDeploy the app.",
    );

    let commands = sources::commands::load(&paths);

    assert_eq!(
        commands.len(),
        2,
        "expected 2 commands, got {}",
        commands.len()
    );

    let user_cmd = commands
        .iter()
        .find(|c| c.scope == Scope::User)
        .expect("user command");
    assert_eq!(user_cmd.name, "test");
    assert_eq!(user_cmd.description, "Run tests");
    assert_eq!(user_cmd.file_name, "test");
    assert_eq!(user_cmd.body, "Run the test suite.");

    let proj_cmd = commands
        .iter()
        .find(|c| c.scope == Scope::Project)
        .expect("project command");
    assert_eq!(proj_cmd.name, "deploy");
    assert_eq!(proj_cmd.description, "Deploy to prod");
    assert_eq!(proj_cmd.file_name, "deploy");
}

#[test]
fn test_commands_defaults_name_from_filename() {
    let claude_dir = TempDir::new().unwrap();
    let project_root = TempDir::new().unwrap();
    let paths = make_paths(&claude_dir, &project_root);

    // Command without name frontmatter — should use filename
    write_fixture(
        claude_dir.path(),
        "commands/review.md",
        "Review the latest changes.",
    );

    let commands = sources::commands::load(&paths);
    assert_eq!(commands.len(), 1);
    assert_eq!(commands[0].name, "review");
    assert_eq!(commands[0].file_name, "review");
    assert_eq!(commands[0].body, "Review the latest changes.");
}

#[test]
fn test_commands_empty_dir() {
    let claude_dir = TempDir::new().unwrap();
    let project_root = TempDir::new().unwrap();
    let paths = make_paths(&claude_dir, &project_root);

    // No commands directory at all
    let commands = sources::commands::load(&paths);
    assert!(commands.is_empty());
}

#[test]
fn test_commands_ignores_non_md_files() {
    let claude_dir = TempDir::new().unwrap();
    let project_root = TempDir::new().unwrap();
    let paths = make_paths(&claude_dir, &project_root);

    write_fixture(claude_dir.path(), "commands/valid.md", "A valid command.");
    write_fixture(claude_dir.path(), "commands/ignored.txt", "Not a command.");
    write_fixture(claude_dir.path(), "commands/also-ignored.json", "{}");

    let commands = sources::commands::load(&paths);
    assert_eq!(commands.len(), 1);
    assert_eq!(commands[0].name, "valid");
}

#[test]
fn test_commands_ignores_directories() {
    let claude_dir = TempDir::new().unwrap();
    let project_root = TempDir::new().unwrap();
    let paths = make_paths(&claude_dir, &project_root);

    write_fixture(claude_dir.path(), "commands/flat.md", "A command.");
    // Create a subdirectory (should be ignored — commands are flat files only)
    write_fixture(
        claude_dir.path(),
        "commands/subdir/nested.md",
        "Nested, should be ignored.",
    );

    let commands = sources::commands::load(&paths);
    assert_eq!(commands.len(), 1);
    assert_eq!(commands[0].name, "flat");
}
