//! Interactive browser state: where we are in the tree, what's selected, what's
//! marked for reclaiming. Pure logic — no rendering, no terminal I/O — so it's
//! testable in isolation.

use std::collections::HashSet;

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
}
