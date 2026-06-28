//! Reveal a path in the macOS Finder — non-destructive, needs no special permission.

use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, bail};

/// Open Finder with `path` selected (`open -R`), so the user can inspect or act on
/// it themselves.
pub fn in_finder(path: &Path) -> Result<()> {
    let status = Command::new("open")
        .arg("-R")
        .arg(path)
        .status()
        .with_context(|| format!("failed to launch Finder for {}", path.display()))?;
    if !status.success() {
        bail!("`open -R` exited with {status}");
    }
    Ok(())
}
