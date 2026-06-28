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
}
