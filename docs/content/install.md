+++
title = "Install"
template = "page.html"
+++

## Homebrew

This repo hosts a Homebrew tap.

Install the latest tagged release:

```bash
brew tap prateek/w https://github.com/prateek/w
brew install prateek/w/w
```

This installs both `w` and `wt`.

For a development build from `main`:

```bash
brew install --HEAD prateek/w/w
```

## From source (Cargo)

```bash
cargo install --locked --git https://github.com/prateek/w --bin w
```

Or, from a local checkout:

```bash
git clone https://github.com/prateek/w
cd w
cargo install --locked --path crates/w
```

## Releases

Releases ship both `w` and a pinned `wt` build together. Homebrew installs without `--HEAD` track the latest tagged release; use `--HEAD` (or install from source) for development builds from `main`.
