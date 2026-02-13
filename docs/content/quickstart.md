+++
title = "Quickstart"
template = "page.html"
+++

## 1) Enable a centralized worktree layout (Worktrunk config)

`w` uses Worktrunk’s config and semantics for creating/switching/removing worktrees.

To put worktrees in a centralized directory, configure Worktrunk’s `worktree-path` template (example):

```toml
# ~/.config/worktrunk/config.toml
worktree-path = "~/code/wt/{{ project_identifier | sanitize }}/{{ branch | sanitize }}"
```

If you have `wt` installed, `wt config create` is the easiest way to bootstrap a config file.

## 2) Configure repo discovery (w config)

Multi-repo commands (`w ls`, `w switch`, `w repo …`) scan directories you configure:

```toml
# ~/.config/w/config.toml
repo_roots = ["~/code/github.com"]
max_depth = 6
max_concurrent_repos = 4
```

You can override concurrency per command with `--jobs <n>`, or globally with `W_MAX_CONCURRENT_REPOS` (cap: 32).

The repo index cache defaults to `~/.cache/w/repo-index.json`.

## 3) Try it

Pick a repo:

```bash
w repo pick
```

List worktrees across repos:

```bash
w ls
w ls --format json
```

Switch to a worktree across repos:

```bash
w switch
w switch --filter my-repo
```

Switch/create a worktree in a specific repo:

```bash
w -C /path/to/repo new feature-branch
w -C /path/to/repo run feature-branch -- cargo test
```

Interactive `w repo pick` / `w switch` requires a TTY; for non-interactive use, pass `--filter`.

## 4) Shell integration (`w cd` / `w new` / `w switch`)

A subprocess can’t change your current shell directory, so `w` provides an init snippet:

```bash
eval "$(w shell init zsh)"
```

After that, `w cd …`, `w new …`, and `w switch …` will `cd` in your current shell. Use `command w …` to bypass the shell function and call the binary directly.
