//! Interactive browser state: where we are in the tree, what's selected, what's
//! marked for reclaiming. Pure logic — no rendering, no terminal I/O — so it's
//! testable in isolation.

use std::collections::HashSet;
use std::path::PathBuf;

use crate::clean;
use crate::format;
use crate::reveal;
use crate::scan::Tree;

/// Which list the user is looking at.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum View {
    /// Navigate the directory tree, ncdu-style.
    Browser,
    /// A flat list of every detected artifact, largest first.
    Cleanable,
}

pub struct App {
    pub tree: Tree,
    /// Directory currently being viewed (Browser view).
    pub cwd: usize,
    /// Indices of the rows shown in the current view, already sorted largest-first.
    pub rows: Vec<usize>,
    /// Cursor position into `rows`.
    pub cursor: usize,
    /// Node indices the user has marked to reclaim.
    pub marked: HashSet<usize>,
    pub view: View,
    /// True while the delete-confirmation prompt is showing.
    pub confirming: bool,
    /// A transient one-line message (e.g. the result of a reclaim), cleared on the
    /// next keypress.
    pub status: Option<String>,
    pub should_quit: bool,
}

impl App {
    pub fn new(tree: Tree) -> Self {
        let cwd = tree.root;
        let mut app = App {
            tree,
            cwd,
            rows: Vec::new(),
            cursor: 0,
            marked: HashSet::new(),
            view: View::Browser,
            confirming: false,
            status: None,
            should_quit: false,
        };
        app.rebuild_rows();
        app
    }

    /// The node index under the cursor, if any.
    pub fn selected(&self) -> Option<usize> {
        self.rows.get(self.cursor).copied()
    }

    /// Total disk usage marked for reclaiming.
    pub fn marked_size(&self) -> u64 {
        self.marked.iter().map(|&i| self.tree.nodes[i].size).sum()
    }

    /// Recompute `rows` for the current view, keeping the cursor in range.
    pub fn rebuild_rows(&mut self) {
        self.rows = match self.view {
            View::Browser => {
                let mut children = self.tree.nodes[self.cwd].children.clone();
                children.sort_by_key(|&i| std::cmp::Reverse(self.tree.nodes[i].size));
                children
            }
            View::Cleanable => {
                let mut artifacts: Vec<usize> = self.tree.artifacts().map(|(i, _)| i).collect();
                artifacts.sort_by_key(|&i| std::cmp::Reverse(self.tree.nodes[i].size));
                artifacts
            }
        };
        if self.cursor >= self.rows.len() {
            self.cursor = self.rows.len().saturating_sub(1);
        }
    }

    pub fn move_cursor(&mut self, delta: isize) {
        if self.rows.is_empty() {
            return;
        }
        let last = self.rows.len() - 1;
        let next = self.cursor as isize + delta;
        self.cursor = next.clamp(0, last as isize) as usize;
    }

    pub fn cursor_to(&mut self, pos: usize) {
        self.cursor = pos.min(self.rows.len().saturating_sub(1));
    }

    /// Descend into the selected directory (Browser view only).
    pub fn enter(&mut self) {
        if self.view != View::Browser {
            return;
        }
        if let Some(idx) = self.selected()
            && self.tree.nodes[idx].is_dir
            && !self.tree.nodes[idx].children.is_empty()
        {
            self.cwd = idx;
            self.cursor = 0;
            self.rebuild_rows();
        }
    }

    /// Go up to the parent directory, restoring the cursor onto the dir we left.
    pub fn leave(&mut self) {
        if self.view != View::Browser {
            return;
        }
        let left = self.cwd;
        if let Some(parent) = self.tree.nodes[self.cwd].parent {
            self.cwd = parent;
            self.rebuild_rows();
            let pos = self.rows.iter().position(|&i| i == left).unwrap_or(0);
            self.cursor_to(pos);
        }
    }

    pub fn toggle_mark(&mut self) {
        if let Some(idx) = self.selected()
            && !self.marked.remove(&idx)
        {
            self.marked.insert(idx);
        }
    }

    pub fn toggle_view(&mut self) {
        self.view = match self.view {
            View::Browser => View::Cleanable,
            View::Cleanable => View::Browser,
        };
        self.cursor = 0;
        self.rebuild_rows();
    }

    /// Reveal the selected item in the Finder so the user can inspect or remove it
    /// themselves. Reports the outcome in the status line.
    pub fn reveal(&mut self) {
        if let Some(idx) = self.selected() {
            let path = self.tree.nodes[idx].path.clone();
            self.status = Some(match reveal::in_finder(&path) {
                Ok(()) => format!("Revealed {} in Finder", self.tree.nodes[idx].name),
                Err(e) => format!("Could not reveal in Finder: {e}"),
            });
        }
    }

    /// Marked targets with any that are nested inside another marked target removed,
    /// so deleting them is disjoint (no double-counting, no child outliving a parent).
    fn reclaim_targets(&self) -> Vec<usize> {
        let marked: Vec<(usize, PathBuf)> = self
            .marked
            .iter()
            .map(|&i| (i, self.tree.nodes[i].path.clone()))
            .collect();
        marked
            .iter()
            .filter(|(_, path)| {
                !marked
                    .iter()
                    .any(|(_, other)| other != path && path.starts_with(other))
            })
            .map(|(i, _)| *i)
            .collect()
    }

    /// Count and total size that confirming a reclaim would actually free.
    pub fn reclaim_plan(&self) -> (usize, u64) {
        let targets = self.reclaim_targets();
        let size = targets.iter().map(|&i| self.tree.nodes[i].size).sum();
        (targets.len(), size)
    }

