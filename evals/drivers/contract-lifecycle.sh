#!/usr/bin/env bash
# contract-lifecycle execute-phase replay — operator aid, not CI.
# Plan setup and Gate 1 remain agent-driven per evals/scenarios/contract-lifecycle.md.
# Run under an operator-held session lock: `specify plan lock -- bash contract-lifecycle.sh`.
set -euo pipefail
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib.sh"

SANDBOX="${SPECIFY_SANDBOX:-$SANDBOX_ROOT/contract-lifecycle}"
WS="${SPECIFY_WS:-$SANDBOX/platform}"
CONTRACT_FIX="$FIXTURES_DIR/contract-lifecycle"

drive_slice() {
  local slice="$1" project="$2" slot="$WS/workspace/$project"
  run "$WS" plan next
  run "$WS" slice synthesize "$slice"
  run "$WS" slice validate "$slice"
  run "$WS" slice transition "$slice" refined
  run "$WS" slice build "$slice" --phase prepare
  cp -R "$CONTRACT_FIX/." "$slot/"
  run "$WS" slice build "$slice" --phase finalize
  run "$WS" slice merge run "$slice"
}

run_execute_loop() {
  local action slice
  while true; do
    action="$(status_action "$WS")"
    case "$action" in
      drained) echo drained; return 0 ;;
      stop) echo "stop: $(status_stop "$WS")"; return 1 ;;
    esac
    slice="$(status_slice "$WS")"
    case "$slice" in
      oauth-contract) drive_slice oauth-contract contracts ;;
      oauth-backend)  drive_slice oauth-backend backend ;;
      oauth-mobile)   drive_slice oauth-mobile mobile ;;
      *) echo "unexpected slice: $slice" >&2; return 1 ;;
    esac
  done
}

if [ "${BASH_SOURCE[0]}" = "$0" ]; then
  case "${1:-execute}" in
    execute) require_tools; run_execute_loop ;;
    *)
      echo "usage: $(basename "$0") [execute]" >&2
      echo "  Runs the approved-plan execute loop against SPECIFY_WS (default: $WS)." >&2
      exit 2
      ;;
  esac
fi
