# arbor shell integration for fish

# Helper function to parse arbor output and handle directives
function _arbor_exec
    set -l output (arbor $argv 2>&1)
    set -l exit_code $status

    # Parse output line by line
    for line in $output
        if string match -q '__ARBOR_CD__*' -- $line
            # Extract path and change directory
            cd (string sub -s 14 -- $line)
        else
            # Regular output - print it
            echo $line
        end
    end

    return $exit_code
end

# Main commands that support directory changes
function {{ cmd_prefix }}-switch
    _arbor_exec switch --internal $argv
end

function {{ cmd_prefix }}-finish
    _arbor_exec finish --internal $argv
end

# Convenience aliases
alias {{ cmd_prefix }}-sw='{{ cmd_prefix }}-switch'
alias {{ cmd_prefix }}-fin='{{ cmd_prefix }}-finish'

{% if hook.to_string() == "prompt" %}
# Prompt hook for tracking current worktree
function _arbor_prompt_hook --on-event fish_prompt
    # Call arbor to update tracking
    command arbor hook prompt 2>/dev/null; or true
end
{% endif %}
