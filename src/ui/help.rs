use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

use crate::app::Panel;
use super::theme::THEME;

pub(crate) fn render_help(frame: &mut Frame, panel: Panel, scroll: usize, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(THEME.border_focused))
        .title(format!(" Help: {} ", panel.label()));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut lines = Vec::new();

    // Panel description
    lines.push(Line::from(""));
    for line in panel_description(panel) {
        lines.push(body(line));
    }

    // File locations
    let locations = panel_locations(panel);
    if !locations.is_empty() {
        lines.push(Line::from(""));
        lines.push(section("  Location"));
        for (label, path) in locations {
            lines.push(location_line(label, path));
        }
    }

    // Management info
    let mgmt = panel_management(panel);
    if !mgmt.is_empty() {
        lines.push(Line::from(""));
        lines.push(section("  How it works"));
        for line in mgmt {
            lines.push(body(line));
        }
    }

    // Panel-specific keybindings
    lines.push(Line::from(""));
    lines.push(section("  Panel keybindings"));
    for (key, desc) in panel_keys(panel) {
        lines.push(key_line(key, desc));
    }

    // Global navigation
    lines.push(Line::from(""));
    lines.push(section("  Navigation"));
    for (key, desc) in global_keys() {
        lines.push(key_line(key, desc));
    }

    // References
    let refs = panel_references(panel);
    if !refs.is_empty() {
        lines.push(Line::from(""));
        lines.push(section("  References"));
        for (label, url) in refs {
            lines.push(ref_line(label, url));
        }
    }

    lines.push(Line::from(""));
    lines.push(dim("  Press ? to close  |  J/K to scroll"));
    lines.push(Line::from(""));

    let paragraph = Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .scroll((scroll as u16, 0));
    frame.render_widget(paragraph, inner);
}

// ── Per-panel content ────────────────────────────────────────────────────

fn panel_description(panel: Panel) -> Vec<&'static str> {
    match panel {
        Panel::Projects => vec![
            "  Switch between Claude Code projects. Each project",
            "  has its own configuration scope (memory, skills,",
            "  agents, MCP servers, settings). Select \"Global\" to",
            "  view user-level configuration only.",
        ],
        Panel::Config => vec![
            "  CLAUDE.md files and rules provide persistent",
            "  instructions to Claude Code. They are loaded at the",
            "  start of every conversation and guide Claude's",
            "  behavior, coding style, and project knowledge.",
        ],
        Panel::Memory => vec![
            "  Memory files let Claude Code remember context across",
            "  conversations. Claude creates and updates these",
            "  automatically when asked to remember something, or",
            "  when it learns important project details.",
            "  ",
            "  Types: user, feedback, project, reference.",
        ],
        Panel::Skills => vec![
            "  Skills are reusable slash commands that extend Claude",
            "  Code. Each skill is a SKILL.md file with instructions",
            "  that Claude follows when the skill is invoked. Skills",
            "  marked as user_invocable appear as /commands.",
        ],
        Panel::Agents => vec![
            "  Agents are specialized subagent definitions that",
            "  Claude Code can delegate tasks to. Each agent has its",
            "  own AGENT.md with a description, model preference,",
            "  and instructions for how to handle delegated work.",
        ],
        Panel::Mcp => vec![
            "  Model Context Protocol (MCP) servers extend Claude",
            "  Code with external tools, resources, and data sources.",
            "  Servers run as local processes that Claude communicates",
            "  with during conversations via a standardized protocol.",
        ],
        Panel::Settings => vec![
            "  Permissions, hooks, keybindings, and general settings",
            "  that control Claude Code's behavior. Settings are",
            "  merged hierarchically: User < Project < Local, with",
            "  the most specific scope winning.",
        ],
        Panel::Sessions => vec![
            "  Conversation history from Claude Code sessions. Each",
            "  session is stored as a JSONL file containing the full",
            "  message exchange between you and Claude. Browse past",
            "  conversations and review what was discussed.",
        ],
        Panel::Stats => vec![
            "  Usage statistics aggregated from your Claude Code",
            "  session history. View total sessions, messages, daily",
            "  activity patterns, model usage breakdown, and hourly",
            "  distribution of your Claude Code usage.",
        ],
        Panel::Plugins => vec![
            "  Plugins extend Claude Code with third-party tools and",
            "  capabilities. Search discovers plugins from local",
            "  marketplaces and the official Anthropic directory",
            "  (123+ plugins). Install, block, and manage plugins.",
        ],
        Panel::Todos => vec![
            "  Todo items tracked by Claude Code during sessions.",
            "  When Claude creates or updates a task list, items are",
            "  persisted here. Review pending work and completed",
            "  tasks from your conversations.",
        ],
    }
}

