#!/usr/bin/env bash

# Print a read-only snapshot of an acceptance run's artifacts: the sandbox
# directory tree plus the bodies of the key Specify artifacts and the rendered
# slice models. Paste the output into the "Artefact snapshot" section of a run
# record (acceptance/shared/run-summary-template.md).
#
# This is evidence capture only — it drives no /spec:* command, runs only
# read-only `specify` verbs, and asserts nothing. It is not a scenario runner.
#
# Usage: bash ./scripts/acceptance-snapshot.sh <sandbox-dir>
#   e.g. bash ./scripts/acceptance-snapshot.sh acceptance/.sandbox/pure-intent

set -euo pipefail

ROOT="${1:-.}"

if [ ! -d "$ROOT" ]; then
  echo "Error: sandbox directory not found: $ROOT" >&2
  exit 1
fi

ROOT="$(cd "$ROOT" && pwd)"

section() {
  echo ""
  echo "===== $* ====="
}

dump() {
  # dump <relative-path-from-ROOT>
  local f="$ROOT/$1"
  if [ -f "$f" ]; then
    section "$1"
    cat "$f"
  fi
}

section "tree: $ROOT"
if command -v tree >/dev/null 2>&1; then
  # Skip noise that bloats the snapshot without aiding the mental model.
  tree -a -I '.git|target|node_modules' "$ROOT"
else
  # Portable fallback when `tree` is not installed.
  ( cd "$ROOT" && find . -path ./.git -prune -o -path '*/target' -prune -o -path '*/node_modules' -prune -o -print | sort )
fi

# Plan + change live at the project root today (path drift, see findings); also
# probe under .specify/ so the snapshot stays correct if the location moves.
for rel in plan.yaml discovery.md registry.yaml topology.lock \
           .specify/plan.yaml .specify/discovery.md .specify/registry.yaml; do
  dump "$rel"
done

if [ -f "$ROOT/.specify/journal.jsonl" ]; then
  section ".specify/journal.jsonl"
  cat "$ROOT/.specify/journal.jsonl"
fi

# Render each slice's model through the read-only viewer.
SLICES_DIR="$ROOT/.specify/slices"
if [ -d "$SLICES_DIR" ]; then
  for slice_path in "$SLICES_DIR"/*/; do
    [ -d "$slice_path" ] || continue
    slice="$(basename "$slice_path")"
    section "specify slice model show $slice"
    ( cd "$ROOT" && specify slice model show "$slice" ) || echo "(slice model show failed for $slice)"
  done
fi
