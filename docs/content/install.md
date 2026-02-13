+++
title = "Install"
template = "page.html"
+++

## Homebrew

This repo hosts a Homebrew tap. Until the first tagged release is published (and the formula has stable URLs), the formula is HEAD-only (tracks `main`):

```bash
brew tap prateek/w https://github.com/prateek/w
brew install --HEAD prateek/w/w
```

Once a tagged release is published, you can install without `--HEAD`.

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

Releases ship both `w` and a pinned `wt` build together. Once releases are published, Homebrew installs without `--HEAD` track the latest tagged release; use `--HEAD` (or install from source) for development builds from `main`.