fn panel_locations(panel: Panel) -> Vec<(&'static str, &'static str)> {
    match panel {
        Panel::Projects => vec![("Discovery", "~/.claude/projects/<encoded-path>/")],
        Panel::Config => vec![
            ("User", "~/.claude/CLAUDE.md"),
            ("Project", "<project>/CLAUDE.md  or  .claude/CLAUDE.md"),
            ("User rules", "~/.claude/rules/*.md"),
            ("Project rules", "<project>/.claude/rules/*.md"),
        ],
        Panel::Memory => vec![("Files", "~/.claude/projects/<path>/memory/*.md")],
        Panel::Skills => vec![
            ("User", "~/.claude/skills/<name>/SKILL.md"),
            ("Project", "<project>/.claude/skills/<name>/SKILL.md"),
        ],
        Panel::Agents => vec![
            ("User", "~/.claude/agents/<name>/AGENT.md"),
            ("Project", "<project>/.claude/agents/<name>/AGENT.md"),
        ],
        Panel::Mcp => vec![("User", "~/.mcp.json"), ("Project", "<project>/.mcp.json")],
        Panel::Settings => vec![
            ("User", "~/.claude/settings.json"),
            ("Project", "<project>/.claude/settings.json"),
            ("Local", "<project>/.claude/settings.local.json"),
            ("Keybindings", "~/.claude/keybindings.json"),
        ],
        Panel::Sessions => vec![("Files", "~/.claude/projects/<path>/*.jsonl")],
        Panel::Stats => vec![("Cache", "~/.claude/stats-cache.json")],
        Panel::Plugins => vec![
            ("Installed", "~/.claude/plugins/installed_plugins.json"),
            ("Blocked", "~/.claude/plugins/blocklist.json"),
            ("Marketplaces", "~/.claude/plugins/known_marketplaces.json"),
        ],
        Panel::Todos => vec![("Files", "~/.claude/todos/*.json")],
    }
}

