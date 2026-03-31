mod app;
mod ui;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tracing_subscriber::EnvFilter;

use lazyclaude::config::Paths;
use lazyclaude::sources;

#[derive(Parser)]
#[command(
    name = "lazyclaude",
    version,
    about = "A lazygit-inspired TUI for managing Claude Code configuration"
)]
struct Cli {
    /// Dump all configuration as JSON (for scripting / nvim integration)
    #[arg(long)]
    json: bool,

    /// Override the Claude home directory (default: ~/.claude)
    #[arg(long, value_name = "DIR")]
    claude_dir: Option<PathBuf>,

    /// Override the project root directory (default: git root or cwd)
    #[arg(long, value_name = "DIR")]
    project_dir: Option<PathBuf>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// List items from a source
    List {
        /// Source: memory, skills, commands, mcp, settings, hooks, claude-md, keybindings, agents
        source: String,
    },
    /// Show resolved paths for debugging
    Paths,
}

fn main() -> Result<()> {
    // Initialize file-based logging before anything else.
    // Logs go to ~/.claude/lazyclaude.log; control verbosity with LAZYCLAUDE_LOG env var.
    let log_dir = dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("/tmp"))
        .join(".claude");
    let file_appender = tracing_appender::rolling::never(&log_dir, "lazyclaude.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    tracing_subscriber::fmt()
        .with_writer(non_blocking)
        .with_env_filter(
            EnvFilter::try_from_env("LAZYCLAUDE_LOG").unwrap_or_else(|_| EnvFilter::new("warn")),
        )
        .with_ansi(false)
        .init();

    tracing::info!("lazyclaude starting");

    let cli = Cli::parse();

    let paths = build_paths(&cli);

    // JSON dump mode
    if cli.json {
        let data = sources::load_all(&paths);
        println!("{}", serde_json::to_string_pretty(&data)?);
        return Ok(());
    }

    // Subcommands
    if let Some(cmd) = cli.command {
        return run_command(cmd, &paths);
    }

    // Default: TUI — install a panic hook that restores the terminal first
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        ratatui::restore();
        original_hook(panic_info);
    }));

    let mut terminal = ratatui::init();
    crossterm::execute!(std::io::stdout(), crossterm::event::EnableMouseCapture).ok();
    let result = app::App::with_paths(paths).run(&mut terminal);
    crossterm::execute!(std::io::stdout(), crossterm::event::DisableMouseCapture).ok();
    ratatui::restore();
    result
}

fn build_paths(cli: &Cli) -> Paths {
    let base = Paths::detect();
    Paths::new(
        cli.claude_dir.clone().unwrap_or(base.claude_dir),
        cli.project_dir.clone().unwrap_or(base.project_root),
    )
}

fn run_command(cmd: Commands, paths: &Paths) -> Result<()> {
    match cmd {
        Commands::List { source } => {
            let data = sources::load_all(paths);
            let json = match source.as_str() {
                "memory" => serde_json::to_string_pretty(&data.memory)?,
                "skills" => serde_json::to_string_pretty(&data.skills)?,
                "commands" => serde_json::to_string_pretty(&data.commands)?,
                "mcp" => serde_json::to_string_pretty(&data.mcp)?,
                "settings" => serde_json::to_string_pretty(&data.settings)?,
                "hooks" => serde_json::to_string_pretty(&data.hooks)?,
                "claude-md" => serde_json::to_string_pretty(&data.claude_md)?,
                "keybindings" => serde_json::to_string_pretty(&data.keybindings)?,
                "agents" => serde_json::to_string_pretty(&data.agents)?,
                "stats" => serde_json::to_string_pretty(&data.stats)?,
                "plugins" => serde_json::to_string_pretty(&data.plugins)?,
                "todos" => serde_json::to_string_pretty(&data.todos)?,
                other => anyhow::bail!("unknown source: {other}. Options: memory, skills, commands, mcp, settings, hooks, claude-md, keybindings, agents, stats, plugins, todos"),
            };
            println!("{json}");
        }
        Commands::Paths => {
            println!("claude_dir:   {}", paths.claude_dir.display());
            println!("project_root: {}", paths.project_root.display());
            println!("memory_dir:   {}", paths.memory_dir().display());
            println!("keybindings:  {}", paths.keybindings_path().display());
            println!("mcp (user):   {}", paths.mcp_path("user").display());
            println!("mcp (proj):   {}", paths.mcp_path("project").display());
            println!("settings (u): {}", paths.settings_path("user").display());
            println!("settings (p): {}", paths.settings_path("project").display());
            println!("settings (l): {}", paths.settings_path("local").display());
            println!("skills (u):   {}", paths.user_skills_dir().display());
            println!("skills (p):   {}", paths.project_skills_dir().display());
            println!("agents (u):   {}", paths.user_agents_dir().display());
            println!("agents (p):   {}", paths.project_agents_dir().display());
            println!("commands (u): {}", paths.user_commands_dir().display());
            println!("commands (p): {}", paths.project_commands_dir().display());
        }
    }
    Ok(())
}
