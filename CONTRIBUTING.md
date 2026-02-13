# Contributing

Thanks for your interest in contributing!

## Repository layout

- `crates/w/`: the native Rust `w` wrapper (this repo’s code).
- `vendor/worktrunk/`: vendored upstream Worktrunk via `git subtree`.

## Local development

Run checks for `w`:

```bash
cargo fmt --check
cargo clippy --workspace -- -D warnings
cargo test --workspace
```

Run checks for the vendored Worktrunk subtree (prefer unit tests; snapshot-style tests are brittle
in a subtree):

```bash
cargo fmt --check --manifest-path vendor/worktrunk/Cargo.toml
cargo clippy --manifest-path vendor/worktrunk/Cargo.toml --workspace -- -D warnings
cargo test --manifest-path vendor/worktrunk/Cargo.toml --workspace --lib --bins
```

## Working with vendored Worktrunk

Changes under `vendor/worktrunk/` should be written as if they will be upstreamed:

- Keep diffs small and focused (avoid churn/refactors).
- Prefer clean commits that touch only `vendor/worktrunk/` when making upstreamable patches.

To sync/update the subtree or prepare an upstreamable branch, see `vendor/worktrunk/UPSTREAM.md`.
