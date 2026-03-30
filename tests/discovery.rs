//! Integration tests for ccm configuration source discovery.
//!
//! Each test creates a mock filesystem using `tempfile::TempDir` and verifies
//! that the loaders correctly discover files at both user and project scopes.

use std::fs;
use tempfile::TempDir;

use ccm::config::Paths;
use ccm::sources;

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

    let user_skill = skills.iter().find(|s| s.scope == "user").expect("user skill");
    assert_eq!(user_skill.name, "My Skill");
    assert_eq!(user_skill.description, "A user skill");
    assert!(user_skill.user_invocable);
    assert_eq!(user_skill.dir_name, "my-skill");
    assert_eq!(user_skill.body, "Do things.");

    let proj_skill = skills.iter().find(|s| s.scope == "project").expect("project skill");
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

    // User-level agent: ~/.claude/agents/my-agent/AGENT.md
    write_fixture(
        claude_dir.path(),
        "agents/my-agent/AGENT.md",
        "---\nname: My Agent\ndescription: A user agent\nmodel: opus\n---\nAgent instructions.",
    );

    // Project-level agent: <project>/.claude/agents/proj-agent/AGENT.md
    write_fixture(
        project_root.path(),
        ".claude/agents/proj-agent/AGENT.md",
        "---\nname: Proj Agent\ndescription: A project agent\nmodel: sonnet\n---\nProject agent.",
    );

    let agents = sources::agents::load(&paths);

    assert_eq!(agents.len(), 2, "expected 2 agents, got {}", agents.len());

    let user_agent = agents.iter().find(|a| a.scope == "user").expect("user agent");
    assert_eq!(user_agent.name, "My Agent");
    assert_eq!(user_agent.description, "A user agent");
    assert_eq!(user_agent.model, "opus");
    assert_eq!(user_agent.dir_name, "my-agent");
    assert_eq!(user_agent.body, "Agent instructions.");

    let proj_agent = agents.iter().find(|a| a.scope == "project").expect("project agent");
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
        "agents/fallback-name/AGENT.md",
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
    let claude_dir = TempDir::new().unwrap();
    let project_root = TempDir::new().unwrap();
    let paths = make_paths(&claude_dir, &project_root);

    // User-level: ~/.claude/.mcp.json
    write_fixture(
        claude_dir.path(),
        ".mcp.json",
        r#"{"mcpServers": {"user-server": {"command": "node", "args": ["user.js"]}}}"#,
    );

    // Project-level: <project>/.claude/.mcp.json
    write_fixture(
        project_root.path(),
        ".claude/.mcp.json",
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
    let claude_dir = TempDir::new().unwrap();
    let project_root = TempDir::new().unwrap();
    let paths = make_paths(&claude_dir, &project_root);

    write_fixture(
        claude_dir.path(),
        ".mcp.json",
        r#"{"mcpServers": {"off-server": {"command": "node", "args": [], "disabled": true}}}"#,
    );

    let mcp = sources::mcp::load(&paths);
    assert_eq!(mcp.user.len(), 1);
    assert!(mcp.user[0].disabled);
}

