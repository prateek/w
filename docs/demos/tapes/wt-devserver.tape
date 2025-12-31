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

# Show the config: post-start hooks + URL template
Type "cat .config/wt.toml"
Enter
Sleep 2s

# Create a new worktree - dev server starts automatically
Type "wt switch --create feature"
Enter
Sleep 3s

# wt list shows the URL column
Type "wt list"
Enter
Sleep 4s

Type "exit"
Enter
Sleep 500ms
