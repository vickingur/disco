//! disco — a disk-space analyzer that finds and reclaims developer build artifacts.

mod cli;
mod detect;
mod format;
mod report;
mod scan;

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::Parser;

use cli::{Cli, Command};

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Some(Command::Scan { path }) => run_scan(path),
        // The interactive TUI lands in the next ring; for now `disco` shows the table.
        None => run_scan(None),
    }
}

fn run_scan(path: Option<PathBuf>) -> Result<()> {
    let root = path.unwrap_or_else(|| PathBuf::from("."));
    let root = canonical_root(&root)?;
    let tree = scan::scan(&root).with_context(|| format!("failed to scan {}", root.display()))?;
    report::print_table(&tree, &root);
    Ok(())
}

/// Resolve the scan root to an absolute path, failing loudly if it doesn't exist.
fn canonical_root(path: &Path) -> Result<PathBuf> {
    path.canonicalize()
        .with_context(|| format!("cannot access {}", path.display()))
}