#[test]
fn test_mcp_with_env() {
    let claude_dir = TempDir::new().unwrap();
    let project_root = TempDir::new().unwrap();
    let paths = make_paths(&claude_dir, &project_root);

    write_fixture(
        claude_dir.path(),
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
        .find(|r| r.scope == "user")
        .expect("user allow rule");
    assert_eq!(user_allow.rule, "Bash(echo *)");

    let local_allow = settings
        .permissions
        .allow
        .iter()
        .find(|r| r.scope == "local")
        .expect("local allow rule");
    assert_eq!(local_allow.rule, "Read(*)");

    // Check deny rules
    assert_eq!(
        settings.permissions.deny.len(),
        1,
        "expected 1 deny rule, got {}",
        settings.permissions.deny.len()
    );
    assert_eq!(settings.permissions.deny[0].scope, "project");
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

    let user_hook = hooks.iter().find(|h| h.scope == "user").expect("user hook");
    assert_eq!(user_hook.event, "PostToolUse");
    assert_eq!(user_hook.matcher, "Bash");
    assert_eq!(user_hook.command, "echo user-hook");
    assert_eq!(user_hook.hook_type, "command");

    let proj_hook = hooks.iter().find(|h| h.scope == "project").expect("project hook");
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
    assert_eq!(hooks[0].scope, "local");
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

    // 4) User-level CLAUDE.md (parent of claude_dir = wrapper dir, simulating ~/)
    write_fixture(wrapper.path(), "CLAUDE.md", "# User CLAUDE");

    // 5) User-level rules (~/.claude/rules/user-rule.md)
    write_fixture(&claude_dir, "rules/user-rule.md", "# User rule");

    let files = sources::claude_md::load(&paths);

    assert_eq!(files.len(), 5, "expected 5 claude_md files, got {}", files.len());

    // Verify scopes
    let project_files: Vec<_> = files.iter().filter(|f| f.scope == "project").collect();
    let user_files: Vec<_> = files.iter().filter(|f| f.scope == "user").collect();
    assert_eq!(project_files.len(), 3, "expected 3 project-scope files");
    assert_eq!(user_files.len(), 2, "expected 2 user-scope files");

    // Verify file_type tags
    let claude_md_files: Vec<_> = files.iter().filter(|f| f.file_type == "claude_md").collect();
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
    assert_eq!(files[0].scope, "project");
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

    assert_eq!(bindings.len(), 2, "expected 2 keybindings, got {}", bindings.len());

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
    assert_eq!(bindings.len(), 3, "expected 3 keybindings, got {}", bindings.len());

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
    fs::write(
        mem_dir.join("note.md"),
        "---\nname: Note\n---\nSomething.",
    )
    .unwrap();

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
    // MCP paths
    assert_eq!(
        paths.mcp_path("user"),
        claude_dir.path().join(".mcp.json")
    );
    assert_eq!(
        paths.mcp_path("project"),
        project_root.path().join(".claude").join(".mcp.json")
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
        project_root.path().join(".claude").join("settings.local.json")
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
    let claude_dir = TempDir::new().unwrap();
    let project_root = TempDir::new().unwrap();
    let paths = make_paths(&claude_dir, &project_root);

    // --- Skills ---
    write_fixture(
        claude_dir.path(),
        "skills/test-skill/SKILL.md",
        "---\nname: Test Skill\ndescription: test\n---\nBody.",
    );

    // --- Agents ---
    write_fixture(
        claude_dir.path(),
        "agents/test-agent/AGENT.md",
        "---\nname: Test Agent\ndescription: test agent\nmodel: opus\n---\nAgent body.",
    );

    // --- MCP ---
    write_fixture(
        claude_dir.path(),
        ".mcp.json",
        r#"{"mcpServers": {"test-server": {"command": "node", "args": ["test.js"]}}}"#,
    );

    // --- Settings ---
    write_fixture(
        claude_dir.path(),
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
        claude_dir.path(),
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
    let claude_dir = TempDir::new().unwrap();
    let project_root = TempDir::new().unwrap();
    let paths = make_paths(&claude_dir, &project_root);

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
    let claude_dir = TempDir::new().unwrap();
    let project_root = TempDir::new().unwrap();
    let paths = make_paths(&claude_dir, &project_root);

    write_fixture(
        claude_dir.path(),
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
    assert_eq!(settings.permissions.allow[0].scope, "user");
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
    sources::settings::remove_permission(&paths, "user", "allow", 1)
        .expect("remove permission");

    let settings = sources::settings::load(&paths);
    assert_eq!(settings.permissions.allow.len(), 2);
    let rules: Vec<&str> = settings.permissions.allow.iter().map(|r| r.rule.as_str()).collect();
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
fn test_agents_non_directory_entries_ignored() {
    let claude_dir = TempDir::new().unwrap();
    let project_root = TempDir::new().unwrap();
    let paths = make_paths(&claude_dir, &project_root);

    write_fixture(claude_dir.path(), "agents/stray-file.txt", "Not an agent");
    write_fixture(
        claude_dir.path(),
        "agents/real-agent/AGENT.md",
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
    let claude_dir = TempDir::new().unwrap();
    let project_root = TempDir::new().unwrap();
    let paths = make_paths(&claude_dir, &project_root);

    write_fixture(claude_dir.path(), ".mcp.json", "broken json!!!");

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
        "agents/zeta/AGENT.md",
        "---\nname: Zeta\nmodel: opus\n---\nZ.",
    );
    write_fixture(
        claude_dir.path(),
        "agents/alpha/AGENT.md",
        "---\nname: Alpha\nmodel: opus\n---\nA.",
    );

    let agents = sources::agents::load(&paths);
    assert_eq!(agents.len(), 2);
    assert_eq!(agents[0].name, "Alpha");
    assert_eq!(agents[1].name, "Zeta");
}
