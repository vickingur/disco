# disco — design

A small disk-space analyzer tailored for cleaning up developer build artifacts.
Walks a directory tree, ranks everything by real disk usage, flags regenerable
artifacts (`.venv`, `node_modules`, `target`, caches, …), and lets you review +
reclaim space — interactively (TUI) or scripted (CLI).

## Goals

- **Full disk analyzer**: ncdu-style navigable tree, sorted by size, with size bars.
- **Artifact detection layered on top**: known cleanable kinds tagged inline + a
  filtered "cleanable" view across the whole tree.
- **Review then reclaim**: nothing is deleted without explicit confirmation.
- **Reversible deletion**: default = move to macOS Trash; permanent removal only
  behind an explicit `--purge` flag.
- Personal dogfood tool: scoped tight, reliable for daily use, no enterprise polish.

Non-goals: graphical treemap/sunburst (not legible in a terminal), Linux/Windows
trash backends for v1 (macOS first), remote/network filesystems.

## Surfaces

- `disco [PATH]` — scan PATH (default: `~`) and launch the TUI browser.
- `disco scan [PATH] [--kind K,…] [--min SIZE] [--json]` — non-interactive ranked table.
- `disco clean [PATH] --kind K,… [--older-than 30d] [--dry-run] [--purge] [--yes]`
  — scripted cleanup. Default is dry-run-safe: prints what it would do unless `--yes`.

## Architecture (layers; lower never imports higher)

```
format   byte/age formatting helpers (leaf, pure)
detect   ArtifactKind + classify(dir) rules (pure)
scan     parallel walk -> arena Tree<Node>; classifies artifact dirs during walk
model*   Tree/Node live in scan; marking state for the TUI
clean    deletion: trash (default) | purge, with dry-run; reversible-first
report   scan -> ranked table / JSON (non-interactive)
tui      ratatui browser over the Tree; consumes scan + clean + format
cli      clap command defs
main     wiring / dispatch
```

## Data model

Arena tree (`Vec<Node>` + indices, no `Rc`): cache-friendly, no ref-counting.

```
Node { name, parent, children, is_dir, own_size, total_size, mtime, kind }
```

- `total_size` uses real disk blocks (`st_blocks * 512`), not apparent file length.
- Artifact directories are classified during the walk; we still sum their size but
  do **not** classify anything nested inside them (you clean the whole unit).

## Detection rules (detect.rs)

Model lifted from kondo (MIT, © 2020 Trent Billington — see ATTRIBUTION.md):
a directory is a **project root** when it contains a **marker file**; each project
type then maps to a set of **artifact directories** (relative to the root) that are
the cleanable units. This is more precise than raw name-matching — a `target/` only
counts when there's a `Cargo.toml` beside it, not anywhere named `target`.

Examples (full table in `detect.rs`): `Cargo.toml → target`; `package.json →
node_modules` (or the React-Native set); `pom.xml → target`; `build.gradle →
build,.gradle`; `*.py → __pycache__,.pytest_cache,.ruff_cache,.mypy_cache,.tox,…`;
`Package.swift → .build,.swiftpm`; `*.csproj → bin,obj`; plus Unity/Unreal/Godot/
Pub/Elixir/Zig/Composer/CocoaPods/Terraform/Pixi/Turborepo.

**Our addition (kondo lacks it; you asked for it):** a directory containing
`pyvenv.cfg` is a Python virtualenv — the directory *itself* is the cleanable unit
(catches `.venv`/`venv`/`env`/any name, robustly, by content not name).

Once a project root is detected, we don't re-detect artifacts nested inside its
artifact dirs (no `target` flagged inside `node_modules`). Sizes are still summed
for the analyzer view.

## Safety (destructive actions explicit + reversible — security profile)

- TUI delete and `clean` both **default to Trash** (`trash` crate, macOS).
- Permanent removal requires `--purge`; never the default.
- `clean` without `--yes` is a dry run (prints the plan, deletes nothing).
- TUI delete shows a confirm modal with the count + total reclaimable size.
- Never follow symlinks during the walk (avoid escaping the scan root / loops).

## Rings (each end-to-end + runnable before the next)

1. **scan + `disco scan`** — walk, aggregate, detect, print ranked table. ✅ ship first.
2. **TUI browser** — ncdu-style navigate/drill, size bars, kind tags. Read-only.
3. **Reclaim** — mark in TUI + confirm delete; `disco clean` CLI; trash/purge/dry-run.
4. **Polish** — cleanable-only filter view, `--older-than`, `--json`, kinds config, README.
</content>
</invoke>
