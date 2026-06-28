# disco

A small disk-space analyzer tailored for reclaiming developer build artifacts —
`.venv`, `node_modules`, Rust `target`, caches, and ~25 other project types. Walks a
directory, ranks everything by real disk usage, flags what's regenerable, and lets you
review then reclaim the space.

Detection rules are adapted from [kondo](https://github.com/tbillington/kondo) (MIT);
see [ATTRIBUTION.md](ATTRIBUTION.md). disco adds `pyvenv.cfg`-based virtualenv detection
and is built as a full-disk analyzer rather than an artifact-only cleaner.

The installed command is **`disko`** (`disco` is taken by Mono's discovery tool).

## Status

Built in vertical slices (see [DESIGN.md](DESIGN.md)):

- **Ring 1 — scan + report (done):** `disko scan` prints a ranked table of artifacts.
- **Ring 2 — interactive TUI browser (done):** `disko` opens a navigable size view.
- **Ring 3 — reclaim (done):** mark + confirm in the TUI; `disko clean` from the CLI.
- Ring 4 — polish: `--json` output, docs.

## Usage

```sh
disko                 # browse the current directory interactively (TUI)
disko ~/code          # browse a specific path
disko scan ~/code     # non-interactive ranked table

# Reclaim (always moves to the Trash — recoverable; never permanent):
disko clean ~/code                          # dry run — prints the plan, moves nothing
disko clean ~/code --kind venv,node_modules # filter by kind
disko clean ~/code --older-than 30d         # only stale artifacts
disko clean ~/code --yes                    # actually move matches to Trash
```

In the TUI: `↑↓` move · `→`/`←` drill in/out · `o` (or `⏎` in the cleanable view)
**reveal in Finder** · `c` cleanable-only view · `space` mark · `d` then `y` reclaim ·
`q` quit.

Reveal-in-Finder is the quickest non-destructive way to locate an artifact and act on
it yourself — `o` opens Finder with the selected folder selected.

On a large tree the scan runs on a background thread with a live progress display —
the TUI shows an animated scanning screen (`q` cancels), and `disko scan`/`clean`
print a progress line to the terminal (suppressed when piped). It never blocks blankly.

**Safety:** disco only ever moves to the OS Trash — there is no permanent-delete
option. Nothing is removed without a `y` keypress (TUI) or an explicit `--yes` (CLI).
Sizes are real on-disk usage (block-based); symlinks are never followed.

## Develop

```sh
cargo test
cargo clippy -- -D warnings
cargo run -- scan <path>
```
