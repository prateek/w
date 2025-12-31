Output "{{OUTPUT_GIF}}"

Require wt
Require git

Env GIT_TERMINAL_PROMPT "0"
Env ANTHROPIC_API_KEY "{{ANTHROPIC_API_KEY}}"

Source docs/demos/tapes/shared-setup.tape

# Pre-install marketplace plugin before recording
Type "claude plugin marketplace add anthropics/claude-plugins-official"
Enter
Sleep 5s
Type "clear"
Enter
Sleep 500ms
Show
Sleep 500ms

# Switch to existing worktree
Type "wt switch alpha"
Enter
Sleep 1.5s

# Create new worktree (runs post-start hooks)
Type "wt switch --create api"
Enter
Sleep 1.5s

# Create with specific command execution - launch Claude Code UI
Type "wt switch --execute claude --create dashboard"
Sleep 1s
Enter
Sleep 6s
