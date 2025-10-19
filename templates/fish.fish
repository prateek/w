# worktrunk shell integration for fish

# Helper function to parse wt output and handle directives
function _wt_exec
    set -l output (wt $argv 2>&1)
    set -l exit_code $status

    # Parse output line by line
    for line in $output
        if string match -q '__WORKTRUNK_CD__*' -- $line
            # Extract path and change directory
            cd (string sub -s 18 -- $line)
        else
            # Regular output - print it
            echo $line
        end
    end

    return $exit_code
end

# Main commands that support directory changes
function {{ cmd_prefix }}-switch
    _wt_exec switch --internal $argv
end

function {{ cmd_prefix }}-finish
    _wt_exec finish --internal $argv
end

# Convenience aliases
alias {{ cmd_prefix }}-sw='{{ cmd_prefix }}-switch'
alias {{ cmd_prefix }}-fin='{{ cmd_prefix }}-finish'

{% if hook.to_string() == "prompt" %}
# Prompt hook for tracking current worktree
function _wt_prompt_hook --on-event fish_prompt
    # Call wt to update tracking
    command wt hook prompt 2>/dev/null; or true
end
{% endif %}
