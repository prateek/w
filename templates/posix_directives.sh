# Shared directive parser for POSIX shells (bash, zsh, oil).
# Streams worktrunk's NUL-delimited output in real-time via FIFO while keeping
# stderr attached to the TTY for child processes (colors, progress bars).
#
# Note: Named without leading underscore to avoid being filtered by shell
# snapshot systems (e.g., Claude Code) that exclude private functions.
# Note: Uses ${_WORKTRUNK_CMD:-{{ cmd_prefix }}} fallback because shell snapshot
# systems may capture functions but not environment variables.
wt_exec() {
    # ┌─────────────────────────────────────────────────────────────────────────┐
    # │ BASH JOB CONTROL: Why we need TWO suppression mechanisms                │
    # │                                                                         │
    # │ Bash has two separate job notification mechanisms that are controlled   │
    # │ by DIFFERENT flags:                                                     │
    # │                                                                         │
    # │ 1. START notification "[1] 12345" (when `&` runs):                      │
    # │    - Controlled by `interactive` flag, NOT `monitor` (-m)               │
    # │    - Printed by describe_pid() via DESCRIBE_PID macro in jobs.c         │
    # │    - Cannot be disabled by any shell option in an interactive shell     │
    # │    - Must be suppressed via stderr redirection: { cmd & } 2>/dev/null   │
    # │                                                                         │
    # │ 2. DONE notification "[1]+ Done ..." (at next prompt):                  │
    # │    - Controlled by `monitor` (-m) option                                │
    # │    - Only printed for jobs started while monitor mode was ON            │
    # │    - Suppressed by `set +m` BEFORE backgrounding the job                │
    # │                                                                         │
    # │ References:                                                             │
    # │ - https://www.gnu.org/s/bash/manual/html_node/Job-Control-Basics.html   │
    # │ - https://superuser.com/questions/489722 (job notifications vs monitor) │
    # │ - https://stackoverflow.com/questions/20707316 (suppress background)    │
    # └─────────────────────────────────────────────────────────────────────────┘

    # Suppress DONE notifications by disabling monitor mode.
    # zsh: LOCAL_OPTIONS makes the change function-scoped (automatic restore)
    # bash: must manually save/restore since set +m affects the global shell
    # Note: We only touch -m, not -b (notify), to preserve user's notify preference.
    local _wt_saved_monitor=""
    if [[ -n "${ZSH_VERSION:-}" ]]; then
        setopt LOCAL_OPTIONS NO_MONITOR
    elif [[ -n "${BASH_VERSION:-}" && $- == *m* ]]; then
        _wt_saved_monitor=1
        set +m
    fi

    local exec_cmd="" chunk="" exit_code=0 tmp_dir="" fifo_path="" runner_pid=""

    # Cleanup handler for signals and normal exit
    _wt_cleanup() {
        # Kill background process if still running
        if [[ -n "$runner_pid" ]] && kill -0 "$runner_pid" 2>/dev/null; then
            kill "$runner_pid" 2>/dev/null || true
        fi
        # Remove temp files
        /bin/rm -f "$fifo_path" 2>/dev/null || true
        /bin/rmdir "$tmp_dir" 2>/dev/null || true
        # Restore bash job control if we disabled it
        [[ -n "$_wt_saved_monitor" ]] && set -m
    }

    # On SIGINT: cleanup and exit immediately with 130
    trap '_wt_cleanup; return 130' INT

    # Create temp directory with FIFO for streaming output
    tmp_dir=$(mktemp -d "${TMPDIR:-/tmp}/wt.XXXXXX") || {
        echo "Failed to create temp directory for worktrunk shim" >&2
        [[ -n "$_wt_saved_monitor" ]] && set -m
        return 1
    }
    fifo_path="$tmp_dir/stdout.fifo"

    if ! mkfifo "$fifo_path"; then
        echo "Failed to create FIFO for worktrunk shim" >&2
        /bin/rm -rf "$tmp_dir"
        [[ -n "$_wt_saved_monitor" ]] && set -m
        return 1
    fi

    # Run worktrunk in background, piping stdout to FIFO.
    # Backgrounding is required: the FIFO blocks until both ends connect, so we
    # must background the writer (wt) while the foreground reads from it.
    #
    # For bash: Suppress the START notification "[1] 12345" via fd redirection.
    # The job-start message is printed by bash's describe_pid() to stderr when
    # `&` executes. By wrapping in { } with 2>/dev/null, we catch that message
    # while preserving the command's own stderr via fd 9.
    #
    # This pattern is stable across bash 3.x-5.x: describe_pid() has printed to
    # stderr since early bash versions, and redirections on grouped commands
    # affecting the shell's own writes is standard POSIX behavior.
    if [[ -n "${BASH_VERSION:-}" ]]; then
        exec 9>&2  # Save real stderr to fd 9 for the command's errors
        { command "${_WORKTRUNK_CMD:-{{ cmd_prefix }}}" "$@" >"$fifo_path" 2>&9 & } 2>/dev/null
        runner_pid=$!
        exec 9>&-  # Close fd 9
    else
        command "${_WORKTRUNK_CMD:-{{ cmd_prefix }}}" "$@" >"$fifo_path" &
        runner_pid=$!
    fi

    # Parse directives as they stream in
    while IFS= read -r -d '' chunk || [[ -n "$chunk" ]]; do
        if [[ "$chunk" == __WORKTRUNK_CD__* ]]; then
            # Directory change directive
            local path="${chunk#__WORKTRUNK_CD__}"
            \cd "$path"
        elif [[ "$chunk" == __WORKTRUNK_EXEC__* ]]; then
            # Command execution directive (deferred until after worktrunk exits)
            exec_cmd="${chunk#__WORKTRUNK_EXEC__}"
        else
            # Regular output - print to stdout
            [[ -n "$chunk" ]] && printf '%s\n' "$chunk"
        fi
    done <"$fifo_path"

    # Wait for worktrunk to complete and capture its exit code.
    # Note: `wait $pid` works correctly even with monitor mode disabled (set +m).
    # Unlike `disown` which removes jobs from bash's tracking entirely, `set +m`
    # only disables job control features (fg/bg, DONE notifications) while still
    # allowing exit status retrieval via wait.
    wait "$runner_pid" >/dev/null 2>&1 || exit_code=$?

    # Cleanup
    trap - INT
    _wt_cleanup

    # Execute deferred command if specified (its exit code takes precedence)
    if [[ -n "$exec_cmd" ]]; then
        eval "$exec_cmd"
        exit_code=$?
    fi

    return "${exit_code:-0}"
}
