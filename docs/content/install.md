+++
title = "Install"
template = "page.html"
+++

## Homebrew

This repo hosts a Homebrew tap.

```bash
brew tap prateek/w https://github.com/prateek/w
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

Releases are intended to ship both `w` and a pinned `wt` build together. Until the first tagged release exists, use Homebrew `--HEAD` or install from source.
