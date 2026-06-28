//! Filesystem walk: builds an arena tree of directory/file nodes with real disk
//! usage (block-based, not apparent length) and tags developer artifacts inline.
//!
//! One pass does three things at once: sum sizes, record the most recent mtime per
//! subtree (for "age"), and classify cleanable artifact dirs via [`crate::detect`].
//! Subdirectories are walked in parallel on rayon's thread pool — each returns a
//! self-contained subtree arena that the parent stitches in by offsetting indices.
//! Symlinks are never followed — they're counted as their own (tiny) entry so the
//! walk can't escape the scan root or loop.

use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::SystemTime;

use rayon::prelude::*;

use crate::detect;

/// Live counters a long walk increments so a caller can render progress. Shared
/// across threads via `&Progress`; all reads/writes are `Relaxed` (counters only).
#[derive(Default)]
pub struct Progress {
    pub dirs: AtomicU64,
    pub files: AtomicU64,
    pub bytes: AtomicU64,
}

impl Progress {
    /// `(dirs, files, bytes)` seen so far.
    pub fn snapshot(&self) -> (u64, u64, u64) {
        (
            self.dirs.load(Ordering::Relaxed),
            self.files.load(Ordering::Relaxed),
            self.bytes.load(Ordering::Relaxed),
        )
    }
}

/// One filesystem entry in the arena. Children reference parents/children by index
/// into [`Tree::nodes`] — no `Rc`, cache-friendly, easy to iterate flat.
///
/// `name`/`parent`/`is_dir` are read by the interactive browser (next ring) for
/// navigation and display; `scan`'s table view only needs `path`/`size`/`mtime`/
/// `kind`. They're part of the one shared tree both views walk, not speculative.
#[allow(dead_code)]
pub struct Node {
    pub name: String,
    pub path: PathBuf,
    pub parent: Option<usize>,
    pub children: Vec<usize>,
    pub is_dir: bool,
    /// Disk usage in bytes (blocks × 512) for this entry plus all descendants.
    pub size: u64,
    /// Most recent modification time anywhere in this subtree.
    pub mtime: Option<SystemTime>,
    /// Display name of the cleanable artifact kind, if this node is one.
    pub kind: Option<&'static str>,
}

pub struct Tree {
    pub nodes: Vec<Node>,
    pub root: usize,
}

impl Tree {
    /// Total disk usage under the scan root.
    pub fn total_size(&self) -> u64 {
        self.nodes[self.root].size
    }

    /// Every node tagged as a cleanable artifact, as `(index, &Node)`.
    pub fn artifacts(&self) -> impl Iterator<Item = (usize, &Node)> {
        self.nodes
            .iter()
            .enumerate()
            .filter(|(_, n)| n.kind.is_some())
    }
}

/// Walk `root` and build the tree with no progress/cancellation — a convenience
/// wrapper used by the test suite. Production goes through [`scan_with_progress`].
#[cfg(test)]
pub fn scan(root: &Path) -> std::io::Result<Tree> {
    scan_with_progress(root, &Progress::default(), &AtomicBool::new(false))
}

/// Like [`scan`], but increments `progress` as it walks and stops early when `cancel`
/// is set. The walk runs in parallel across rayon's thread pool. On cancel it returns
/// whatever partial tree it built (callers typically discard it).
pub fn scan_with_progress(
    root: &Path,
    progress: &Progress,
    cancel: &AtomicBool,
) -> std::io::Result<Tree> {
    let name = root
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| root.to_string_lossy().into_owned());
    let subtree = walk(root.to_path_buf(), name, None, true, progress, cancel)
        .ok_or_else(|| std::io::Error::other("scan root is unreadable or scan was cancelled"))?;
    // The root subtree's local arena is already the whole tree, root at index 0.
    Ok(Tree {
        nodes: subtree,
        root: 0,
    })
}

/// One subtree's nodes as a self-contained arena: `nodes[0]` is the subtree's own
/// entry, and every `parent`/`children` index is local to this `Vec`. A parent merges
/// children by appending their arenas and offsetting their indices.
type Subtree = Vec<Node>;

