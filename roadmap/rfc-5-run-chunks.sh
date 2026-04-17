#!/usr/bin/env bash
#
# rfc-5-run-chunks.sh -- drive RFC-5 chunk execution, one fresh agent context
# per chunk, until every chunk in roadmap/rfc-5-tasks.md is marked [x].
#
# Each iteration:
#   1. Parses roadmap/rfc-5-tasks.md for the first status-table row whose
#      Status cell is "[ ]" (pending) -- file order is already topologically
#      valid, so "first pending" is always a safe next pick.
#   2. Invokes `cursor-agent -p` with a prompt that tells the agent which
#      chunk to execute and, critically, NOT to self-mark [x] if verification
#      fails. That makes the post-run gate below trustworthy.
#   3. After the agent exits, re-reads the file and confirms that chunk is
#      now [x]. If it isn't, the loop stops so you can inspect before the
#      next context spins up on top of a half-applied state.
#   4. Also refuses to start the next chunk if the working tree is dirty
#      (the per-chunk rule is exactly one commit on `vectis-cli`, already
#      pushed, so a clean tree between chunks is the invariant).
#
# Requirements:
#   - `cursor-agent` on PATH (Cursor CLI, headless "print" mode).
#   - Run from the repo root or anywhere inside it; the script cd's to the
#     repo root before doing anything.
#
# Optional env vars:
#   CURSOR_AGENT_ARGS   Extra args passed to cursor-agent (e.g. --model ...).
#   MAX_CHUNKS          Stop after this many successful chunks (default: run
#                       until none remain). Useful for dry runs.
#   SKIP_CLEAN_CHECK=1  Don't bail on a dirty working tree between chunks.

set -euo pipefail

REPO_ROOT=$(git rev-parse --show-toplevel)
cd "$REPO_ROOT"

TASKS_FILE="roadmap/rfc-5-tasks.md"
RFC_FILE="roadmap/rfc-5-vectis-bootstrap.md"

if [[ ! -f "$TASKS_FILE" || ! -f "$RFC_FILE" ]]; then
  echo "error: expected $TASKS_FILE and $RFC_FILE to exist" >&2
  exit 2
fi

if ! command -v cursor-agent >/dev/null 2>&1; then
  echo "error: cursor-agent not on PATH. Install the Cursor CLI and retry." >&2
  exit 2
fi

# Extract the chunk id (col 2) from the first status row whose status cell
# (col 4) contains "[ ]". Chunk ids look like 1, 2, 3a, 3b, 10, ...
next_pending_chunk() {
  awk -F'|' '
    /^\| *[0-9][0-9a-z]* *\|/ {
      status = $4
      gsub(/^[ \t]+|[ \t]+$/, "", status)
      if (status == "[ ]") {
        id = $2
        gsub(/^[ \t]+|[ \t]+$/, "", id)
        print id
        exit
      }
    }
  ' "$TASKS_FILE"
}

# Status of a specific chunk id ("[ ]" or "[x]" or empty if not found).
chunk_status() {
  local id=$1
  awk -F'|' -v want="$id" '
    /^\| *[0-9][0-9a-z]* *\|/ {
      cid = $2
      gsub(/^[ \t]+|[ \t]+$/, "", cid)
      if (cid == want) {
        status = $4
        gsub(/^[ \t]+|[ \t]+$/, "", status)
        print status
        exit
      }
    }
  ' "$TASKS_FILE"
}

ensure_clean_tree() {
  if [[ "${SKIP_CLEAN_CHECK:-0}" == "1" ]]; then
    return 0
  fi
  if ! git diff --quiet || ! git diff --cached --quiet; then
    echo "error: working tree is dirty; refusing to start the next chunk." >&2
    echo "       commit/stash/reset first, or set SKIP_CLEAN_CHECK=1." >&2
    git status --short >&2
    exit 1
  fi
}

build_prompt() {
  local chunk=$1
  cat <<EOF
Consider @${RFC_FILE} and @${TASKS_FILE}. Execute Chunk ${chunk}, then record that completion and update any future chunks if they are impacted by anything you've discovered during execution.

Gate: if any of the chunk's verification commands fail, do NOT mark the chunk [x]. Leave it [ ] and record what you observed in that chunk's Notes column so the next agent can pick up from a correct state. Only flip the status cell to [x] after every verification command in the chunk has passed.
EOF
}

run_one_chunk() {
  local chunk=$1
  local prompt
  prompt=$(build_prompt "$chunk")

  echo "=== $(date '+%H:%M:%S') running chunk ${chunk} ==="
  # shellcheck disable=SC2086  # intentional word-split of CURSOR_AGENT_ARGS
  cursor-agent -p "$prompt" ${CURSOR_AGENT_ARGS:-} --trust
}

processed=0
max=${MAX_CHUNKS:-0}

while :; do
  ensure_clean_tree

  next=$(next_pending_chunk || true)
  if [[ -z "${next}" ]]; then
    echo "=== all chunks marked [x]; done. ==="
    break
  fi

  if [[ "$max" -gt 0 && "$processed" -ge "$max" ]]; then
    echo "=== reached MAX_CHUNKS=${max}; stopping with chunk ${next} still pending. ==="
    break
  fi

  run_one_chunk "$next"

  # Gate: the agent must have flipped the row to [x]. If it didn't, either
  # verification failed (agent followed instructions and left it [ ]) or
  # something went wrong mid-run. Either way, stop -- don't start the next
  # chunk on top of an unverified state.
  post=$(chunk_status "$next")
  if [[ "$post" != "[x]" ]]; then
    echo "error: chunk ${next} did not self-mark [x] (status is '${post:-<missing>}')." >&2
    echo "       inspect $TASKS_FILE notes + git log, then rerun this script." >&2
    exit 1
  fi

  processed=$((processed + 1))
  echo "=== chunk ${next} complete (${processed} done this run) ==="
done
