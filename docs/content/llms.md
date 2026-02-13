+++
title = "LLMs / Codex"
template = "page.html"
+++

This repo is an experimental, upstream-first wrapper around Worktrunk. Many changes are made with the help of LLM-based coding tools (including Codex), with human review.

## What we optimize for

- Small, reviewable diffs
- Tests and deterministic output contracts
- Upstreamable changes under `vendor/worktrunk/`

## If you use an LLM to contribute

1) Pick one task from `PRD.md` (highest priority) and work it end-to-end.
2) Keep changes scoped: avoid refactors and format churn, especially in `vendor/worktrunk/`.
3) Run checks locally:

```bash
cargo fmt --check
cargo clippy --workspace -- -D warnings
cargo test --workspace
```

For vendored Worktrunk:

```bash
cargo fmt --check --manifest-path vendor/worktrunk/Cargo.toml
cargo clippy --manifest-path vendor/worktrunk/Cargo.toml --workspace -- -D warnings
cargo test --manifest-path vendor/worktrunk/Cargo.toml --workspace --lib --bins
```

Docs:

```bash
cd docs && zola build
```

4) Update `PRD.md` (milestone progress) and append a line to `progress.md`.
5) Prefer explicit, descriptive commits.

## Safety

- Don’t paste secrets, tokens, or private code into prompts.
- Treat remote URLs and hook/template contexts as potentially sensitive; prefer redaction and add tests for it.
