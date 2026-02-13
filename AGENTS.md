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
```

## Notes

- Prefer small, reviewable diffs.
- Keep vendored upstream code changes upstreamable under `vendor/worktrunk/`.

## Gotchas (from iteration logs)

- `cargo new` creates a nested git repo by default; prefer `cargo new --vcs none ...` for new crates.
- PAL `codereview` requires `--relevant-files` to be **absolute** paths (even without `--raw`).
- PAL `continuation_id` flows generally don’t resume across separate `pal-mcporter` invocations (fresh server per call).
- `codex review` may spam opentelemetry export errors to `http://localhost:14318/v1/logs`; if it hangs, kill the spawned `codex` process and proceed with manual review.
- This environment may set `NO_COLOR=1`; Worktrunk snapshot tests expect ANSI output, so run with `NO_COLOR= CLICOLOR_FORCE=1` if you need to execute them.