/// Walk `path` and return its subtree arena. `assigned_kind` is set when the caller
/// already classified this entry (an artifact dir of a parent project); `detect`
/// is false once we're inside a tagged artifact or venv, so we don't re-detect (and
/// double-count) artifacts nested within artifacts. Subdirectories recurse in
/// parallel; results merge deterministically in directory-read order.
fn walk(
    path: PathBuf,
    name: String,
    assigned_kind: Option<&'static str>,
    detect: bool,
    progress: &Progress,
    cancel: &AtomicBool,
) -> Option<Subtree> {
    if cancel.load(Ordering::Relaxed) {
        return None;
    }

    // symlink_metadata so symlinks report as themselves and are never traversed.
    let meta = fs::symlink_metadata(&path).ok()?;
    let is_dir = meta.is_dir();
    let own_size = meta.blocks() * 512;

    if is_dir {
        progress.dirs.fetch_add(1, Ordering::Relaxed);
    } else {
        progress.files.fetch_add(1, Ordering::Relaxed);
    }
    progress.bytes.fetch_add(own_size, Ordering::Relaxed);

    let mut root = Node {
        name,
        path: path.clone(),
        parent: None,
        children: Vec::new(),
        is_dir,
        size: own_size,
        mtime: meta.modified().ok(),
        kind: assigned_kind,
    };

    if !is_dir {
        return Some(vec![root]);
    }

    let Ok(entries) = fs::read_dir(&path) else {
        // Unreadable directory (e.g. permissions): keep its own size, no children.
        return Some(vec![root]);
    };
    let entries: Vec<fs::DirEntry> = entries.filter_map(Result::ok).collect();

    // Names of regular files in this dir drive marker-based project detection.
    let file_names: Vec<String> = entries
        .iter()
        .filter(|e| e.file_type().map(|t| t.is_file()).unwrap_or(false))
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    let file_refs: Vec<&str> = file_names.iter().map(String::as_str).collect();

    // Classify this directory. A virtualenv is itself the artifact; a project root
    // instead tags specific child directories.
    let mut artifact_dirs: &[&str] = &[];
    let mut artifact_kind: Option<&'static str> = None;
    let mut is_venv = false;
    if detect {
        if detect::is_venv(&file_refs) {
            root.kind = Some(detect::VENV_KIND);
            is_venv = true;
        } else if let Some(kind) = detect::project_kind(&file_refs) {
            artifact_dirs = kind.artifact_dirs;
            artifact_kind = Some(kind.name);
        }
    }

    // Recurse into each child in parallel, then stitch the subtrees together.
    let child_subtrees: Vec<Subtree> = entries
        .into_par_iter()
        .filter_map(|entry| {
            let child_name = entry.file_name().to_string_lossy().into_owned();
            // Tag a child only when it's one of this project's direct artifact dirs.
            let child_kind = artifact_kind.filter(|_| artifact_dirs.contains(&child_name.as_str()));
            // Keep detecting in untagged children (so a project's sibling `.venv`, and
            // every nested artifact in a monorepo, still get found) but stop once inside
            // a tagged artifact or a venv — that prevents double-counting nested
            // artifacts against their already-counted parent.
            let child_detect = detect && !is_venv && child_kind.is_none();
            walk(
                entry.path(),
                child_name,
                child_kind,
                child_detect,
                progress,
                cancel,
            )
        })
        .collect();

    let mut nodes = vec![root];
    for mut child in child_subtrees {
        let offset = nodes.len();
        // Shift the child arena's internal indices into our frame, then reparent its
        // root onto this directory (index 0).
        for node in &mut child {
            node.parent = Some(node.parent.map_or(0, |p| p + offset));
            for c in &mut node.children {
                *c += offset;
            }
        }
        nodes[0].size += child[0].size;
        nodes[0].mtime = max_time(nodes[0].mtime, child[0].mtime);
        nodes[0].children.push(offset);
        nodes.extend(child);
    }
    Some(nodes)
}

/// The later of two optional timestamps.
fn max_time(a: Option<SystemTime>, b: Option<SystemTime>) -> Option<SystemTime> {
    match (a, b) {
        (Some(a), Some(b)) => Some(a.max(b)),
        (a, b) => a.or(b),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// A fresh, isolated temp directory for one test (cargo runs tests in parallel).
    fn unique_dir(tag: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("disco_scan_{}_{tag}", std::process::id()));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        p
    }

    fn kinds(tree: &Tree) -> Vec<(&'static str, String)> {
        tree.artifacts()
            .map(|(_, n)| (n.kind.unwrap(), n.name.clone()))
            .collect()
    }

    #[test]
    fn detects_venv_inside_python_project() {
        // Regression: a project root used to suppress detection of its own subtree,
        // hiding a sibling `.venv`. The venv and the pycache must both be tagged.
        let root = unique_dir("venv");
        fs::create_dir_all(root.join("pyproj/.venv/lib")).unwrap();
        fs::write(root.join("pyproj/main.py"), "x").unwrap();
        fs::write(root.join("pyproj/.venv/pyvenv.cfg"), "home = /usr").unwrap();
        fs::write(root.join("pyproj/.venv/lib/big.bin"), vec![0u8; 200_000]).unwrap();
        fs::create_dir_all(root.join("pyproj/__pycache__")).unwrap();
        fs::write(root.join("pyproj/__pycache__/m.pyc"), vec![0u8; 50_000]).unwrap();

        let tree = scan(&root).unwrap();
        let k = kinds(&tree);
        assert!(
            k.iter()
                .any(|(kind, n)| *kind == detect::VENV_KIND && n == ".venv"),
            "venv should be tagged: {k:?}"
        );
        assert!(
            k.iter()
                .any(|(kind, n)| *kind == "Python" && n == "__pycache__"),
            "pycache should be tagged: {k:?}"
        );
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn target_without_cargo_is_not_flagged() {
        let root = unique_dir("notrust");
        fs::create_dir_all(root.join("plain/target")).unwrap();
        fs::write(root.join("plain/target/data.bin"), vec![0u8; 10_000]).unwrap();

        let tree = scan(&root).unwrap();
        assert_eq!(tree.artifacts().count(), 0, "no Cargo.toml ⇒ no artifact");
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn nested_artifact_not_double_tagged() {
        // A package nested inside an already-tagged node_modules must not yield a
        // second tagged artifact (which would double-count against its parent).
        let root = unique_dir("nested");
        fs::create_dir_all(root.join("app/node_modules/dep")).unwrap();
        fs::write(root.join("app/package.json"), "{}").unwrap();
        fs::write(root.join("app/node_modules/dep/package.json"), "{}").unwrap();

        let tree = scan(&root).unwrap();
        assert_eq!(
            tree.artifacts().count(),
            1,
            "only the top node_modules is tagged"
        );
        fs::remove_dir_all(&root).ok();
    }
}
