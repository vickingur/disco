//! disco — a disk-space analyzer that finds and reclaims developer build artifacts.

mod clean;
mod cli;
mod detect;
mod format;
mod report;
mod scan;
mod tui;

use std::path::PathBuf;
use std::time::SystemTime;

use anyhow::{Context, Result};
use clap::Parser;

use cli::{Cli, Command};
use scan::Tree;

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        None => {
            let (_, tree) = scan_root(cli.path)?;
            tui::run(tree)
        }
        Some(Command::Scan { path }) => {
            let (root, tree) = scan_root(path)?;
            report::print_table(&tree, &root);
            Ok(())
        }
        Some(Command::Clean {
            path,
            kind,
            older_than,
            yes,
        }) => run_clean(path, kind, older_than, yes),
    }
}

/// Resolve `path` (default: current dir) to an absolute root and scan it.
fn scan_root(path: Option<PathBuf>) -> Result<(PathBuf, Tree)> {
    let root = path
        .unwrap_or_else(|| PathBuf::from("."))
        .canonicalize()
        .with_context(|| "cannot access the requested directory".to_string())?;
    let tree = scan::scan(&root).with_context(|| format!("failed to scan {}", root.display()))?;
    Ok((root, tree))
}

fn run_clean(
    path: Option<PathBuf>,
    kind: Vec<String>,
    older_than: Option<String>,
    yes: bool,
) -> Result<()> {
    let (root, tree) = scan_root(path)?;
    let window = older_than.as_deref().map(clean::parse_window).transpose()?;
    let now = SystemTime::now();

    let mut targets: Vec<&scan::Node> = tree
        .artifacts()
        .map(|(_, n)| n)
        .filter(|n| clean::kind_matches(&kind, n.kind.unwrap_or(""), &n.name))
        .filter(|n| match window {
            None => true,
            // Require a known mtime older than the window; unknown ages are skipped.
            Some(w) => n
                .mtime
                .and_then(|m| now.duration_since(m).ok())
                .is_some_and(|age| age >= w),
        })
        .collect();
    targets.sort_by(|a, b| b.size.cmp(&a.size));

    if targets.is_empty() {
        println!("Nothing matches in {}.", root.display());
        return Ok(());
    }

    let total: u64 = targets.iter().map(|n| n.size).sum();

    println!("{:>10}  {:<13} PATH", "SIZE", "KIND");
    for n in &targets {
        let rel = n.path.strip_prefix(&root).unwrap_or(&n.path);
        println!(
            "{:>10}  {:<13} {}",
            format::size(n.size),
            n.kind.unwrap_or(""),
            rel.display()
        );
    }
    println!();

    if !yes {
        println!(
            "{} across {} artifact(s) would be moved to Trash.\nDry run — re-run with --yes to reclaim.",
            format::size(total),
            targets.len()
        );
        return Ok(());
    }

    let mut reclaimed = 0u64;
    let mut failures = 0usize;
    for n in &targets {
        match clean::remove(&n.path) {
            Ok(()) => reclaimed += n.size,
            Err(e) => {
                eprintln!("  ! {e:#}");
                failures += 1;
            }
        }
    }
    println!(
        "Moved {} to Trash{}.",
        format::size(reclaimed),
        if failures > 0 {
            format!(" · {failures} failed")
        } else {
            String::new()
        }
    );
    Ok(())
}
