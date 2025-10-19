# Arbor: Git Worktree Management

A Rust-based CLI tool for managing git worktrees with seamless shell integration.

## Features

- **Shell Integration**: Automatically `cd` to worktrees when switching
- **Multiple Shells**: Supports Bash, Fish, and Zsh
- **Customizable**: Configure command prefix and hook behavior
- **Fast**: Built in Rust for performance
- **Clean Design**: Uses the proven "eval init" pattern from tools like zoxide and starship

## Installation

```bash
cargo build --release
# Copy target/release/arbor to a directory in your PATH
```

## Shell Integration Setup

Arbor uses shell integration to automatically change directories when switching worktrees. Add one of the following to your shell config:

### Bash

Add to `~/.bashrc`:
```bash
eval "$(arbor init bash)"
```

### Fish

Add to `~/.config/fish/config.fish`:
```fish
arbor init fish | source
```

### Zsh

Add to `~/.zshrc`:
```bash
eval "$(arbor init zsh)"
```

## Usage

### Basic Commands

```bash
# List all worktrees
arbor list

# Switch to a worktree (creates if doesn't exist)
arbor-switch feature-branch

# Finish current worktree and return to primary
arbor-finish

# Push changes between worktrees
arbor push target-worktree

# Merge and cleanup
arbor merge main --squash
```

### Customization

**Custom command prefix:**
```bash
# Use 'wt' instead of 'arbor'
eval "$(arbor init bash --cmd wt)"

# Now use: wt-switch, wt-finish, etc.
```

**Enable prompt hook:**
```bash
# Track worktree changes in your prompt
eval "$(arbor init bash --hook prompt)"
```

**Disable aliases:**
```bash
# Don't create short aliases like arbor-sw, arbor-fin
eval "$(arbor init bash --no-alias)"
```

## How It Works

Arbor uses a **directive protocol** to communicate with shell wrappers:

1. Shell wrapper calls `arbor switch --internal my-branch`
2. Arbor outputs special directives mixed with regular output:
   ```
   __ARBOR_CD__/path/to/worktree
   Switched to worktree: my-branch
   ```
3. Shell wrapper parses output, executes `cd` for directives, displays other lines

This separation keeps the Rust binary focused on git logic while letting the shell handle directory changes.

## Development Status

Current implementation:

- ✅ Shell integration infrastructure (eval init pattern)
- ✅ Template-based shell code generation (Askama)
- ✅ Directive protocol (__ARBOR_CD__)
- ✅ Basic CLI structure
- ⏳ Git primitives (coming next)
- ⏳ Worktree operations (coming next)
- ⏳ Advanced features (push, merge, etc.)

See [TODO.md](TODO.md) for detailed roadmap.

## Architecture

```
arbor (Rust binary)
├── Core commands (work standalone)
│   ├── arbor list
│   ├── arbor remove
│   └── arbor status
├── Internal commands (for shell wrapper)
│   ├── arbor switch --internal → outputs __ARBOR_CD__ directives
│   ├── arbor finish --internal → outputs __ARBOR_CD__ directives
│   └── arbor hook prompt → for prompt integration
└── Shell integration
    └── arbor init <shell> → outputs shell wrapper functions
```

## Design Principles

- **Progressive Enhancement**: Works without shell integration, better with it
- **One Canonical Path**: No options, no configuration unless explicitly needed
- **Fast**: Keep shell integration code minimal (<500ms execution time)
- **Stateless**: Binary doesn't maintain state, shell handles environment

## Inspiration

Arbor's shell integration pattern is inspired by successful tools:

- **zoxide**: Smarter cd with frequency tracking
- **starship**: Cross-shell prompt customization
- **direnv**: Per-directory environment variables
- **pyenv**: Python version management with shims

## License

MIT (or your preferred license)
