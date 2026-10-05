# disco: design

A disk-space analyzer tailored for reclaiming developer build artifacts. Walks a
directory tree, ranks everything by real disk usage, flags regenerable artifacts
(`.venv`, `node_modules`, `target`, caches, …), and lets you review and reclaim
space, interactively (TUI) or scripted (CLI).

## Goals

- **Full disk analyzer**: ncdu-style navigable tree, sorted by size, with size bars.
- **Artifact detection layered on top**: known cleanable kinds tagged inline, plus a
  filtered "cleanable" view across the whole tree.
- **Review then reclaim**: nothing moves without explicit confirmation.
- **Reversible only**: every reclaim goes to the OS Trash. There is no permanent
  delete path.
- Scoped tight and reliable for daily use over feature breadth.

Non-goals: graphical treemap/sunburst (not legible in a terminal), Windows support,
remote/network filesystems.

## Surfaces

The installed command is `disko` (`disco` collides with Mono's discovery tool); the
crate keeps the name `disco`.

- `disko [PATH]`: scan PATH (default: current dir) and open the TUI browser.
- `disko scan [PATH]`: non-interactive ranked table.
- `disko clean [PATH] [--kind K,…] [--older-than 30d] [--yes]`: scripted reclaim.
  A dry run that prints the plan unless `--yes` is given.

## Architecture (layers; lower never imports higher)

```
format   byte/age formatting helpers (leaf, pure)
detect   project kinds + classify(dir) rules (pure)
scan     parallel walk -> arena Tree<Node>; classifies artifact dirs during walk
clean    reclaim: move to Trash, plus the --older-than and --kind filters
report   scan -> ranked table (non-interactive)
reveal   open the selected path in Finder (macOS)
tui      ratatui browser over the Tree; consumes scan + clean + format + reveal
cli      clap command definitions
main     wiring / dispatch
```

The walk runs across rayon's thread pool; each subtree is built as a local arena and
spliced into its parent, so no shared mutable tree is needed during the scan. A
`Progress` counter and a cancel flag let the TUI and CLI show live progress and
abort.

## Data model

Arena tree (`Vec<Node>` + indices, no `Rc`): cache-friendly, no ref-counting.

```
Node { name, path, parent, children, is_dir, size, mtime, kind }
```

- `size` uses real disk blocks (`st_blocks * 512`), not apparent file length.
- Artifact directories are classified during the walk. Their size is still summed,
  but nothing nested inside them is classified: you reclaim the whole unit.

## Detection rules (detect.rs)

Model adapted from kondo (MIT, © 2020 Trent Billington; see ATTRIBUTION.md): a
directory is a **project root** when it contains a **marker file**; each project type
then maps to a set of **artifact directories** (relative to the root) that are the
cleanable units. This is more precise than name matching: a `target/` only counts
when there is a `Cargo.toml` beside it, not anywhere named `target`.

Examples (full table in `detect.rs`): `Cargo.toml → target`; `package.json →
node_modules`; `pom.xml → target`; `build.gradle → build,.gradle`; `*.py →
__pycache__,.pytest_cache,.ruff_cache,.mypy_cache,.tox,…`; `Package.swift →
.build,.swiftpm`; `*.csproj → bin,obj`; plus Unity, Unreal, Godot, Pub, Elixir, Zig,
Composer, CocoaPods, Terraform, Pixi and Turborepo.

**Beyond kondo:** a directory containing `pyvenv.cfg` is a Python virtualenv and is
itself the cleanable unit. This catches `.venv`, `venv`, `env` or any other name,
by content rather than by name.

Once a project root is detected, artifacts nested inside its artifact directories
are not re-detected (no `target` flagged inside `node_modules`). Sizes are still
summed for the analyzer view.

## Safety

disco **only ever moves to the OS Trash**, so every reclaim is recoverable. This is
a deliberate design decision, not a missing feature: a tool that bulk-removes
directories should not have a mode where a typo is unrecoverable.

- **Trash-only, everywhere.** No `--purge`, no `rm -rf`. Pull requests that add a
  permanent-delete path are out of scope.
- On macOS, Trash goes through Foundation's `NSFileManager` rather than Finder over
  AppleScript, so it needs no "control Finder" Automation permission. On Linux the
  `trash` crate's freedesktop backend is used.
- TUI: reclaim only via `d`, then a confirm modal showing count and reclaimable size,
  then `y`.
- CLI `clean`: a dry run that prints the plan and moves nothing unless `--yes`.
- Symlinks are never followed during the walk, so a scan cannot escape its root or
  loop.

**Reveal in Finder** (`o`, or `⏎` in the cleanable view) opens Finder with the
selected item selected (`open -R`): the non-destructive way to locate an artifact and
act on it yourself.

## Roadmap

- `disko scan --json` for machine-readable output.
- `--kind` filtering on `scan`, matching `clean`.
- A reveal equivalent on Linux (open the parent directory in the file manager).
