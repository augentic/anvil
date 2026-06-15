#!/usr/bin/env bash
# contract-lifecycle execute-phase replay — operator aid, not CI.
# Plan setup and Gate 1 remain agent-driven per evals/scenarios/contract-lifecycle.md.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
SANDBOX="${SPECIFY_SANDBOX:-$ROOT/evals/.sandbox/contract-lifecycle}"
WS="${SPECIFY_WS:-$SANDBOX/platform}"
CLI_FIX="${SPECIFY_CLI_FIX:-$ROOT/../specify-cli/tests/fixtures/fan-in-fan-out}"
SPECIFY="${SPECIFY_BIN:-specify}"

run() {
  (cd "$WS" && "$SPECIFY" "$@")
}

drive_slice() {
  local slice="$1"
  local project="$2"
  local slot="$WS/workspace/$project"

  run plan next
  run slice synthesize "$slice"
  run slice validate "$slice"
  run slice transition "$slice" refined
  run slice build "$slice" --phase prepare
  cp -R "$CLI_FIX/." "$slot/"
  run slice build "$slice" --phase finalize
  run slice merge run "$slice"
}

run_execute_loop() {
  while true; do
    local status action slice
    status="$(run plan status --format json)"
    action="$(python3 -c 'import json,sys; print(json.load(sys.stdin).get("action",""))' <<<"$status")"
    if [[ "$action" == "drained" ]]; then
      echo drained
      return 0
    fi
    if [[ "$action" == "stop" ]]; then
      echo "$status"
      return 1
    fi
    slice="$(python3 -c 'import json,sys; d=json.load(sys.stdin); print(d.get("slice") or d.get("entry",""))' <<<"$status")"
    case "$slice" in
      oauth-contract) drive_slice oauth-contract contracts ;;
      oauth-backend)  drive_slice oauth-backend backend ;;
      oauth-mobile)   drive_slice oauth-mobile mobile ;;
      *) echo "unexpected slice: $slice" >&2; return 1 ;;
    esac
  done
}

if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
  case "${1:-execute}" in
    execute) run_execute_loop ;;
    *)
      echo "usage: $0 [execute]" >&2
      echo "  Runs the approved-plan execute loop against SPECIFY_WS (default: $WS)." >&2
      exit 2
      ;;
  esac
fi
