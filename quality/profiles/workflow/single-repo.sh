# shellcheck shell=bash
# Single-repo `specify plan execute` replay shared by the execute-* scenarios.
# The guest owns the refine -> build -> merge loop; this driver does the
# clerical setup, authors the plan from the scenario's intent fixture, stamps
# Gate 1 on the operator's behalf, and drives `plan execute` — resuming once
# per invocation when the loop parks (the operator fixes between legs).
# Sourced after the scenario sets SCENARIO / PLAN.

source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib.sh"

SANDBOX="$SANDBOX_ROOT/$SCENARIO"
FX="$FIXTURES_DIR/$SCENARIO"

# --- project lifecycle -------------------------------------------------------

# Non-model leg: init from the release-built omnia component (mirrored into
# the project component cache), write the project-root deployment manifest,
# and start a git history for merge residue.
setup_project() {
  ensure_binary
  rm -rf "$SANDBOX"
  mkdir -p "$SANDBOX"
  run "$SANDBOX" init "$(adapter_component omnia)" --name "$SCENARIO"
  write_manifest "$SANDBOX" source:intent target:omnia
  git -C "$SANDBOX" init -b main -q
}

author()  { run "$SANDBOX" plan author "$PLAN" --intent "$(cat "$FX/intent.txt")"; }
approve() { run "$SANDBOX" plan transition "$PLAN" approved --actor agent; }

# One execute leg: drained exits 0; a park (engineered build failure, an
# operator interrupt) leaves the entry in-progress for the resume leg.
execute() {
  try "$SANDBOX" plan execute
  report_final "$SANDBOX"
}

# --- orchestration -----------------------------------------------------------

main() {
  require_tools
  require_model
  setup_project
  author
  approve
  echo "== execute =="
  execute
}

resume() {
  require_tools
  require_model
  ensure_binary
  echo "== resume =="
  execute
}

dispatch() {
  case "${1:-main}" in
    setup) require_tools; setup_project ;;
    resume) resume ;;
    main|"") main ;;
    *) echo "usage: $(basename "$0") [setup|resume]" >&2; exit 2 ;;
  esac
}
