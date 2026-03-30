# lazyclaude

A lazygit-inspired TUI for managing Claude Code configuration. One place to view and edit memory, skills, MCP servers, permissions, hooks, instructions, keybindings, agents, sessions, stats, plugins, and todos — across user and project scopes.

## Install

### Prebuilt binaries (recommended)

Download the latest release for your platform from [GitHub Releases](https://github.com/idohaber/lazyclaude/releases/latest), then:

```sh
tar xzf lazyclaude-*.tar.gz
sudo install lazyclaude-*/lazyclaude /usr/local/bin/
```

### From crates.io

```sh
cargo install lazyclaude
```

### From source

```sh
git clone https://github.com/idohaber/lazyclaude.git
cd lazyclaude
cargo install --path .
```

## Usage

```sh
lazyclaude                    # launch TUI
lazyclaude --version          # show version
lazyclaude --json             # dump all config as JSON
lazyclaude list mcp           # list a single source as JSON
lazyclaude paths              # show resolved config paths
```

Override paths for custom setups:

```sh
lazyclaude --claude-dir ~/.claude --project-dir /path/to/project
```

Available sources for `list`: `memory`, `skills`, `mcp`, `settings`, `hooks`, `claude-md`, `keybindings`, `agents`, `stats`, `plugins`, `todos`.

## Navigation

| Key | Action |
|-----|--------|
| `1-9, 0` | Switch panel directly |
| `j/k` | Move up/down |
| `J/K` | Scroll detail preview |
| `Enter` | Select project / confirm action |
| `l` | Focus detail pane |
| `h/BS` | Back to panels |
| `Tab` | Toggle panels/detail focus |
| `/` | Filter items (fuzzy matching) |
| `Esc` | Clear filter / back |
| `?` | Help |
| `R` | Refresh data |
| `q` | Quit |

Mouse support: click to select panels/items, scroll wheel to navigate.

### Panel actions

| Key | Action |
|-----|--------|
| `e` | Edit in `$EDITOR` (Config/Memory/Skills/Agents) |
| `a` | Add / create item (Settings/MCP/Skills/Agents) |
| `d` | Delete item |
| `u` | Undo last delete |
| `D` | Add deny permission (Settings) |
| `t` | Toggle server (MCP) |
| `s` | Search registry (Skills/MCP/Plugins) |
| `y` | Copy to clipboard |
| `x` | Export panel data as JSON to clipboard |

### Search overlay

Press `s` on Skills, MCP, or Plugins to open a search overlay:

- Type to fuzzy-filter the list
- `Up/Down` to navigate results
- `Enter` or `y` to install to user scope, `p` for project scope
- `Tab` to preview details
- `Esc` to close
- Items already installed show a checkmark

Skills are sourced from [anthropics/skills](https://github.com/anthropics/skills). MCP packages are sourced from npm.

## Panels

| Key | Panel | Scope | Description |
|-----|-------|-------|-------------|
| `1` | Projects | -- | Switch active project context |
| `2` | Config | User + Project | CLAUDE.md and rules (edit) |
| `3` | Memory | Project | Memory files (edit/delete) |
| `4` | Skills | User + Project | Skill definitions (create/search/install) |
| `5` | Agents | User + Project | Agent definitions (create/edit) |
| `6` | MCP | User + Project | Servers (add/remove/toggle/search) |
| `7` | Settings | User/Project/Local | Permissions, hooks, keybindings (with diff view) |
| `8` | Sessions | Project | Conversation history |
| `9` | Stats | Global | Usage dashboard with charts |
| `0` | Plugins | Global | Installed, blocked, marketplaces (search/install) |
| -- | Todos | Global | Todo items from Claude sessions |

## Features

- **Auto-refresh**: Config files are watched for changes — the TUI updates automatically when you edit files in another terminal
- **Fuzzy search**: The `/` filter and search overlay use fuzzy matching (type `mcpgit` to find `mcp-server-git`)
- **Settings diff**: The Settings panel preview shows per-scope values (user/project/local) so you can see which scope defines each setting
- **Undo**: Press `u` to undo the last delete operation (supports memory, skills, agents, MCP servers, and permissions)
- **Clipboard**: `y` copies the current item, `x` exports the entire panel as JSON
- **Logging**: Diagnostics are written to `~/.claude/lazyclaude.log` (set `LAZYCLAUDE_LOG=debug` for verbose output)

## Architecture

lazyclaude is split into a library (`lazyclaude::config`, `lazyclaude::sources`) and a TUI binary. The library can be used by external tools:

```rust
let paths = lazyclaude::config::Paths::detect();
let data  = lazyclaude::sources::load_all(&paths);
```

The `--json` flag and `list` subcommand output structured JSON, enabling integration with editor plugins, scripts, or other tools.

## Releasing

Tag a version to trigger a release build:

```sh
git tag v0.1.0
git push origin v0.1.0
```

GitHub Actions builds binaries for macOS (x86_64, aarch64) and Linux (x86_64, aarch64), then publishes them as a GitHub Release.

## License

MIT
