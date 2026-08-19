#!/usr/bin/env bash
# The release gate over the eval scorecard record (CONSTITUTION
# invariant 6; remediation Phase 4 item 3): a release cuts only when a
# committed scorecard under rfcs/scorecards/ is green and names the
# release-tip sha. CI verifies the record; the live eval itself is
# operator-invoked (emery-adapters `cargo make eval`) and never runs
# here — grading must not move into CI where a red run becomes a
# pressure to weaken the gate (the R3 lesson).
#
# Usage: scripts/scorecard-gate.sh <release-tip-sha>
set -euo pipefail

sha="${1:?usage: scorecard-gate.sh <release-tip-sha>}"
dir="rfcs/scorecards"

shopt -s nullglob
for card in "$dir"/*.md; do
  if grep -qx -- "- status: green" "$card" \
    && grep -qx -- "- emery-sha: $sha" "$card"; then
    echo "release gate: $card is green and names $sha"
    exit 0
  fi
done

echo "release gate: no green scorecard under $dir names $sha" >&2
echo "run the live eval (emery-adapters: cargo make eval) at this tip and commit its scorecard as $dir/<date>.md" >&2
exit 1
