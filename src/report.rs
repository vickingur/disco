//! Non-interactive output for `scan` and `clean`: a human table, or JSON for
//! scripts and agents. The JSON shapes here are a public contract (see README);
//! add fields freely, never rename or remove one.

use std::path::Path;
use std::time::UNIX_EPOCH;

use serde::Serialize;

use crate::format;
use crate::scan::{Node, Tree};

/// One reclaimable artifact as reported to callers.
#[derive(Serialize)]
pub struct Artifact {
    /// Absolute path.
    pub path: String,
    /// Path relative to the scan root, for display.
    pub relative_path: String,
    /// Display name of the kind, e.g. `Cargo`, `Node`, `Python venv`.
    pub kind: &'static str,
    /// Real on-disk usage of the whole artifact.
    pub size_bytes: u64,
    /// Newest modification time anywhere inside, as Unix seconds; null if unknown.
    pub modified_unix: Option<u64>,
}

impl Artifact {
    pub fn from_node(n: &Node, root: &Path) -> Self {
        Self {
            path: n.path.display().to_string(),
            relative_path: n
                .path
                .strip_prefix(root)
                .unwrap_or(&n.path)
                .display()
                .to_string(),
            kind: n.kind.unwrap_or(""),
            size_bytes: n.size,
            modified_unix: n
                .mtime
                .and_then(|m| m.duration_since(UNIX_EPOCH).ok())
                .map(|d| d.as_secs()),
        }
    }
}

/// `disko scan --json`.
#[derive(Serialize)]
pub struct ScanReport {
    pub root: String,
    pub scanned_bytes: u64,
    pub reclaimable_bytes: u64,
    pub artifacts: Vec<Artifact>,
}

impl ScanReport {
    pub fn new(tree: &Tree, root: &Path, artifacts: &[&Node]) -> Self {
        Self {
            root: root.display().to_string(),
            scanned_bytes: tree.total_size(),
            reclaimable_bytes: artifacts.iter().map(|n| n.size).sum(),
            artifacts: artifacts
                .iter()
                .map(|n| Artifact::from_node(n, root))
                .collect(),
        }
    }
}

/// What happened to one artifact during `clean --yes`.
#[derive(Serialize)]
pub struct Outcome {
    pub path: String,
    /// `trashed` or `failed`.
    pub status: &'static str,
    /// The failure message when `status` is `failed`.
    pub error: Option<String>,
}

/// `disko clean --json`: the plan, and the outcomes when `--yes` was given.
#[derive(Serialize)]
pub struct CleanReport {
    pub root: String,
    pub dry_run: bool,
    pub reclaimable_bytes: u64,
    pub artifacts: Vec<Artifact>,
    /// Empty on a dry run.
    pub results: Vec<Outcome>,
    pub reclaimed_bytes: u64,
    pub failed: usize,
}

impl CleanReport {
    pub fn plan(root: &Path, artifacts: &[&Node], dry_run: bool) -> Self {
        Self {
            root: root.display().to_string(),
            dry_run,
            reclaimable_bytes: artifacts.iter().map(|n| n.size).sum(),
            artifacts: artifacts
                .iter()
                .map(|n| Artifact::from_node(n, root))
                .collect(),
            results: Vec::new(),
            reclaimed_bytes: 0,
            failed: 0,
        }
    }
}

/// Emit any report as one pretty-printed JSON document on stdout.
pub fn print_json<T: Serialize>(report: &T) {
    println!(
        "{}",
        serde_json::to_string_pretty(report).expect("report structs always serialize")
    );
}

/// `disko scan`: every selected artifact, largest first, with a summary.
pub fn print_scan_table(tree: &Tree, root: &Path, artifacts: &[&Node]) {
    let total_scanned = tree.total_size();
    if artifacts.is_empty() {
        println!(
            "No reclaimable artifacts found in {} ({} scanned).",
            root.display(),
            format::size(total_scanned)
        );
        return;
    }

    println!("{:>10}  {:<13} {:>5}  PATH", "SIZE", "KIND", "AGE");
    for n in artifacts {
        println!(
            "{:>10}  {:<13} {:>5}  {}",
            format::size(n.size),
            n.kind.unwrap_or(""),
            format::age(n.mtime),
            relative(n, root).display()
        );
    }

    let reclaimable: u64 = artifacts.iter().map(|n| n.size).sum();
    println!();
    println!(
        "{} reclaimable across {} artifact(s)  ·  {} scanned in {}",
        format::size(reclaimable),
        artifacts.len(),
        format::size(total_scanned),
        root.display()
    );
}

/// `disko clean`: the plan table, then either the dry-run note or the outcome.
pub fn print_clean_plan(root: &Path, artifacts: &[&Node]) {
    println!("{:>10}  {:<13} PATH", "SIZE", "KIND");
    for n in artifacts {
        println!(
            "{:>10}  {:<13} {}",
            format::size(n.size),
            n.kind.unwrap_or(""),
            relative(n, root).display()
        );
    }
    println!();
}

pub fn print_dry_run_note(reclaimable: u64, count: usize) {
    println!(
        "{} across {count} artifact(s) would be moved to Trash.\nDry run — re-run with --yes to reclaim.",
        format::size(reclaimable)
    );
}

pub fn print_clean_summary(reclaimed: u64, failed: usize) {
    let suffix = if failed > 0 {
        format!(" · {failed} failed")
    } else {
        String::new()
    };
    println!("Moved {} to Trash{suffix}.", format::size(reclaimed));
}

fn relative<'a>(n: &'a Node, root: &Path) -> &'a Path {
    n.path.strip_prefix(root).unwrap_or(&n.path)
}
