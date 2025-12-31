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

# Switch to branch with uncommitted changes
Type "wt switch hooks"
Enter
Sleep 1s

# Show staged changes
Type "git diff --staged --stat"
Enter
Sleep 2s

# Use wt step commit to generate message via LLM
Type "wt step commit"
Enter
Sleep 4s

# Show the result
Type "git log -1"
Enter
Sleep 3s

Type "exit"
Enter
Sleep 500ms
