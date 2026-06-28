//! Command-line surface. Bare `disco [PATH]` opens the interactive browser;
//! subcommands give scriptable access.

use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "disco",
    version,
    about = "Find and reclaim developer build artifacts",
    // The bare-`disco` positional and the subcommands are mutually exclusive.
    args_conflicts_with_subcommands = true
)]
pub struct Cli {
    /// Directory to scan and browse interactively (default: current directory).
    pub path: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand)]
pub enum Command {
    /// Scan a directory and print a ranked table of reclaimable artifacts.
    Scan {
        /// Directory to scan (default: current directory).
        path: Option<PathBuf>,
    },

    /// Reclaim artifacts by moving them to the Trash (recoverable). Dry-run unless
    /// `--yes`. disco never deletes permanently.
    Clean {
        /// Directory to scan (default: current directory).
        path: Option<PathBuf>,

        /// Only artifacts of these kinds (e.g. `Cargo,Node,venv`); matches kind name
        /// or artifact directory name, case-insensitive. Default: all kinds.
        #[arg(long, value_delimiter = ',')]
        kind: Vec<String>,

        /// Only artifacts not modified within this window (e.g. `30d`, `2w`, `6h`).
        #[arg(long)]
        older_than: Option<String>,

        /// Actually reclaim. Without this, prints the plan and moves nothing.
        #[arg(long)]
        yes: bool,
    },
}