fn panel_management(panel: Panel) -> Vec<&'static str> {
    match panel {
        Panel::Projects => vec![
            "  Projects are auto-discovered from encoded directory",
            "  paths under ~/.claude/projects/. Select a project to",
            "  scope all other panels to that project's config.",
            "  \"Global\" shows only user-level (cross-project) data.",
        ],
        Panel::Config => vec![
            "  CLAUDE.md at the project root is loaded automatically.",
            "  Place repo-wide instructions there. Rules in .claude/",
            "  rules/ are also loaded and can be split by topic.",
            "  User-level CLAUDE.md applies to all projects.",
        ],
        Panel::Memory => vec![
            "  Each memory file has YAML frontmatter with name,",
            "  description, and type fields. Claude uses these to",
            "  decide when a memory is relevant. You can edit or",
            "  delete memory files to curate what Claude remembers.",
        ],
        Panel::Skills => vec![
            "  A skill is a directory containing SKILL.md with YAML",
            "  frontmatter (name, description, user_invocable). When",
            "  user_invocable is true, the skill appears as a slash",
            "  command. Search discovers skills from anthropics/skills",
            "  and ComposioHQ/awesome-claude-skills.",
        ],
        Panel::Agents => vec![
            "  Each agent directory contains AGENT.md with frontmatter",
            "  (name, description, model). Claude Code spawns agents",
            "  as subprocesses to handle specialized tasks. The model",
            "  field can override which Claude model the agent uses.",
        ],
        Panel::Mcp => vec![
            "  Each server entry in .mcp.json has: command (executable",
            "  to run), args (arguments), env (environment variables),",
            "  and disabled (toggle on/off). Claude discovers tools",
            "  from running servers at conversation start. Search the",
            "  npm, official MCP, and Smithery registries to discover",
            "  and install MCP servers.",
        ],
        Panel::Settings => vec![
            "  Permissions control which tools and shell commands",
            "  Claude can run (allow/deny rules with glob patterns).",
            "  Hooks run shell commands on events (e.g., after each",
            "  tool call). Settings merge across scopes: user-level",
            "  defaults, project overrides, local overrides.",
        ],
        Panel::Sessions => vec![
            "  Each JSONL line is one message in the conversation.",
            "  The first user message is shown as a summary. Session",
            "  size and last-modified time help identify recent work.",
        ],
        Panel::Stats => vec![
            "  Stats are computed from session history and cached.",
            "  Press R to refresh and recompute from current data.",
        ],
        Panel::Plugins => vec![
            "  Installed plugins are active in all conversations.",
            "  Blocked plugins are prevented from running. Use the",
            "  search registry to browse available marketplace plugins.",
        ],
        Panel::Todos => vec![
            "  Todos are extracted from Claude session data. Each",
            "  item has a status (pending/completed) and content",
            "  describing the task. Managed by Claude during sessions.",
        ],
    }
}

fn panel_keys(panel: Panel) -> Vec<(&'static str, &'static str)> {
    match panel {
        Panel::Projects => vec![("Enter", "Select / switch to project")],
        Panel::Config => vec![
            ("e", "Edit in $EDITOR"),
            ("y", "Copy to clipboard"),
            ("x", "Export panel as JSON"),
        ],
        Panel::Memory => vec![
            ("e", "Edit in $EDITOR"),
            ("d", "Delete memory file"),
            ("u", "Undo last delete"),
            ("y", "Copy to clipboard"),
            ("x", "Export panel as JSON"),
        ],
        Panel::Skills => vec![
            ("s", "Search skills registry"),
            ("a", "Create new skill"),
            ("e", "Edit in $EDITOR"),
            ("d", "Delete skill"),
            ("u", "Undo last delete"),
            ("y", "Copy to clipboard"),
            ("x", "Export panel as JSON"),
        ],
        Panel::Agents => vec![
            ("a", "Create new agent"),
            ("e", "Edit in $EDITOR"),
            ("d", "Delete agent"),
            ("u", "Undo last delete"),
            ("y", "Copy to clipboard"),
            ("x", "Export panel as JSON"),
        ],
        Panel::Mcp => vec![
            ("s", "Search MCP registries"),
            ("a", "Add MCP server"),
            ("t", "Toggle enable/disable"),
            ("d", "Delete server"),
            ("u", "Undo last delete"),
            ("y", "Copy to clipboard"),
            ("x", "Export panel as JSON"),
        ],
        Panel::Settings => vec![
            ("a", "Add allow permission"),
            ("D", "Deny selected permission"),
            ("d", "Delete permission/entry"),
            ("u", "Undo last delete"),
            ("y", "Copy to clipboard"),
            ("x", "Export panel as JSON"),
        ],
        Panel::Sessions => vec![("y", "Copy to clipboard"), ("x", "Export panel as JSON")],
        Panel::Stats => vec![("R", "Refresh / recompute stats")],
        Panel::Plugins => vec![
            ("s", "Search plugin marketplace"),
            ("d", "Remove / unblock plugin"),
            ("y", "Copy to clipboard"),
            ("x", "Export panel as JSON"),
        ],
        Panel::Todos => vec![("y", "Copy to clipboard"), ("x", "Export panel as JSON")],
    }
}

