# ccm — Claude Code Manager

TUI dashboard for managing Claude Code configuration. One place to view and edit memory, skills, MCP servers, permissions, hooks, instructions, keybindings, and agents — across user and project scopes.

## Install

```sh
cargo install --path .
```

## Usage

```sh
ccm            # launch TUI
ccm --json     # dump all config as JSON (for scripting / nvim integration)
ccm list mcp   # list a single source as JSON
ccm paths      # show resolved config paths
```

Override paths for testing or custom setups:

```sh
ccm --claude-dir ~/.claude --project-dir /path/to/project
```

## Navigation

| Key | Action |
|-----|--------|
| `j/k` | Move up/down |
| `Enter` | Zoom into section |
| `h/Esc` | Back to dashboard |
| `Tab` | Switch panes |
| `/` | Filter |
| `a` | Add item |
| `d` | Delete item |
| `e` | Edit in $EDITOR |
| `t` | Toggle (MCP) |
| `s` | Search MCP registry |
| `R` | Refresh |
| `?` | Help |
| `q` | Quit |

## What it manages

| Section | Scope | Read | Write |
|---------|-------|------|-------|
| Memory | Project | yes | Edit |
| Skills | User + Project | yes | -- |
| MCP Servers | User + Project | yes | Add/Remove/Toggle/Search |
| Permissions | User/Project/Local | yes | Add/Delete |
| Hooks | User/Project/Local | yes | -- |
| CLAUDE.md & Rules | User + Project | yes | Edit |
| Keybindings | User | yes | -- |
| Agents | User + Project | yes | -- |

## Architecture

ccm is split into a library (`ccm::config`, `ccm::sources`) and a TUI binary. The library can be used by external tools:

```rust
let paths = ccm::config::Paths::detect();
let data = ccm::sources::load_all(&paths);
```

The `--json` flag and `list` subcommand output structured JSON, enabling integration with nvim plugins, scripts, or other tools via subprocess + JSON parsing.

## License

MIT
