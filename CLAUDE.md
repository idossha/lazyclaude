# lazyclaude

A lazygit-inspired TUI for managing Claude Code configuration, built in Rust with ratatui.

## Build & Test

```sh
cargo build                    # debug build
cargo build --release          # release build (thin LTO, stripped)
cargo test                     # run all tests (115 integration tests)
cargo clippy -- -D warnings    # lint
cargo fmt --check              # format check
```

Logging: set `LAZYCLAUDE_LOG=debug` for verbose file logs at `~/.claude/lazyclaude.log`.

## Architecture

**Binary crate** (`src/main.rs`): CLI parsing (clap), logging init, panic recovery, TUI bootstrap.

**Library crate** (`src/lib.rs`): Exposes `config` and `sources` modules for programmatic use.

### Module layout

```
src/
  config.rs              # Path detection and resolution (Paths struct)
  app/
    mod.rs               # App state, Panel/Focus/InputMode enums, file watcher
    event_loop.rs        # Main run loop (poll, draw, events)
    keys.rs              # Key event handlers (normal, input, confirm, search)
    actions.rs           # CRUD operations, undo, clipboard, process_input
    navigation.rs        # Cursor movement, project selection, refresh
    resolve.rs           # Cursor-to-item resolvers
    search.rs            # Search overlay, background network fetch
    mouse.rs             # Mouse event handling
  sources/
    mod.rs               # Data types (SourceData, Scope), load_all(), helpers
    agents.rs            # Agent loader
    claude_md.rs         # CLAUDE.md and rules loader
    hooks.rs             # Hooks loader from settings.json
    keybindings.rs       # keybindings.json loader
    mcp.rs               # MCP server loader
    mcp_registry.rs      # npm registry search
    memory.rs            # Memory file loader
    plugins.rs           # Plugin loader
    plugin_registry.rs   # Marketplace plugin discovery
    projects.rs          # Project discovery
    sessions.rs          # JSONL session loader
    settings.rs          # Settings loader with 3-scope merge
    skills.rs            # Skill loader
    skills_registry.rs   # GitHub skill fetcher
    stats.rs             # Stats cache loader
    todos.rs             # Todo file loader
  ui/
    mod.rs               # Render entry point, overlays
    dashboard.rs         # Main layout (panels + detail + status bar)
    detail.rs            # Detail view items for all 11 panels
    help.rs              # Context-sensitive help screen
    markdown.rs          # Markdown-to-styled-lines renderer
    search_view.rs       # Search overlay render
    stats_view.rs        # Stats dashboard (sparklines, bar charts)
```

### 11 Panels

Projects, Config (CLAUDE.md + rules), Memory, Skills, Agents, MCP, Settings, Sessions, Stats, Plugins, Todos.

### Key types

- `Paths` (config.rs) -- all resolved filesystem paths
- `SourceData` (sources/mod.rs) -- aggregate of all loaded data
- `Scope` enum -- `User | Project | Local`
- `App` (app/mod.rs) -- TUI state machine

## Claude Code File Paths Reference

These are the canonical paths this tool reads/writes. Keep in sync with Claude Code docs.

### Settings
- `~/.claude/settings.json` -- user settings
- `<project>/.claude/settings.json` -- project settings (committed)
- `<project>/.claude/settings.local.json` -- local project settings (gitignored)

### CLAUDE.md & Rules
- `<project>/CLAUDE.md` or `<project>/.claude/CLAUDE.md` -- project instructions
- `~/.claude/CLAUDE.md` -- user-level instructions
- `<project>/.claude/rules/*.md` -- project rules (recursive subdirs supported)
- `~/.claude/rules/*.md` -- user-level rules

### Skills & Agents
- Skills: `~/.claude/skills/<name>/SKILL.md` (user), `<project>/.claude/skills/<name>/SKILL.md` (project)
- Agents: `~/.claude/agents/<name>.md` (user, flat files), `<project>/.claude/agents/<name>.md` (project, flat files)

### MCP Servers
- `~/.claude.json` -- user-level MCP (mcpServers field) and local per-project MCP
- `<project>/.mcp.json` -- project-level MCP (committed)

### Memory
- `~/.claude/projects/<encoded-path>/memory/MEMORY.md` -- index
- `~/.claude/projects/<encoded-path>/memory/*.md` -- topic files

### Sessions
- `~/.claude/projects/<encoded-path>/*.jsonl` -- session transcripts

### Other
- `~/.claude/keybindings.json` -- keyboard shortcuts
- `~/.claude/plugins/` -- installed plugins

## Coding Conventions

- Rust 2021 edition, stable toolchain
- Error handling: `anyhow::Result` for binary, proper Error types for library boundaries
- Serialization: `serde` + `serde_json` throughout
- TUI: `ratatui` 0.29 + `crossterm` 0.28
- All source loaders follow the pattern: read files, parse, return typed data
- Graceful degradation: missing files return empty/default data, never panic
- Integration tests in `tests/discovery.rs` use `tempfile` for isolated filesystem tests

## CI/CD

- `.github/workflows/ci.yml`: check, fmt, clippy, test on ubuntu + macos
- `.github/workflows/release.yml`: builds for 4 targets on `v*` tags, creates GitHub Release
- Tag format: `v0.x.y`

## Known Issues

- Hyphenated project paths may decode incorrectly (upstream Claude Code limitation)
- `is_multiple_of()` requires Rust 1.83+
