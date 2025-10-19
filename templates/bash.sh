# arbor shell integration for {{ shell_name }}

# Helper function to parse arbor output and handle directives
_arbor_exec() {
    local output line exit_code
    output="$("arbor" "$@" 2>&1)"
    exit_code=$?

    # Parse output line by line
    while IFS= read -r line; do
        if [[ "$line" == __ARBOR_CD__* ]]; then
            # Extract path and change directory
            \cd "${line#__ARBOR_CD__}"
        else
            # Regular output - print it
            echo "$line"
        fi
    done <<< "$output"

    return $exit_code
}

# Main commands that support directory changes
{{ cmd_prefix }}-switch() {
    _arbor_exec switch --internal "$@"
}

{{ cmd_prefix }}-finish() {
    _arbor_exec finish --internal "$@"
}

# Convenience aliases
alias {{ cmd_prefix }}-sw='{{ cmd_prefix }}-switch'
alias {{ cmd_prefix }}-fin='{{ cmd_prefix }}-finish'

{% if hook.to_string() == "prompt" %}
# Prompt hook for tracking current worktree
_arbor_prompt_hook() {
    # Call arbor to update tracking
    command arbor hook prompt 2>/dev/null || true
}

# Add to PROMPT_COMMAND
if [[ -z "${PROMPT_COMMAND}" ]]; then
    PROMPT_COMMAND="_arbor_prompt_hook"
else
    PROMPT_COMMAND="${PROMPT_COMMAND}; _arbor_prompt_hook"
fi
{% endif %}
