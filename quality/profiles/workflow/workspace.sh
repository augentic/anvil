#!/usr/bin/env bash
# Workspace `specify plan execute` replay for the multi-repo eval scenarios:
#   workspace-two-projects | workspace-fail-resume | workspace-stale-recovery
# The guest owns refine -> build -> merge and routes each entry to its
# materialised workspace slot; this driver does the clerical setup (workspace
# + registered projects + bare-repo remotes + the OAuth brief), authors the
# plan from the documentation binding, stamps Gate 1 on the operator's
# behalf, and drives `plan execute`. Operator aid; never CI.
#
# The fail/stale scenarios park mid-run (an engineered build failure or an
# operator Ctrl-C leaving a slot dirty); the operator triages per the
# scenario doc, then `bash $0 <scenario> resume` re-enters to drained.
#
# NOTE: workspace routing has no in-guest counterpart yet — `plan author` /
# `plan execute` at a workspace root exit with the typed
# `plan-author-workspace-unsupported` / `plan-execute-workspace-unsupported`
# refusal, so the model legs currently file as blocked; the `setup` leg and
# the typed refusal itself are still replayable.
set -euo pipefail
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib.sh"

PLAN_NAME="oauth-login"
DOC_KEY="${DOC_KEY:-brief}"
WFX="$FIXTURES_DIR/workspace"

resolve_target() {
  case "$1" in
    backend) echo omnia ;;
    mobile) echo vectis ;;
    contracts) echo contracts ;;
  esac
}

proj_desc() {
  case "$1" in
    backend) echo "Omnia backend service for OAuth token exchange, sessions, and provider integration." ;;
    mobile) echo "Vectis mobile client for OAuth sign-in UI, callback handling, and API consumption." ;;
    contracts) echo "Shared OAuth login API contracts for cross-repo consumption." ;;
  esac
}

git_commit_all() {
  git -C "$1" add -A >/dev/null 2>&1 || true
  git -C "$1" commit -q --no-gpg-sign -m "$2" >/dev/null 2>&1 || true
}

# --- workspace + plan setup --------------------------------------------------

setup_git_remotes() {
  local p root bare
  for p in "$@"; do
    root="$SANDBOX/$p"
    git -C "$root" init -b main -q || true
    git_commit_all "$root" "init $p"
    bare="$SANDBOX/$p-origin.git"
    git init --bare -q "$bare"
    git -C "$root" remote add origin "file://$bare" 2>/dev/null || git -C "$root" remote set-url origin "file://$bare"
    git -C "$root" push -q -u origin main
  done
}

# Non-model leg: registry-only workspace at $PLATFORM, per-project init from
# the release-built adapter components, bare-repo remotes, the registry, the
# brief, and the project-root deployment manifest at the workspace.
setup_workspace() {
  ensure_binary
  rm -rf "$SANDBOX"
  mkdir -p "$PLATFORM"
  local p
  for p in "$@"; do mkdir -p "$SANDBOX/$p"; done
  run "$PLATFORM" init --workspace
  for p in "$@"; do
    case "$p" in
      backend) run "$SANDBOX/$p" init "$(adapter_component omnia)" ;;
      contracts) run "$SANDBOX/$p" init "$(adapter_component contracts)" ;;
      mobile) run "$SANDBOX/$p" init "$(adapter_component vectis)" --platforms core,ios,android ;;
    esac
  done
  setup_git_remotes "$@"
  for p in "$@"; do
    run "$PLATFORM" registry add "$p" --url "../$p" --adapter "$(resolve_target "$p")" --description "$(proj_desc "$p")"
  done
  run "$PLATFORM" registry validate
  mkdir -p "$PLATFORM/docs"
  copy_fixture "$WFX/oauth-login.md" "$PLATFORM/docs/oauth-login.md"
  write_manifest "$PLATFORM" source:documentation target:omnia target:vectis target:contracts
}

author() {
  run "$PLATFORM" plan author "$PLAN_NAME" --source "$DOC_KEY=documentation:docs/oauth-login.md"
}

approve() { run "$PLATFORM" plan transition "$PLAN_NAME" approved --actor agent; }

execute() {
  try "$PLATFORM" plan execute
  report_final "$PLATFORM"
}

# --- orchestration -----------------------------------------------------------

setup_for() {
  case "$1" in
    workspace-fail-resume) setup_workspace backend mobile ;;
    *) setup_workspace backend mobile contracts ;;
  esac
}

main() {
  require_tools
  require_model
  setup_for "$SCENARIO"
  author
  approve
  echo "== execute =="
  execute
}

resume() {
  require_tools
  require_model
  ensure_binary
  # Stale-recovery re-entry: resync slots first so a dirty slot surfaces
  # its diagnostic; the operator commits or cleans per the scenario doc
  # before invoking this leg.
  [ "$SCENARIO" = "workspace-stale-recovery" ] && run "$PLATFORM" workspace sync
  echo "== resume =="
  execute
}

SCENARIO="${1:-}"
case "$SCENARIO" in
  workspace-two-projects | workspace-fail-resume | workspace-stale-recovery) ;;
  *)
    echo "usage: $(basename "$0") <workspace-two-projects|workspace-fail-resume|workspace-stale-recovery> [setup|resume]" >&2
    exit 2
    ;;
esac
SANDBOX="$SANDBOX_ROOT/$SCENARIO"
PLATFORM="$SANDBOX/platform"

case "${2:-main}" in
  setup) require_tools; setup_for "$SCENARIO" ;;
  resume) resume ;;
  main|"") main ;;
  *) echo "usage: $(basename "$0") <scenario> [setup|resume]" >&2; exit 2 ;;
esac
