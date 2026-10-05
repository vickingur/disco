//! Reclaiming artifacts. disco only ever moves things to the OS Trash — there is no
//! permanent-delete path by design, so every reclaim is recoverable.

use std::path::Path;
use std::time::{Duration, SystemTime};

use anyhow::{Context, Result, bail};

use crate::scan::{Node, Tree};

/// Which artifacts a command acts on: the parsed `--kind` and `--older-than`
/// filters. `scan` and `clean` share it so a dry run and the real run agree.
pub struct Selection {
    pub kinds: Vec<String>,
    pub older_than: Option<Duration>,
}

impl Selection {
    /// Build from the raw CLI strings, failing on a malformed window.
    pub fn parse(kinds: Vec<String>, older_than: Option<&str>) -> Result<Self> {
        Ok(Self {
            kinds,
            older_than: older_than.map(parse_window).transpose()?,
        })
    }
}

/// The artifacts in `tree` that pass `sel`, largest first. An `--older-than`
/// window requires a known mtime at least that old; unknown ages are skipped.
pub fn select<'a>(tree: &'a Tree, sel: &Selection, now: SystemTime) -> Vec<&'a Node> {
    let mut out: Vec<&Node> = tree
        .artifacts()
        .map(|(_, n)| n)
        .filter(|n| kind_matches(&sel.kinds, n.kind.unwrap_or(""), &n.name))
        .filter(|n| match sel.older_than {
            None => true,
            Some(w) => n
                .mtime
                .and_then(|m| now.duration_since(m).ok())
                .is_some_and(|age| age >= w),
        })
        .collect();
    out.sort_by(|a, b| b.size.cmp(&a.size));
    out
}

/// Move `path` to the OS Trash. Fails loudly with the path in context so callers can
/// report which item failed and keep going.
pub fn remove(path: &Path) -> Result<()> {
    trash_to_os(path).with_context(|| format!("failed to move {} to Trash", path.display()))
}

/// On macOS, trash via Foundation's `NSFileManager` rather than driving Finder over
/// AppleScript — the latter needs the "control Finder" Automation permission, which a
/// fresh terminal/CLI hasn't been granted. `NSFileManager` needs no such grant.
#[cfg(target_os = "macos")]
fn trash_to_os(path: &Path) -> Result<(), trash::Error> {
    use trash::macos::{DeleteMethod, TrashContextExtMacos};
    let mut ctx = trash::TrashContext::default();
    ctx.set_delete_method(DeleteMethod::NsFileManager);
    ctx.delete(path)
}

#[cfg(not(target_os = "macos"))]
fn trash_to_os(path: &Path) -> Result<(), trash::Error> {
    trash::delete(path)
}

/// Parse a coarse time window like `30d`, `2w`, `6h`, `45m`, `90s`.
pub fn parse_window(s: &str) -> Result<Duration> {
    let s = s.trim();
    let split = s
        .find(|c: char| c.is_ascii_alphabetic())
        .with_context(|| format!("expected a number and unit like `30d`, got `{s}`"))?;
    let (num, unit) = s.split_at(split);
    let n: u64 = num
        .parse()
        .with_context(|| format!("invalid duration `{s}`"))?;
    let secs = match unit {
        "s" => n,
        "m" => n * 60,
        "h" => n * 3600,
        "d" => n * 86_400,
        "w" => n * 604_800,
        other => bail!("unknown duration unit `{other}` (use s/m/h/d/w)"),
    };
    Ok(Duration::from_secs(secs))
}

/// Whether an artifact passes a `--kind` filter. Empty filter matches everything;
/// otherwise a filter token (case-insensitive) must be a substring of the kind name
/// (so `venv` matches `Python venv`) or equal the artifact's directory name.
pub fn kind_matches(filters: &[String], kind_name: &str, dir_name: &str) -> bool {
    if filters.is_empty() {
        return true;
    }
    let kind = kind_name.to_lowercase();
    let dir = dir_name.to_lowercase();
    filters.iter().any(|f| {
        let f = f.to_lowercase();
        kind.contains(&f) || dir == f
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_windows() {
        assert_eq!(parse_window("30d").unwrap(), Duration::from_secs(2_592_000));
        assert_eq!(parse_window("2w").unwrap(), Duration::from_secs(1_209_600));
        assert_eq!(parse_window("6h").unwrap(), Duration::from_secs(21_600));
        assert!(parse_window("30x").is_err());
        assert!(parse_window("d").is_err());
    }

    #[test]
    fn kind_filter_matches_name_or_dir() {
        assert!(kind_matches(&[], "Cargo", "target"));
        assert!(kind_matches(&["venv".into()], "Python venv", ".venv"));
        assert!(kind_matches(
            &["node_modules".into()],
            "Node",
            "node_modules"
        ));
        assert!(!kind_matches(&["rust".into()], "Node", "node_modules"));
    }
}
