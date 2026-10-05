# disco — agent notes

Rust CLI/TUI, single binary. The crate is `disco`; the installed command is `disko`.

- Spec: `DESIGN.md` is the contract. Trash-only is a hard invariant: no permanent
  delete path, ever. Read it before non-trivial changes.
- Gate: `make check` (fmt --check, clippy `-D warnings`, tests). Must be green before
  a commit. CI runs it on macOS and Linux plus an MSRV build and `cargo audit`.
- Layout: `src/` is layered leaf-first (`format`, `detect`, `scan`, `clean`, `report`,
  `tui`, `cli`, `main`); lower layers never import higher ones.
- Tests: unit tests live beside the code. Filesystem fixtures go through
  `testutil::unique_dir` so parallel tests never share a path.
- Platform: macOS first. Linux builds and runs; "reveal in Finder" is macOS-only.
- Commits: Conventional Commits, one change per commit, message says why.
