mod app;
mod ui;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

use ccm::config::Paths;
use ccm::sources;

#[derive(Parser)]
#[command(name = "ccm", about = "Claude Code Manager — TUI for managing all Claude Code configuration")]
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
        /// Source: memory, skills, mcp, settings, hooks, claude-md, keybindings, agents
        source: String,
    },
    /// Show resolved paths for debugging
    Paths,
}

fn main() -> Result<()> {
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

    // Default: TUI
    let mut terminal = ratatui::init();
    let result = app::App::with_paths(paths).run(&mut terminal);
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
                "mcp" => serde_json::to_string_pretty(&data.mcp)?,
                "settings" => serde_json::to_string_pretty(&data.settings)?,
                "hooks" => serde_json::to_string_pretty(&data.hooks)?,
                "claude-md" => serde_json::to_string_pretty(&data.claude_md)?,
                "keybindings" => serde_json::to_string_pretty(&data.keybindings)?,
                "agents" => serde_json::to_string_pretty(&data.agents)?,
                other => anyhow::bail!("unknown source: {other}. Options: memory, skills, mcp, settings, hooks, claude-md, keybindings, agents"),
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
        }
    }
    Ok(())
}
