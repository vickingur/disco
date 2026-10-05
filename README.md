# disco

A disk-space analyzer for developers. It walks a directory, ranks everything by real
on-disk usage, flags the build artifacts you can regenerate (`node_modules`, Rust
`target`, Python virtualenvs, caches, about two dozen project types) and lets you
review, then reclaim, the space. Browse interactively in a terminal UI or script it
from the command line.

The installed command is **`disko`** (`disco` is taken by Mono's discovery tool).

**Safety:** disco only ever moves things to the OS Trash. There is no permanent-delete
option, and nothing moves without a `y` keypress in the TUI or an explicit `--yes` on
the command line. Symlinks are never followed.

## Install

Requires Rust 1.88 or newer.

```sh
cargo install --git https://github.com/vickingur/disco
```

Or from a checkout: `make install` (runs `cargo install --path .`).

## Usage

```sh
disko                 # browse the current directory interactively (TUI)
disko ~/code          # browse a specific path
disko scan ~/code     # non-interactive ranked table of reclaimable artifacts

# Reclaim: always to the Trash, recoverable, never permanent
disko clean ~/code                          # dry run: prints the plan, moves nothing
disko clean ~/code --kind venv,node_modules # filter by kind or directory name
disko clean ~/code --older-than 30d         # only artifacts untouched for 30 days
disko clean ~/code --yes                    # actually move the matches to Trash
```

### TUI keys

| Key | Action |
|---|---|
| `↑` `↓` or `j` `k` | move |
| `→` `l` `⏎` / `←` `h` | drill into / out of a directory |
| `c` or `Tab` | toggle the cleanable-only view across the whole tree |
| `o` (or `⏎` in the cleanable view) | reveal the selected item in Finder (macOS) |
| `space` or `x` | mark / unmark for reclaim |
| `d` then `y` | move marked items (or the highlighted row) to Trash |
| `g` / `G` | jump to top / bottom |
| `q` or `Esc` | quit |

On a large tree the scan runs in the background with live progress. The TUI shows a
scanning screen (`q` cancels); `scan` and `clean` print a progress line to stderr
when attached to a terminal, so stdout stays clean for piping.

## How detection works

A directory is a project root when it contains a marker file; each project type then
maps to the directories under that root that are regenerable. A `target/` only counts
when a `Cargo.toml` sits beside it, not anywhere named `target`. Artifacts nested
inside another artifact are not reported separately: you reclaim the whole unit.

Supported: Cargo, Node, Turborepo, Python (`__pycache__`, `.pytest_cache`,
`.ruff_cache`, `.mypy_cache`, `.tox`, `.nox`), Python virtualenvs (any directory
containing `pyvenv.cfg`, whatever its name), Jupyter, Pixi, Maven, Gradle, SBT, Stack,
Cabal, CMake, Swift, Zig, Elixir, Pub/Flutter, Composer, CocoaPods, .NET, Unity,
Unreal, Godot, Terraform.

Sizes are real on-disk usage (block-based), not apparent file length, so sparse and
cloud-offloaded files are counted at what they actually occupy.

The marker-to-artifact table is adapted from [kondo](https://github.com/tbillington/kondo)
(MIT); see [ATTRIBUTION.md](ATTRIBUTION.md). disco adds virtualenv detection by
content and wraps the rules in a full-disk analyzer rather than an artifact-only
cleaner.

## Platform support

macOS is the primary target. Linux builds and runs: Trash goes through the
freedesktop trash spec, and "reveal in Finder" is unavailable. Windows is not
supported.

## Develop

```sh
make check            # fmt --check, clippy -D warnings, tests: the pre-commit gate
make run ARGS="scan ~/code"
make help             # all targets
```

[DESIGN.md](DESIGN.md) describes the architecture and the invariants (Trash-only,
symlinks never followed). See [CONTRIBUTING.md](CONTRIBUTING.md) before opening a
pull request.

## License

MIT. See [LICENSE](LICENSE).
