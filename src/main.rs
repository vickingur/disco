//! disco — a disk-space analyzer that finds and reclaims developer build artifacts.

mod clean;
mod cli;
mod detect;
mod format;
mod report;
mod reveal;
mod scan;
mod tui;

use std::io::{IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, SystemTime};

use anyhow::{Context, Result};
use clap::Parser;

use cli::{Cli, Command};
use scan::{Progress, Tree};

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        None => tui::run(resolve_root(cli.path)?),
        Some(Command::Scan { path }) => {
            let root = resolve_root(path)?;
            let tree = scan_cli(&root)?;
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

/// Resolve `path` (default: current dir) to an absolute root, failing loudly if it
/// doesn't exist. Scanning happens separately so callers can show progress.
fn resolve_root(path: Option<PathBuf>) -> Result<PathBuf> {
    path.unwrap_or_else(|| PathBuf::from("."))
        .canonicalize()
        .with_context(|| "cannot access the requested directory".to_string())
}

/// Scan `root` for the non-interactive commands, printing a live progress line to a
/// TTY stderr (stdout stays clean for piping) while a worker thread does the walk.
fn scan_cli(root: &Path) -> Result<Tree> {
    let progress = Arc::new(Progress::default());
    let cancel = Arc::new(AtomicBool::new(false));
    let (tx, rx) = mpsc::channel();
    {
        let root = root.to_path_buf();
        let progress = Arc::clone(&progress);
        let cancel = Arc::clone(&cancel);
        thread::spawn(move || {
            let _ = tx.send(scan::scan_with_progress(&root, &progress, &cancel));
        });
    }

    let show = std::io::stderr().is_terminal();
    let tree = loop {
        match rx.try_recv() {
            Ok(result) => {
                if show {
                    eprint!("\r\x1b[K"); // clear the progress line
                    let _ = std::io::stderr().flush();
                }
                break result.with_context(|| format!("failed to scan {}", root.display()))?;
            }
            Err(mpsc::TryRecvError::Empty) => {}
            Err(mpsc::TryRecvError::Disconnected) => {
                anyhow::bail!("scan worker stopped unexpectedly")
            }
        }
        if show {
            let (dirs, files, bytes) = progress.snapshot();
            eprint!(
                "\r\x1b[KScanning… {dirs} dirs · {files} files · {}",
                format::size(bytes)
            );
            let _ = std::io::stderr().flush();
        }
        thread::sleep(Duration::from_millis(90));
    };
    Ok(tree)
}

fn run_clean(
    path: Option<PathBuf>,
    kind: Vec<String>,
    older_than: Option<String>,
    yes: bool,
) -> Result<()> {
    let root = resolve_root(path)?;
    let tree = scan_cli(&root)?;
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
