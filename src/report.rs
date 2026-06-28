//! Non-interactive output: a ranked table of reclaimable artifacts for `disco scan`.

use std::path::Path;

use crate::format;
use crate::scan::Tree;

/// Print every detected artifact, largest first, with a reclaimable-space summary.
/// Paths are shown relative to `root` for legibility.
pub fn print_table(tree: &Tree, root: &Path) {
    let mut artifacts: Vec<_> = tree.artifacts().map(|(_, n)| n).collect();
    artifacts.sort_by(|a, b| b.size.cmp(&a.size));

    let total_scanned = tree.total_size();
    let reclaimable: u64 = artifacts.iter().map(|n| n.size).sum();

    if artifacts.is_empty() {
        println!(
            "No reclaimable artifacts found in {} ({} scanned).",
            root.display(),
            format::size(total_scanned)
        );
        return;
    }

    println!("{:>10}  {:<13} {:>5}  PATH", "SIZE", "KIND", "AGE");
    for n in &artifacts {
        let rel = n.path.strip_prefix(root).unwrap_or(&n.path);
        println!(
            "{:>10}  {:<13} {:>5}  {}",
            format::size(n.size),
            n.kind.unwrap_or(""),
            format::age(n.mtime),
            rel.display()
        );
    }

    println!();
    println!(
        "{} reclaimable across {} artifact(s)  ·  {} scanned in {}",
        format::size(reclaimable),
        artifacts.len(),
        format::size(total_scanned),
        root.display()
    );
}
