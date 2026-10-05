# disco: notes for coding agents

Rust CLI/TUI, single binary, no runtime services. The crate is `disco`; the
installed command is `disko`.

## Orientation

| Need | Where |
|---|---|
| Behaviour contract | `DESIGN.md` (goals, surfaces, invariants, roadmap) |
| Public scripting contract | README "Scripting and agents"; pinned by `tests/cli.rs` |
| Detection rules | `src/detect.rs`, one `ProjectKind` per project type |
| Walk + tree | `src/scan.rs` (parallel, arena tree, real block sizes) |
| Selection filters | `clean::Selection` / `clean::select` (shared by scan and clean) |
| Output | `src/report.rs` (table + JSON), `src/tui/` (interactive) |
| Wiring | `src/cli.rs` (clap), `src/main.rs` (dispatch, exit codes) |

Layers import downward only: format, detect, scan, clean, report, reveal, tui, cli,
main.

## Hard invariants

- **Trash-only.** No permanent delete path, ever. Not behind a flag, not in a test.
- Nothing moves without `y` (TUI) or `--yes` (CLI). Dry run is the default.
- Symlinks are never followed.
- JSON fields and exit codes are only ever added, never renamed or removed.

## Verify

```sh
make check                      # fmt --check, clippy -D warnings, unit + CLI tests
make fixture                    # prints a sample tree path with 5 artifacts, 1 decoy, 1 stale
cargo run -- scan "$(make -s fixture)" --json
```

`make check` is the done bar and what CI runs (macOS + Linux, MSRV 1.88, cargo
audit). Tests take under a second. Never run `clean --yes` against anything but a
tree you created.

## Recipes

- **Add a project type:** add a `ProjectKind` to `PROJECT_KINDS` in `detect.rs`,
  add a test beside the existing ones, add the name to the README supported list.
- **Add a CLI flag:** `cli.rs` (clap derive) then `main.rs`; shared filters go in
  `cli::Filter` and `clean::Selection` so scan and clean stay in step; extend
  `tests/cli.rs`.
- **Add a JSON field:** add it to the struct in `report.rs`, assert it in
  `tests/cli.rs`, add it to the README example.
- **Change TUI behaviour:** `tui/app.rs` holds state and actions, `tui/ui.rs`
  renders, `tui/mod.rs` maps keys. Render tests use ratatui's `TestBackend`.
- **Filesystem fixtures in unit tests:** always `crate::testutil::unique_dir`.

## Conventions

- Conventional Commits, one change per commit, message says why.
- Branch off `main`, one PR per change, `make check` green before opening it.
- No dead code or compatibility shims; migrate callers in the same change.
