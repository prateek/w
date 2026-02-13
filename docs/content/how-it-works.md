+++
title = "How it works"
template = "page.html"
+++

`w` is a multi-repo wrapper that reuses Worktrunk (`wt`) semantics for creating, switching, and removing worktrees.

## Worktree identity (`project_identifier`)

Worktrunk computes a canonical `project_identifier` for each repo (typically `host/owner/name`, including nested groups).

`w` uses it for:

- namespacing centralized worktree directories (via Worktrunk’s `worktree-path` template)
- stable list/switch output across repos (`w ls`, `w switch`)

This value is credential-safe (userinfo is stripped from remote URLs).

## Centralized layout (Worktrunk `worktree-path`)

`w` does not invent a new worktree layout; it defers to Worktrunk’s `worktree-path` template in `~/.config/worktrunk/config.toml`:

```toml
worktree-path = "~/code/wt/{{ project_identifier | sanitize }}/{{ branch | sanitize }}"
```

The `sanitize` filter makes identifiers safe for filesystem paths.

## Repo discovery + index

Multi-repo commands scan the roots in `~/.config/w/config.toml` (or repeated `--root` flags):

```toml
repo_roots = ["~/code/github.com"]
max_depth = 6
```

Scans are cached (default: `~/.cache/w/repo-index.json`). Commands that reuse the cache (`w ls`, `w switch`, `w repo pick`) accept `--refresh` to force a rescan.

`w repo index` always scans unless you pass `--cached`, and can output `--format json|tsv` for scripting/debugging.

## Cross-repo concurrency

Commands that fan out over many repos (`w ls`, `w switch`) run per-repo jobs with bounded concurrency:

- config: `max_concurrent_repos` in `~/.config/w/config.toml`
- env: `W_MAX_CONCURRENT_REPOS` (cap: 32)
- per-command: `--jobs <n>`

Defaults are conservative (`min(available_parallelism, 4)`).

## Interactive pickers and TTY

`w repo pick` and `w switch` use `skim` for interactive selection. If you don’t have a TTY, use `--filter` to select non-interactively.

## Shell integration

A subprocess can’t `cd` your current shell. `w shell init <shell>` prints a small wrapper function that:

1. runs the real `w` binary
2. captures the printed path for `cd/new/switch`
3. changes directory in your current shell
