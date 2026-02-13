+++
title = "Commands"
template = "page.html"
+++

This page is a practical reference for the `w` CLI.

Notes:

- `w` uses Worktrunk config and semantics for worktrees.
- Many commands accept Worktrunk branch symbols like `@`, `-`, `^`.
- `w repo pick` and `w switch` are interactive by default and require a TTY:
  - `w repo pick`: TTY on stdin and stdout
  - `w switch`: TTY on stdin
  Use `--filter` for non-interactive use.

## Global options

### `-C, --repo <PATH>`

Operate on a specific repository (like `git -C`):

```bash
w -C ~/code/github.com/org/repo ls
w -C ~/code/github.com/org/repo new feature-branch
```

## Worktrees

### `w new <branch>`

Create a worktree for a branch, or switch if it already exists.

```bash
w -C /path/to/repo new my-branch
w -C /path/to/repo new my-branch --base main
```

Options:

- `--base <ref>`: base ref used when creating the branch.
- `--clobber`: move aside a pre-existing directory at the computed worktree path.
- `--print`: print the resolved path (even with shell integration enabled).

### `w cd <branch>`

Switch to a worktree for an existing branch and print its path.

```bash
w -C /path/to/repo cd my-branch
```

Options:

- `--print`: print the resolved path (even with shell integration enabled).

### `w run <branch> -- <cmd...>`

Switch/create a worktree, then run a command in it.

```bash
w -C /path/to/repo run my-branch -- cargo test
w -C /path/to/repo run my-branch --base main -- cargo fmt
```

Options:

- `--base <ref>`: base ref used when creating the branch.
- `--clobber`: move aside a pre-existing directory at the computed worktree path.

### `w rm <branch>`

Remove a worktree for a branch (keeps the branch).

```bash
w -C /path/to/repo rm my-branch
w -C /path/to/repo rm my-branch --force
```

### `w prune`

Remove stale worktree directories under the configured worktree root.

```bash
w -C /path/to/repo prune
```

## Multi-repo

Multi-repo commands use `~/.config/w/config.toml` by default. You can override discovery with repeated `--root` flags.

### `w ls`

List worktrees across repositories.

```bash
w ls
w ls --format json
w ls --format tsv
```

Options:

- `--format text|json|tsv` (default: `text`)
- `--preset compact|default|full`: text preset (only applies to `--format text`; can also be set via `[ls].preset` in config)
- `--sort repo|project|path`: sort order for output (can also be set via `[ls].sort` in config)
- `--jobs <n>`: max repositories to process concurrently (overrides config/env)
- `--include-prunable`: include worktrees that are prunable (directory missing but metadata still present)
- Indexing: `--cached` (cache-only) / `--refresh` (force rescan) / `--cache-path <path>`
- Discovery: `--config <path>` / `--root <path>` (repeatable) / `--max-depth <n>`

### `w switch`

Pick a worktree across repositories and print its path.

```bash
w switch
w switch --filter my-repo
```

Options:

- `--filter <text>`: non-interactively select the first match (substring match on project identifier, repo path, branch, or worktree path)
- `--print`: print the resolved path (even with shell integration enabled).
- `--jobs <n>`: max repositories to process concurrently (overrides config/env)
- `--include-prunable`: include worktrees that are prunable
- Indexing/discovery options are the same as `w ls`

### `w repo index`

Build and print the repository index.

```bash
w repo index
w repo index --format tsv
```

### `w repo pick`

Pick a repository and print its path.

```bash
w repo pick
w repo pick --filter my-repo
```

## Shell integration

### `w shell init <shell>`

Print an init snippet for shell integration:

```bash
eval "$(w shell init zsh)"
```

Supported shells: `zsh`, `bash`, `fish`, `pwsh`.

Notes:

- With shell integration enabled, `w cd/new/switch` will change your current directory.
- Pass `--print` (or use `command w …`) to bypass the directory change and just print the path.
