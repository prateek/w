# Debugging Interactive Terminal Commands

When debugging TUI commands like `wt beta select`, use the `tmux-cli` skill (preferred) or MCP's `node-terminal` tools to test interactively.

## Debugging Workflow

### 1. Create Test Environment

```console
cargo run --bin setup-select-test
```

This creates a reproducible test repo at `/tmp/wt-select-test/test-repo`.

### 2. Test Interactively

#### Option A: tmux-cli skill (preferred, if available)

Load the `tmux-cli` skill, then use the `tmux-cli` tool. Install if needed: `uv tool install claude-code-tools` (requires tmux).

```console
# Launch shell in test repo
pane=$(tmux-cli launch "zsh")
tmux-cli send "cd /tmp/wt-select-test/test-repo" --pane=$pane
tmux-cli wait_idle --pane=$pane

# Run with debug logging
tmux-cli send "RUST_LOG=worktrunk=debug cargo run --quiet -- beta select 2> debug.log" --pane=$pane
tmux-cli wait_idle --pane=$pane

# Test interaction (e.g., select option 3)
tmux-cli send "3" --pane=$pane
tmux-cli wait_idle --pane=$pane

# Capture output
tmux-cli capture --pane=$pane
```

#### Option B: MCP node-terminal

MCP terminals use pseudo-TTY, not real terminals. If tests pass in MCP but users report issues, the bug is likely environment-specific. Always test on the actual problematic repository.

```typescript
// Create terminal and navigate to test repo
mcp__node-terminal__terminal_create({ sessionId: "test" })
mcp__node-terminal__terminal_write({ sessionId: "test", input: "cd /tmp/wt-select-test/test-repo" })
mcp__node-terminal__terminal_send_key({ sessionId: "test", key: "enter" })

// Run with debug logging
mcp__node-terminal__terminal_write({
  sessionId: "test",
  input: "RUST_LOG=worktrunk=debug cargo run --quiet -- beta select 2> debug.log"
})
mcp__node-terminal__terminal_send_key({ sessionId: "test", key: "enter" })

// Test the interaction
mcp__node-terminal__terminal_write({ sessionId: "test", input: "3" })
mcp__node-terminal__terminal_read({ sessionId: "test" })
```

### 3. Analyze Logs

```console
tail -100 debug.log | grep -E "error|hang|stuck"
```

## Important Flags

- **`-C <path>`**: Set working directory (alternative to `cd`)
- **`--source`**: Use local source (only needed with installed `wt`, not with `cargo run`)

```console
# Testing with cargo run (already uses local source):
cargo run --quiet -- -C /path/to/repo beta select

# Testing with installed wt:
wt --source -C /path/to/repo beta select
```

## Shell Completion for CLI Arguments

Branch and worktree arguments should include shell completion for better UX. Add completion helpers to CLI definitions:

```rust
/// Target branch (defaults to current)
#[arg(long, add = crate::completion::branch_value_completer())]
branch: Option<String>,
```

**Available completers:**
- `branch_value_completer()` - Completes with branch names
- `worktree_branch_completer()` - Completes with worktree paths and branch names

**Pattern:** All branch arguments should use `branch_value_completer()` for consistency with commands like `wt merge`, `wt switch --base`, `wt rebase`.
