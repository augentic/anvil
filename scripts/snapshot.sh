#!/usr/bin/env bash

# Print a read-only snapshot of an eval run's artifacts: the sandbox
# directory tree plus the bodies of the key Specify artifacts and the rendered
# slice models. Reference from the **Evidence** section of a run record
# (evals/shared/run-template.md); paste output on fail only.
#
# This is evidence capture only — it drives no /spec:* command, runs only
# read-only `specify` verbs, and asserts nothing. It is not a scenario runner.
#
# Usage: bash ./scripts/snapshot.sh <sandbox-dir>
#   e.g. bash ./scripts/snapshot.sh evals/.sandbox/intent-only

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

# Plan + change are operator-facing artifacts at the project root (canonical;
# see decision-log "F4"). The .specify/ probes are belt-and-suspenders only.
for rel in plan.yaml change.md discovery.md registry.yaml topology.lock \
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
