# Working in this repo

Source of truth for the roadmap: `PRD.md`.

## Quick commands

```bash
# w
cargo fmt --check
cargo clippy --workspace -- -D warnings
cargo test --workspace

# vendored worktrunk (run unit tests; integration snapshots are brittle in subtree)
cargo fmt --check --manifest-path vendor/worktrunk/Cargo.toml
cargo clippy --manifest-path vendor/worktrunk/Cargo.toml --workspace -- -D warnings
cargo test --manifest-path vendor/worktrunk/Cargo.toml --workspace --lib --bins

# docs (zola)
cd docs && zola build
cd docs && zola serve
# if `zola` isn't installed locally, use Docker:
cd docs && docker run --rm -v "$(pwd)":/app -w /app ghcr.io/getzola/zola:v0.19.2 build
```

## Notes

- Prefer small, reviewable diffs.
- Keep vendored upstream code changes upstreamable under `vendor/worktrunk/`.

## Gotchas (from iteration logs)

- `cargo new` creates a nested git repo by default; prefer `cargo new --vcs none ...` for new crates.
- If `cargo fmt`/`cargo metadata` fails with "multiple workspace roots found" after adding a path dependency on `vendor/worktrunk`, add `exclude = ["vendor/worktrunk"]` to the root workspace `Cargo.toml`.
- Homebrew tap naming: `brew tap prateek/w` assumes a `prateek/homebrew-w` repo; this tap lives in `prateek/w`, so use `brew tap prateek/w https://github.com/prateek/w`.
- PAL `codereview` currently errors unless you pass `--relevant-files` (comma-separated **absolute** paths).
- PAL `codereview` external validation may time out or model-mismatch; `--model o4-mini --thinking-mode minimal` has worked.
- PAL `continuation_id` flows generally don’t resume across separate `pal-mcporter` invocations (fresh server per call).
- `codex review` may spam opentelemetry export errors to `http://localhost:14318/v1/logs`; if it hangs, kill the spawned `codex` process and proceed with manual review.
- `codex review`: `--uncommitted` is mutually exclusive with `--base` (use one or the other).
- This environment may set `NO_COLOR=1`; Worktrunk snapshot tests expect ANSI output, so run with `NO_COLOR= CLICOLOR_FORCE=1` if you need to execute them.
