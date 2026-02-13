# Working in this repo

Source of truth for the roadmap: `PRD.md`.

## Quick commands

```bash
cargo fmt --check
cargo clippy --workspace -- -D warnings
cargo test --workspace
```

## Notes

- Prefer small, reviewable diffs.
- Keep vendored upstream code changes upstreamable when we add `vendor/worktrunk/`.

## Gotchas (from iteration logs)

- `cargo new` creates a nested git repo by default; prefer `cargo new --vcs none ...` for new crates.
- PAL `codereview` requires `relevant_files` to be **absolute** paths when using `--raw`.
- `codex review` may spam opentelemetry export errors to `http://localhost:14318/v1/logs`; if it hangs, kill the spawned `codex` process and proceed with manual review.
