#!/usr/bin/env bash
# contract-lifecycle execute-phase replay — operator aid, not CI.
# Plan setup and Gate 1 remain agent-driven per
# quality/runbooks/contract-lifecycle.md; this driver runs the guest-owned
# execute loop over the already-approved plan: `specify plan execute` claims
# each entry, routes it to its workspace slot, and drives refine -> build ->
# merge until the plan drains.
set -euo pipefail
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib.sh"

SANDBOX="${SPECIFY_SANDBOX:-$SANDBOX_ROOT/contract-lifecycle}"
WS="${SPECIFY_WS:-$SANDBOX/platform}"

run_execute_loop() {
  ensure_binary
  try "$WS" plan execute
  report_final "$WS"
}

if [ "${BASH_SOURCE[0]}" = "$0" ]; then
  case "${1:-execute}" in
    execute) require_tools; require_model; run_execute_loop ;;
    *)
      echo "usage: $(basename "$0") [execute]" >&2
      echo "  Runs the approved-plan execute loop against SPECIFY_WS (default: $WS)." >&2
      exit 2
      ;;
  esac
fi