fn global_keys() -> Vec<(&'static str, &'static str)> {
    vec![
        ("1-0", "Switch panel directly"),
        ("j/k", "Move up/down in list"),
        ("J/K", "Scroll help or preview (fast)"),
        ("h/l", "Focus left / right"),
        ("Tab", "Cycle focus: panels > detail > preview"),
        ("Enter", "Select / drill into preview"),
        ("/", "Filter items (fuzzy)"),
        ("Esc", "Back / clear filter"),
        ("?", "Toggle this help"),
        ("R", "Refresh all data"),
        ("q", "Quit"),
    ]
}

fn panel_references(panel: Panel) -> Vec<(&'static str, &'static str)> {
    match panel {
        Panel::Projects => vec![(
            "Claude Code",
            "docs.anthropic.com/en/docs/claude-code/overview",
        )],
        Panel::Config => vec![(
            "CLAUDE.md",
            "docs.anthropic.com/en/docs/claude-code/memory#claudemd",
        )],
        Panel::Memory => vec![("Memory", "docs.anthropic.com/en/docs/claude-code/memory")],
        Panel::Skills => vec![
            ("Skills", "docs.anthropic.com/en/docs/claude-code/skills"),
            ("Official", "github.com/anthropics/skills"),
            ("Community", "github.com/ComposioHQ/awesome-claude-skills"),
        ],
        Panel::Agents => vec![(
            "Sub-agents",
            "docs.anthropic.com/en/docs/claude-code/sub-agents",
        )],
        Panel::Mcp => vec![
            ("MCP spec", "modelcontextprotocol.io"),
            (
                "Claude + MCP",
                "docs.anthropic.com/en/docs/claude-code/mcp-servers",
            ),
        ],
        Panel::Settings => vec![
            (
                "Settings",
                "docs.anthropic.com/en/docs/claude-code/settings",
            ),
            ("Hooks", "docs.anthropic.com/en/docs/claude-code/hooks"),
        ],
        Panel::Sessions => vec![(
            "Claude Code",
            "docs.anthropic.com/en/docs/claude-code/overview",
        )],
        Panel::Stats => vec![(
            "Claude Code",
            "docs.anthropic.com/en/docs/claude-code/overview",
        )],
        Panel::Plugins => vec![(
            "Claude Code",
            "docs.anthropic.com/en/docs/claude-code/overview",
        )],
        Panel::Todos => vec![(
            "Claude Code",
            "docs.anthropic.com/en/docs/claude-code/overview",
        )],
    }
}

// ── Line builders ────────────────────────────────────────────────────────

fn section(title: &str) -> Line<'_> {
    Line::from(Span::styled(
        title,
        Style::default()
            .fg(THEME.text_accent)
            .add_modifier(Modifier::BOLD),
    ))
}

fn body(text: &str) -> Line<'_> {
    Line::from(Span::styled(text, Style::default().fg(THEME.text_primary)))
}

fn dim(text: &str) -> Line<'_> {
    Line::from(Span::styled(text, Style::default().fg(THEME.text_secondary)))
}

fn key_line<'a>(key: &'a str, desc: &'a str) -> Line<'a> {
    Line::from(vec![
        Span::styled(
            format!("    {:<10}", key),
            Style::default().fg(THEME.text_emphasis),
        ),
        Span::styled(desc, Style::default().fg(THEME.text_primary)),
    ])
}

fn location_line<'a>(label: &'a str, path: &'a str) -> Line<'a> {
    Line::from(vec![
        Span::styled(
            format!("    {:<14}", label),
            Style::default().fg(THEME.text_secondary),
        ),
        Span::styled(path, Style::default().fg(THEME.text_success)),
    ])
}

fn ref_line<'a>(label: &'a str, url: &'a str) -> Line<'a> {
    Line::from(vec![
        Span::styled(
            format!("    {:<14}", label),
            Style::default().fg(THEME.text_secondary),
        ),
        Span::styled(
            url,
            Style::default()
                .fg(THEME.text_link)
                .add_modifier(Modifier::UNDERLINED),
        ),
    ])
}
