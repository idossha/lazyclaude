# lazyclaude

A lazygit-inspired TUI for managing Claude Code configuration. One place to view and edit memory, skills, MCP servers, permissions, hooks, instructions, keybindings, agents, and sessions — across user and project scopes.

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
lazyclaude --json             # dump all config as JSON
lazyclaude list mcp           # list a single source as JSON
lazyclaude paths              # show resolved config paths
```

Override paths for custom setups:

```sh
lazyclaude --claude-dir ~/.claude --project-dir /path/to/project
```

Available sources for `list`: `memory`, `skills`, `mcp`, `settings`, `hooks`, `claude-md`, `keybindings`, `agents`.

## Navigation

| Key | Action |
|-----|--------|
| `1-8` | Switch panel directly |
| `j/k` | Move up/down |
| `J/K` | Scroll detail preview |
| `Enter` | Select project / confirm action |
| `l` | Focus detail pane |
| `h/BS` | Back to panels |
| `Tab` | Toggle panels/detail focus |
| `/` | Filter items |
| `Esc` | Clear filter / back |
| `?` | Help |
| `R` | Refresh data |
| `q` | Quit |

### Panel actions

| Key | Action |
|-----|--------|
| `e` | Edit in `$EDITOR` (Config/Memory/Skills/Agents) |
| `a` | Add item (Settings/MCP) |
| `d` | Delete item (Settings/MCP) |
| `D` | Add deny permission (Settings) |
| `t` | Toggle server (MCP) |
| `s` | Search registry (MCP) |

## Panels

| # | Panel | Scope | Read | Write |
|---|-------|-------|------|-------|
| 1 | Projects | -- | yes | Switch active project |
| 2 | Config | User + Project | yes | Edit |
| 3 | Memory | Project | yes | Edit |
| 4 | Skills | User + Project | yes | -- |
| 5 | Agents | User + Project | yes | -- |
| 6 | MCP Servers | User + Project | yes | Add/Remove/Toggle/Search |
| 7 | Settings | User/Project/Local | yes | Add/Delete |
| 8 | Sessions | Project | yes | -- |

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
