//! Command-line surface. Bare `disko [PATH]` opens the interactive browser;
//! subcommands give scriptable access.

use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "disko",
    version,
    about = "Find and reclaim developer build artifacts",
    after_help = "Exit codes: 0 success · 1 failure (incl. any item that could not be trashed) · 2 usage",
    // The bare-`disco` positional and the subcommands are mutually exclusive.
    args_conflicts_with_subcommands = true
)]
pub struct Cli {
    /// Directory to scan and browse interactively (default: current directory).
    pub path: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Option<Command>,
}

/// Filters shared by `scan` and `clean`, so a dry run and the real thing see the
/// same set of artifacts.
#[derive(Args)]
pub struct Filter {
    /// Only artifacts of these kinds (e.g. `Cargo,Node,venv`); matches kind name
    /// or artifact directory name, case-insensitive. Default: all kinds.
    #[arg(long, value_delimiter = ',')]
    pub kind: Vec<String>,

    /// Only artifacts not modified within this window (e.g. `30d`, `2w`, `6h`).
    #[arg(long)]
    pub older_than: Option<String>,
}

#[derive(Subcommand)]
pub enum Command {
    /// Scan a directory and print a ranked table of reclaimable artifacts.
    Scan {
        /// Directory to scan (default: current directory).
        path: Option<PathBuf>,

        #[command(flatten)]
        filter: Filter,

        /// Print the result as JSON on stdout (progress still goes to stderr).
        #[arg(long)]
        json: bool,
    },

    /// Reclaim artifacts by moving them to the Trash (recoverable). Dry-run unless
    /// `--yes`. disco never deletes permanently.
    Clean {
        /// Directory to scan (default: current directory).
        path: Option<PathBuf>,

        #[command(flatten)]
        filter: Filter,

        /// Actually reclaim. Without this, prints the plan and moves nothing.
        #[arg(long)]
        yes: bool,

        /// Print the plan and per-item outcomes as JSON on stdout.
        #[arg(long)]
        json: bool,
    },
}
