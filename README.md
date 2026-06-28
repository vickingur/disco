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

- **Ring 1 — scan + report (done):** `disco scan [PATH]` prints a ranked table of
  reclaimable artifacts with sizes and ages.
- Ring 2 — interactive TUI browser (next).
- Ring 3 — reclaim: mark + delete (Trash by default; `--purge` for permanent).
- Ring 4 — polish: cleanable filter, `--older-than`, `--json`.

## Usage

```sh
disco scan            # scan the current directory
disco scan ~/code     # scan a specific path
```

Sizes are real on-disk usage (block-based), not apparent file length. Symlinks are
never followed.

## Develop

```sh
cargo test
cargo clippy -- -D warnings
cargo run -- scan <path>
```
