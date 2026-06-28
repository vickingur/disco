//! Command-line surface. `disco` launches the TUI (from ring 2); subcommands give
//! scriptable access.

use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "disco",
    version,
    about = "Find and reclaim developer build artifacts"
)]
pub struct Cli {
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
