//! Command-line interface definitions.

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "wxemma",
    version,
    about = "Run multiple WeChat instances on macOS"
)]
pub struct Cli {
    /// Emit machine-readable JSON instead of human text.
    #[arg(long, global = true)]
    pub json: bool,

    /// Assume yes; never prompt (required for non-interactive use).
    #[arg(long, short = 'y', global = true)]
    pub yes: bool,

    /// Force language: `zh` or `en`.
    #[arg(long, global = true)]
    pub lang: Option<String>,

    /// Verbose diagnostics.
    #[arg(long, global = true)]
    pub verbose: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Create one instance at the smallest free index.
    Add {
        /// Optional ASCII note/label for the instance.
        #[arg(long)]
        note: Option<String>,
    },
    /// List all instances.
    List,
    /// Show original-vs-copy version status.
    Status,
    /// Remove an instance by index (prompts if omitted).
    Remove {
        index: Option<u8>,
        /// Also delete the instance's account data.
        #[arg(long)]
        purge_data: bool,
    },
    /// Rebuild all copies from the current WeChat version.
    Rebuild,
    /// Launch all copies, or one by index.
    Open { index: Option<u8> },
    /// Terminate all copy processes.
    Kill,
    /// Check the environment is ready.
    Doctor,
    /// Emit a shell completion script.
    Completions { shell: clap_complete::Shell },
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn parses_add_with_note() {
        let cli = Cli::parse_from(["wxemma", "add", "--note", "work"]);
        assert!(!cli.json);
        match cli.command {
            Commands::Add { note } => assert_eq!(note.as_deref(), Some("work")),
            _ => panic!("expected add"),
        }
    }

    #[test]
    fn parses_global_json_and_yes() {
        let cli = Cli::parse_from(["wxemma", "--json", "-y", "remove", "2", "--purge-data"]);
        assert!(cli.json && cli.yes);
        match cli.command {
            Commands::Remove { index, purge_data } => {
                assert_eq!(index, Some(2));
                assert!(purge_data);
            }
            _ => panic!("expected remove"),
        }
    }
}
