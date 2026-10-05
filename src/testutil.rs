//! Shared helpers for the unit tests. Compiled only under `cfg(test)`.

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

/// A fresh, empty temp directory that no other test shares.
///
/// cargo runs tests in parallel threads of one process, so a name keyed by the
/// process id alone collides across tests in the same module: one test's teardown
/// races another's setup (observed as `NotFound` panics on Linux). A per-process
/// counter makes every call unique; `tag` keeps the leftovers recognisable.
pub fn unique_dir(tag: &str) -> PathBuf {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let p = std::env::temp_dir().join(format!("disco_{tag}_{}_{n}", std::process::id()));
    let _ = fs::remove_dir_all(&p);
    fs::create_dir_all(&p).unwrap();
    p
}
