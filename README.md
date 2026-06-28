# disco

A small disk-space analyzer tailored for reclaiming developer build artifacts —
`.venv`, `node_modules`, Rust `target`, caches, and ~25 other project types. Walks a
directory, ranks everything by real disk usage, flags what's regenerable, and lets you
review then reclaim the space.

Detection rules are adapted from [kondo](https://github.com/tbillington/kondo) (MIT);
see [ATTRIBUTION.md](ATTRIBUTION.md). disco adds `pyvenv.cfg`-based virtualenv detection
and is built as a full-disk analyzer rather than an artifact-only cleaner.

## Status

Built in vertical slices (see [DESIGN.md](DESIGN.md)):

- **Ring 1 — scan + report (done):** `disco scan` prints a ranked table of artifacts.
- **Ring 2 — interactive TUI browser (done):** `disco` opens a navigable size view.
- **Ring 3 — reclaim (done):** mark + confirm in the TUI; `disco clean` from the CLI.
- Ring 4 — polish: `--json` output, docs.

## Usage

```sh
disco                 # browse the current directory interactively (TUI)
disco ~/code          # browse a specific path
disco scan ~/code     # non-interactive ranked table

# Reclaim (always moves to the Trash — recoverable; never permanent):
disco clean ~/code                          # dry run — prints the plan, moves nothing
disco clean ~/code --kind venv,node_modules # filter by kind
disco clean ~/code --older-than 30d         # only stale artifacts
disco clean ~/code --yes                    # actually move matches to Trash
```

In the TUI: `↑↓` move · `→`/`←` drill in/out · `space` mark · `c` cleanable-only view ·
`d` then `y` reclaim · `q` quit.

**Safety:** disco only ever moves to the OS Trash — there is no permanent-delete
option. Nothing is removed without a `y` keypress (TUI) or an explicit `--yes` (CLI).
Sizes are real on-disk usage (block-based); symlinks are never followed.

## Develop

```sh
cargo test
cargo clippy -- -D warnings
cargo run -- scan <path>
```