    /// Open the confirmation prompt if anything is marked.
    pub fn request_reclaim(&mut self) {
        if !self.marked.is_empty() {
            self.confirming = true;
        }
    }

    pub fn cancel_reclaim(&mut self) {
        self.confirming = false;
    }

    /// Move every marked target to the Trash (the TUI only ever does the reversible
    /// disposal; permanent removal is a deliberate CLI `--purge`). Updates the tree
    /// in place so sizes and listings reflect the reclaimed space immediately.
    pub fn confirm_reclaim(&mut self) {
        self.confirming = false;
        let mut reclaimed = 0u64;
        let mut failures = 0usize;
        for idx in self.reclaim_targets() {
            let path = self.tree.nodes[idx].path.clone();
            match clean::remove(&path) {
                Ok(()) => {
                    reclaimed += self.tree.nodes[idx].size;
                    self.detach(idx);
                }
                Err(_) => failures += 1,
            }
        }
        self.marked.clear();
        self.status = Some(match failures {
            0 => format!("Moved {} to Trash", format::size(reclaimed)),
            n => format!("Moved {} to Trash · {n} failed", format::size(reclaimed)),
        });
        self.rebuild_rows();
    }

    /// Remove a now-deleted subtree from the tree: subtract its size from every
    /// ancestor and detach it from its parent so it stops appearing in any view.
    fn detach(&mut self, idx: usize) {
        let size = self.tree.nodes[idx].size;
        let mut ancestor = self.tree.nodes[idx].parent;
        while let Some(a) = ancestor {
            self.tree.nodes[a].size = self.tree.nodes[a].size.saturating_sub(size);
            ancestor = self.tree.nodes[a].parent;
        }
        if let Some(parent) = self.tree.nodes[idx].parent {
            self.tree.nodes[parent].children.retain(|&c| c != idx);
        }
        let node = &mut self.tree.nodes[idx];
        node.kind = None;
        node.size = 0;
        node.children.clear();
    }

    /// Disk usage of the directory whose children fill the Browser view; used to
    /// scale the size bars. In the Cleanable view bars scale to the largest row.
    pub fn scale_size(&self) -> u64 {
        match self.view {
            View::Browser => self.tree.nodes[self.cwd].size,
            View::Cleanable => self
                .rows
                .first()
                .map(|&i| self.tree.nodes[i].size)
                .unwrap_or(0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    fn fixture() -> PathBuf {
        let p = std::env::temp_dir().join(format!("disco_app_{}", std::process::id()));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(p.join("a/node_modules")).unwrap();
        fs::write(p.join("a/package.json"), "{}").unwrap();
        fs::write(p.join("a/node_modules/x.js"), vec![0u8; 100_000]).unwrap();
        fs::create_dir_all(p.join("b")).unwrap();
        fs::write(p.join("b/small.txt"), vec![0u8; 1000]).unwrap();
        p
    }

    #[test]
    fn enter_and_leave_navigates() {
        let root = fixture();
        let tree = crate::scan::scan(&root).unwrap();
        let mut app = App::new(tree);
        // Largest child (a/) sorts first.
        let top = app.selected().unwrap();
        assert_eq!(app.tree.nodes[top].name, "a");
        app.enter();
        assert_eq!(app.tree.nodes[app.cwd].name, "a");
        app.leave();
        assert_eq!(app.cwd, app.tree.root);
        // Cursor lands back on the dir we left.
        assert_eq!(app.selected(), Some(top));
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn marking_accumulates_size() {
        let root = fixture();
        let tree = crate::scan::scan(&root).unwrap();
        let mut app = App::new(tree);
        app.toggle_view(); // Cleanable: lists node_modules
        let nm = app.selected().unwrap();
        assert_eq!(app.tree.nodes[nm].kind, Some("Node"));
        app.toggle_mark();
        assert_eq!(app.marked_size(), app.tree.nodes[nm].size);
        app.toggle_mark();
        assert_eq!(app.marked_size(), 0);
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn detach_subtracts_size_from_ancestors() {
        let root = fixture();
        let tree = crate::scan::scan(&root).unwrap();
        let mut app = App::new(tree);
        let before_root = app.tree.total_size();
        app.toggle_view();
        let nm = app.selected().unwrap(); // node_modules
        let nm_size = app.tree.nodes[nm].size;

        app.detach(nm);

        assert_eq!(app.tree.total_size(), before_root - nm_size);
        assert!(app.tree.nodes[nm].kind.is_none(), "detached node untagged");
        // It no longer appears in the cleanable listing.
        app.rebuild_rows();
        assert!(!app.rows.contains(&nm));
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn reclaim_targets_drops_nested_marks() {
        // Marking both a directory and something inside it must yield one disjoint
        // target (the ancestor), so deletion never double-counts.
        let p = std::env::temp_dir().join(format!("disco_disjoint_{}", std::process::id()));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(p.join("outer/inner")).unwrap();
        fs::write(p.join("outer/inner/f.bin"), vec![0u8; 10_000]).unwrap();
        let mut app = App::new(crate::scan::scan(&p).unwrap());

        let outer = app
            .tree
            .nodes
            .iter()
            .position(|n| n.name == "outer")
            .unwrap();
        let inner = app
            .tree
            .nodes
            .iter()
            .position(|n| n.name == "inner")
            .unwrap();
        app.marked.insert(outer);
        app.marked.insert(inner);

        let targets = app.reclaim_targets();
        assert_eq!(
            targets,
            vec![outer],
            "only the ancestor survives as a target"
        );
        fs::remove_dir_all(&p).ok();
    }
}
