# `w`

Experimental multi-repo wrapper for [Worktrunk](https://github.com/max-sixty/worktrunk).

This repo is a downstream sandbox for exploring a native Rust `w` UX while keeping upstream
Worktrunk (`wt`) focused on single-repo workflows.

## Status

Early bootstrap. See `PRD.md` for goals and milestones.

## Installation (Homebrew)

This repo hosts a Homebrew tap. For now, the formula is HEAD-only (tracks `main`).

```bash
brew tap prateek/w https://github.com/prateek/w
brew install --HEAD prateek/w/w
```

## Development

Run checks for `w`:

```bash
cargo fmt --check
cargo clippy --workspace -- -D warnings
cargo test --workspace
```

## Docs

This repo includes a minimal docs site built with Zola:

```bash
cd docs
zola serve
```
