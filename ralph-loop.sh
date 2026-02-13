#!/bin/bash
set -e

# adapted from: https://gist.github.com/prateek/14fae59c71921710a3e055d74f30c8af
if [[ $# -lt 1 ]]; then
  echo "Usage: $0 <iterations>"
  exit 1
fi

if ! [[ "$1" =~ ^[0-9]+$ ]] || [[ "$1" -lt 1 ]]; then
  echo "Iterations must be a positive integer"
  exit 1
fi

PROMISE_FILE="I_PROMISE_ALL_TASKS_IN_THE_PRD_ARE_DONE_I_AM_NOT_LYING_I_SWEAR"

mkdir -p .logs
rm -f "$PROMISE_FILE"

for ((i = 1; i <= $1; i++)); do
  ITERATION_LOG=".logs/iteration-${i}.log"
  rm -f "$ITERATION_LOG"
  START_SHA="$(git rev-parse HEAD 2>/dev/null || true)"

  ITERATION_LOG="$ITERATION_LOG" codex --dangerously-bypass-approvals-and-sandbox exec <<'EOF' 2>&1 | tee -a ".logs/iterations.log" "$ITERATION_LOG"
1. Find the highest-priority task based on PRD.md and progress.md, and implement it.
2. Run your tests and type checks.
3. Do a tight loop of: code-simplifier -> code-review -> address issues (repeat until you have reasonable confidence, but cap at 3 loops; re-run relevant tests/type checks after fixes).
4. Update PRD.md with what was done.
5. Append your progress to progress.md.
6. Review the logs of what you just did (the current iteration log path is in the ITERATION_LOG env var) and make any necessary updates to AGENTS.md (create it if missing) so future iterations go better.
7. Commit your changes.
8. Push the commit to origin (set upstream if needed).
ONLY WORK ON A SINGLE TASK.

If the PRD is complete, and there are NO tasks left, then and only then touch a file named I_PROMISE_ALL_TASKS_IN_THE_PRD_ARE_DONE_I_AM_NOT_LYING_I_SWEAR. Otherwise respond with a brief summary of changes/progress.
EOF

  if [[ -f "$PROMISE_FILE" ]]; then
    echo "PRD complete after $i iterations."
    exit 0
  fi

  END_SHA="$(git rev-parse HEAD 2>/dev/null || true)"
  if [[ -n "$END_SHA" && "$START_SHA" != "$END_SHA" ]]; then
    git push -u origin HEAD
  fi
done

echo "PRD not complete after $1 iterations."
exit 1
