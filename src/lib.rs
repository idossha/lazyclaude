//! lazyclaude — library interface.
//!
//! Provides programmatic access to all Claude Code configuration sources.
//! Use `Paths::detect()` for auto-detection or `Paths::new()` for custom roots.
//!
//! ```no_run
//! let paths = lazyclaude::config::Paths::detect();
//! let data = lazyclaude::sources::load_all(&paths);
//! println!("{} memory files", data.memory.files.len());
//! ```

pub mod config;
pub mod sources;
