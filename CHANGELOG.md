# Changelog

All notable changes to lazyclaude will be documented in this file.

## [0.1.2] - 2026-04-01

### Added
- **Stats period filter**: cycle between All time / Last 7 days / Last 30 days with `[` / `]` keys
- **Derived metrics**: active days ratio, current streak, longest streak, most active day, favorite model, total tokens
- **Tokens per Day chart**: multi-model line chart with per-model colors, smooth box-drawing connections, and baseline returns on zero-usage days; replaces the hourly chart when data is available
- **Model breakdown**: shows usage percentages with input/output token counts
- Load `dailyModelTokens` from `stats-cache.json` for per-model daily breakdown
- Duration formatting now handles days (e.g. "7d 2h 16m")

### Changed
- Stats overview panel revamped with streaks, active days, and favorite model
- Hourly chart kept as fallback when `dailyModelTokens` is absent from cache

## [0.1.1] - 2026-02-05

### Added
- npm/PyPI/crates.io registry search for MCP servers, skills, and plugins
- Stats dashboard with GitHub-style year heatmap, interactive day selection, hourly activity chart
- Mouse support for heatmap date selection
- Demo GIF in README

### Fixed
- Silent errors on missing config files now handled gracefully
- Expanded search registries for better package discovery

## [0.1.0] - 2026-01-28

### Added
- Initial release
- 11-panel TUI: Projects, Config, Memory, Skills, Agents, MCP, Settings, Sessions, Stats, Plugins, Todos
- lazygit-inspired keyboard navigation with vim-style keys
- CRUD operations for skills, agents, MCP servers, settings permissions, memory files
- 3-scope settings merge (user < project < local)
- File watcher for auto-refresh on config changes
- Undo stack (20-deep) for destructive operations
- Search overlay with fuzzy matching
- Clipboard support (yank with `y`, export with `x`)
- Context-sensitive help screen
- CI/CD with GitHub Actions (check, fmt, clippy, test)
- Cross-platform release builds (macOS x86/arm, Linux x86/arm)
- Homebrew tap for easy installation
