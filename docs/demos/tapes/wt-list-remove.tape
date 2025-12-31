Output "{{OUTPUT_GIF}}"

Require wt
Require git

Env GIT_TERMINAL_PROMPT "0"

Source docs/demos/tapes/shared-setup.tape

Type "clear"
Enter
Sleep 500ms
Show
Sleep 500ms

# Show worktrees
Type "wt list"
Enter
Sleep 2s

# Remove a merged worktree - clean and simple
Type "wt remove billing"
Enter
Sleep 2s

Type "exit"
Enter
Sleep 500ms
