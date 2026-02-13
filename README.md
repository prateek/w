# `w`

Experimental multi-repo wrapper for [Worktrunk](https://github.com/max-sixty/worktrunk).

This repo is a downstream sandbox for exploring a native Rust `w` UX while keeping upstream
Worktrunk (`wt`) focused on single-repo workflows.

## Status

Early bootstrap. See `PRD.md` for goals and milestones.

## Demo

- Docs page: `https://prateek.github.io/w/demo/`
- Play locally: `asciinema play docs/static/demos/w-basic.cast`

## Docs

- Home: `https://prateek.github.io/w/`
- Install: `https://prateek.github.io/w/install/`
- Quickstart: `https://prateek.github.io/w/quickstart/`
- Commands: `https://prateek.github.io/w/commands/`
- LLMs / Codex: `https://prateek.github.io/w/llms/`

## Installation (Homebrew)

This repo hosts a Homebrew tap. For now, the formula is HEAD-only (tracks `main`).

```bash
brew tap prateek/w https://github.com/prateek/w
brew install --HEAD prateek/w/w
```

## Attribution

This project vendors upstream Worktrunk under `vendor/worktrunk/`. See `NOTICE` for attribution and licensing details.

## LLM assistance

This repo is developed with the help of LLM-based coding tools (including Codex), with human review. See the docs page: `https://prateek.github.io/w/llms/`.

## Development

Run checks for `w`:

```bash
cargo fmt --check
cargo clippy --workspace -- -D warnings
cargo test --workspace
```

## Docs (local)

This repo includes a minimal docs site built with Zola:

```bash
cd docs
zola serve
```

If you don’t have `zola` installed, see `AGENTS.md` for a Docker command.
