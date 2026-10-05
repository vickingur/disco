# Contributing

Issues and pull requests are welcome.

## Before you start

- Read [DESIGN.md](DESIGN.md): it is the contract for how the tool behaves. In
  particular, disco is **Trash-only by design**. A pull request that adds a permanent
  delete path will not be merged.
- For anything beyond a small fix, open an issue first so the approach can be agreed
  before you spend time on it.

## Workflow

```sh
make check   # fmt --check, clippy -D warnings, tests — must pass before a PR
make run ARGS="scan ~/code"
```

- One change per pull request, on a branch off `main`.
- Commit messages follow [Conventional Commits](https://www.conventionalcommits.org/)
  (`feat:`, `fix:`, `perf:`, `docs:`, `chore:`) and say *why*, not just what.
- Add or update a test for behaviour you change. Tests that touch the filesystem use
  `testutil::unique_dir` so they stay isolated under parallel execution.
- CI runs the same gate on macOS and Linux, plus a build on the minimum supported
  Rust version (`rust-version` in `Cargo.toml`) and `cargo audit`.

## Adding a project type

Detection rules live in `src/detect.rs` as marker-file to artifact-directory
mappings. Add a rule, add a test beside the existing ones, and mention the kind in
the README's supported list.
