//! disco — a disk-space analyzer that finds and reclaims developer build artifacts.

mod clean;
mod cli;
mod detect;
mod format;
mod report;
mod reveal;
mod scan;
#[cfg(test)]
mod testutil;
mod tui;

use std::io::{IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, SystemTime};

use anyhow::{Context, Result};
use clap::Parser;

use clean::Selection;
use cli::{Cli, Command, Filter};
use scan::{Progress, Tree};

fn main() -> Result<ExitCode> {
    let cli = Cli::parse();
    match cli.command {
        None => tui::run(resolve_root(cli.path)?).map(|()| ExitCode::SUCCESS),
        Some(Command::Scan { path, filter, json }) => run_scan(path, filter, json),
        Some(Command::Clean {
            path,
            filter,
            yes,
            json,
        }) => run_clean(path, filter, yes, json),
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

fn run_scan(path: Option<PathBuf>, filter: Filter, json: bool) -> Result<ExitCode> {
    let root = resolve_root(path)?;
    let sel = Selection::parse(filter.kind, filter.older_than.as_deref())?;
    let tree = scan_cli(&root)?;
    let targets = clean::select(&tree, &sel, SystemTime::now());
    if json {
        report::print_json(&report::ScanReport::new(&tree, &root, &targets));
    } else {
        report::print_scan_table(&tree, &root, &targets);
    }
    Ok(ExitCode::SUCCESS)
}

fn run_clean(path: Option<PathBuf>, filter: Filter, yes: bool, json: bool) -> Result<ExitCode> {
    let root = resolve_root(path)?;
    let sel = Selection::parse(filter.kind, filter.older_than.as_deref())?;
    let tree = scan_cli(&root)?;
    let targets = clean::select(&tree, &sel, SystemTime::now());
    let mut rep = report::CleanReport::plan(&root, &targets, !yes);

    if !json {
        if targets.is_empty() {
            println!("Nothing matches in {}.", root.display());
            return Ok(ExitCode::SUCCESS);
        }
        report::print_clean_plan(&root, &targets);
        if !yes {
            report::print_dry_run_note(rep.reclaimable_bytes, targets.len());
            return Ok(ExitCode::SUCCESS);
        }
    }

    if yes {
        for n in &targets {
            match clean::remove(&n.path) {
                Ok(()) => {
                    rep.reclaimed_bytes += n.size;
                    rep.results.push(report::Outcome {
                        path: n.path.display().to_string(),
                        status: "trashed",
                        error: None,
                    });
                }
                Err(e) => {
                    rep.failed += 1;
                    if !json {
                        eprintln!("  ! {e:#}");
                    }
                    rep.results.push(report::Outcome {
                        path: n.path.display().to_string(),
                        status: "failed",
                        error: Some(format!("{e:#}")),
                    });
                }
            }
        }
    }

    if json {
        report::print_json(&rep);
    } else {
        report::print_clean_summary(rep.reclaimed_bytes, rep.failed);
    }
    Ok(if rep.failed > 0 {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    })
}
