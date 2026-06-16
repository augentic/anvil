#!/usr/bin/env bash
# Workspace `/spec:execute` replay for the multi-repo eval scenarios:
#   workspace-two-projects | workspace-fail-resume | workspace-stale-recovery
# Drives refine -> build -> merge across materialised workspace slots under
# `specify plan lock --` via the lib.sh _drive loop. Operator aid; never CI.
#
# Routing and source bindings come straight from the CLI: `plan status .project`
# names the slot, `plan next .sources` carries each slice's (source, lead). The
# default-on platform bootstrap would insert an `app-foundation` slice with empty sources.
set -euo pipefail
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib.sh"

SELF="$(cd "$(dirname "$0")" && pwd)/$(basename "$0")"
PLAN_NAME="oauth-login"
DOC_KEY="${DOC_KEY:-brief}"
WFX="$FIXTURES_DIR/workspace"

# Context inherited across `specify plan lock --` re-entry (exported by main).
WS_FAIL_SLICE="${WS_FAIL_SLICE:-}"
PAUSE_SLICE="${PAUSE_SLICE:-}"
DIRTY_MARKER="${DIRTY_MARKER:-}"
SCAFFOLD_MOBILE="${SCAFFOLD_MOBILE:-0}"

# --- small helpers -----------------------------------------------------------

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

titleize() { printf '%s' "$1" | tr '-' ' ' | awk '{for(i=1;i<=NF;i++)$i=toupper(substr($i,1,1)) substr($i,2)}1'; }

read_slice_meta() {
  CLAIM_ID="$(awk -F'\t' -v s="$1" '$1==s{print $2}' "$WFX/slices.tsv")"
  STATEMENT="$(awk -F'\t' -v s="$1" '$1==s{print $3}' "$WFX/slices.tsv")"
  DOMAIN="$(awk -F'\t' -v s="$1" '$1==s{print $4}' "$WFX/slices.tsv")"
}

git_commit_all() {
  git -C "$1" add -A >/dev/null 2>&1 || true
  git -C "$1" commit -q --no-gpg-sign -m "$2" >/dev/null 2>&1 || true
}

# Run specify in a workspace slot with the plan dir pinned to the platform root.
wrun() { local slot="$1"; shift; ( cd "$slot" && SPECIFY_PLAN_DIR="$PLATFORM" "$SPECIFY_BIN" "$@" ); }
wcap() { wrun "$@"; }
wtry() {
  local slot="$1"; shift
  TRY_RC=0
  TRY_OUT="$( ( cd "$slot" && SPECIFY_PLAN_DIR="$PLATFORM" "$SPECIFY_BIN" "$@" ) 2>&1 )" || TRY_RC=$?
  [ -z "$TRY_OUT" ] || printf '%s\n' "$TRY_OUT"
}

# --- workspace + plan setup --------------------------------------------------

scaffold_mobile() {
  local d="$1"
  mkdir -p "$d/shared/src"; printf 'pub struct App;\n' > "$d/shared/src/app.rs"
  mkdir -p "$d/iOS"
  printf 'import SwiftUI\nstruct App: SwiftUI.App { var body: some Scene { WindowGroup { Text("App") } } }\n' > "$d/iOS/App.swift"
  mkdir -p "$d/Android/app/src/main/kotlin/com/example/app"
  printf 'package com.example.app\nclass App\n' > "$d/Android/app/src/main/kotlin/com/example/app/App.kt"
}

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

setup_workspace() {
  rm -rf "$SANDBOX"
  mkdir -p "$PLATFORM"
  local p
  for p in "$@"; do mkdir -p "$SANDBOX/$p"; done
  run "$PLATFORM" init --workspace
  mkdir -p "$PLATFORM/adapters/sources"
  ln -sfn "$FRAMEWORK/adapters/sources/documentation" "$PLATFORM/adapters/sources/documentation"
  for p in "$@"; do
    case "$p" in
      backend) run "$SANDBOX/$p" init "$FRAMEWORK/adapters/targets/omnia" ;;
      contracts) run "$SANDBOX/$p" init "$FRAMEWORK/adapters/targets/contracts" ;;
      mobile)
        run "$SANDBOX/$p" init "$FRAMEWORK/adapters/targets/vectis" --platforms core,ios,android
        [ "$SCAFFOLD_MOBILE" = "1" ] && scaffold_mobile "$SANDBOX/$p"
        ;;
    esac
  done
  setup_git_remotes "$@"
  for p in "$@"; do
    run "$PLATFORM" registry add "$p" --url "../$p" --adapter "$(resolve_target "$p")" --description "$(proj_desc "$p")"
  done
  run "$PLATFORM" registry validate
  mkdir -p "$PLATFORM/docs"
  copy_fixture "$WFX/oauth-login.md" "$PLATFORM/docs/oauth-login.md"
}

create_plan() {
  local mode="$1" out scratch pr
  run "$PLATFORM" plan create "$PLAN_NAME" --source "$DOC_KEY=documentation:docs/oauth-login.md"
  run "$PLATFORM" workspace sync
  out="$(capture "$PLATFORM" source survey "$DOC_KEY" --phase prepare --format json)"
  scratch="$(printf '%s' "$out" | jq -r '.["scratch-dir"]')"
  if [ "$mode" = "fail-resume" ]; then
    copy_fixture "$WFX/leads-fail-resume.md" "$scratch/leads.md"
  else
    copy_fixture "$WFX/leads-full.md" "$scratch/leads.md"
  fi
  run "$PLATFORM" source survey "$DOC_KEY" --phase finalize
  printf '# %s\n\nOAuth login across backend, mobile, and contracts projects.\n\n## Cross-cutting leads\n\nNone.\n' \
    "$PLAN_NAME" > "$PLATFORM/change.md"
  pr="$PLATFORM/.specify/scratch/plan/propose-response.json"
  if [ "$mode" = "fail-resume" ]; then
    copy_fixture "$WFX/propose-fail-resume.json" "$pr"
    run "$PLATFORM" plan propose --from ".specify/scratch/plan/propose-response.json"
  else
    copy_fixture "$WFX/propose-full.json" "$pr"
    run "$PLATFORM" plan propose --from ".specify/scratch/plan/propose-response.json"
  fi
  run "$PLATFORM" plan validate
}

approve() { run "$PLATFORM" plan transition "$PLAN_NAME" approved --actor agent; }

# --- slot routing ------------------------------------------------------------

route_to_slot() {
  local project="$1" slice="$2" slot="$PLATFORM/workspace/$project"
  run "$PLATFORM" workspace sync "$project"
  wtry "$PLATFORM" workspace prepare "$project" --change "$PLAN_NAME"
  if [ "$TRY_RC" -ne 0 ]; then
    if printf '%s' "$TRY_OUT" | grep -q dirty-unrelated-tracked; then
      git_commit_all "$slot" "specify: residue $slice"
      run "$PLATFORM" workspace prepare "$project" --change "$PLAN_NAME"
    else
      echo "workspace prepare failed for $project" >&2
      return 1
    fi
  fi
}

# --- content seeding ---------------------------------------------------------

seed_contracts_input() {
  local d="$1/.specify/slices/$2/contracts"
  mkdir -p "$d"
  printf 'openapi: 3.1.0\ninfo:\n  title: OAuth Login\n  version: 1.0.0\npaths: {}\n' > "$d/openapi.yaml"
}

seed_auth_rotate() {
  local c="$1/crates/auth_rotate"
  copy_fixture "$WFX/auth-rotate/Cargo.toml" "$c/Cargo.toml"
  render "$WFX/auth-rotate/lib.rs.tmpl" "$c/src/lib.rs" SECURE="$2"
}

write_report() {
  printf 'version: 1\nslice: %s\ntarget: %s\nstatus: %s\nfindings: []\n' "$2" "$3" "$4" > "$1"
}

write_vectis_outputs() {
  local slot="$1" report="$2" slice="$3" target="$4"
  mkdir -p "$slot/shared/src"; printf 'pub struct App;\n' > "$slot/shared/src/app.rs"
  mkdir -p "$slot/iOS"; printf 'import SwiftUI\n' > "$slot/iOS/App.swift"
  mkdir -p "$slot/Android/app/src/main/kotlin/com/example/app"
  printf 'package com.example.app\nclass App\n' > "$slot/Android/app/src/main/kotlin/com/example/app/App.kt"
  {
    printf 'version: 1\nslice: %s\ntarget: %s\nstatus: success\nfindings: []\n' "$slice" "$target"
    printf 'outputs:\n'
    printf '  - platform: core\n    path: shared/src\n'
    printf '  - platform: ios\n    path: iOS\n'
    printf '  - platform: android\n    path: Android\n'
  } > "$report"
}

# --- _drive scenario hooks ---------------------------------------------------

drive_refine() {
  local plan_dir="$1" slice="$2" project="$3" sources="$4"
  local slot="$PLATFORM/workspace/$project" target title n i src lead out scratch
  route_to_slot "$project" "$slice"
  target="$(resolve_target "$project")"
  wrun "$slot" slice create "$slice" --target "$target"
  read_slice_meta "$slice"
  title="$(titleize "$slice")"
  n="$(printf '%s' "$sources" | jq 'length')"
  if [ "$n" -gt 0 ]; then
    i=0
    while [ "$i" -lt "$n" ]; do
      src="$(printf '%s' "$sources" | jq -r ".[$i].source")"
      lead="$(printf '%s' "$sources" | jq -r ".[$i].lead")"
      out="$(wcap "$slot" source extract "$src" "$lead" --slice "$slice" --phase prepare --format json)"
      scratch="$(printf '%s' "$out" | jq -r '.["scratch-dir"]')"
      render "$WFX/evidence.yaml.tmpl" "$scratch/evidence.yaml" \
        LEAD="$lead" CLAIM_ID="$CLAIM_ID" STATEMENT="$STATEMENT"
      wrun "$slot" source extract "$src" "$lead" --slice "$slice" --phase finalize
      i=$((i + 1))
    done
    render "$WFX/synthesize-response.json.tmpl" "$slot/synth.json" \
      SLICE="$slice" TITLE="$title" DOMAIN="$DOMAIN" CLAIM_ID="$CLAIM_ID" STATEMENT="$STATEMENT" DOC_KEY="$DOC_KEY"
  else
    render "$WFX/synthesize-bootstrap.json.tmpl" "$slot/synth.json" \
      SLICE="$slice" TITLE="$title" DOMAIN="$DOMAIN" STATEMENT="$STATEMENT"
  fi
  wrun "$slot" slice synthesize "$slice" --from synth.json --format json
  wtry "$slot" slice validate "$slice"
  [ "$target" = "contracts" ] && seed_contracts_input "$slot" "$slice"
  [ "$slice" = "auth-rotate" ] && seed_auth_rotate "$slot" false
  wrun "$slot" slice transition "$slice" refined
  git_commit_all "$slot" "specify: residue $slice"
}

drive_build() {
  local plan_dir="$1" slice="$2" project="$3"
  local slot="$PLATFORM/workspace/$project" target report
  route_to_slot "$project" "$slice"
  target="$(resolve_target "$project")"
  wcap "$slot" slice build "$slice" --phase prepare --format json >/dev/null
  report="$slot/.specify/slices/$slice/build/report.yaml"
  mkdir -p "$(dirname "$report")"
  if [ "$slice" = "${WS_FAIL_SLICE:-}" ]; then
    write_report "$report" "$slice" "$target@v1" failure
  elif [ "$target" = "vectis" ]; then
    write_vectis_outputs "$slot" "$report" "$slice" "$target@v1"
  else
    write_report "$report" "$slice" "$target@v1" success
  fi
  wtry "$slot" slice build "$slice" --phase finalize --format json
  git_commit_all "$slot" "specify: residue $slice"
}

drive_build_prepare_only() {
  local plan_dir="$1" slice="$2" project="$3" slot="$PLATFORM/workspace/$3"
  route_to_slot "$project" "$slice"
  wcap "$slot" slice build "$slice" --phase prepare --format json >/dev/null
  [ -n "${DIRTY_MARKER:-}" ] && printf 'dirty\n' > "$slot/$DIRTY_MARKER"
}

drive_merge() {
  local plan_dir="$1" slice="$2" project="$3" slot="$PLATFORM/workspace/$3"
  route_to_slot "$project" "$slice"
  git_commit_all "$slot" "specify: pre-merge $slice"
  wrun "$slot" slice merge run "$slice"
  git_commit_all "$slot" "specify: residue $slice"
}

# Breakout: fix the parked slice and rebuild to success.
_breakout_build() {
  local slice="$1" project slot
  project="$(status_project "$PLATFORM")"
  slot="$PLATFORM/workspace/$project"
  seed_auth_rotate "$slot" true
  git_commit_all "$slot" "specify: triage $slice build failure"
  WS_FAIL_SLICE=""
  drive_build "$PLATFORM" "$slice" "$project"
}

# --- orchestration -----------------------------------------------------------

main() {
  require_tools
  SANDBOX="$SANDBOX_ROOT/$SCENARIO"
  PLATFORM="$SANDBOX/platform"
  case "$SCENARIO" in
    workspace-two-projects)
      SCAFFOLD_MOBILE=0
      setup_workspace backend mobile contracts
      create_plan full
      approve
      run_lock "$PLATFORM" bash "$SELF" _drive "$PLATFORM"
      ;;
    workspace-fail-resume)
      SCAFFOLD_MOBILE=1
      setup_workspace backend mobile
      create_plan fail-resume
      approve
      export WS_FAIL_SLICE="auth-rotate"
      echo "== drive (expect park at auth-rotate) =="
      run_lock "$PLATFORM" bash "$SELF" _drive "$PLATFORM"
      echo "== breakout: build auth-rotate =="
      run_lock "$PLATFORM" bash "$SELF" _breakout "$PLATFORM" auth-rotate
      export WS_FAIL_SLICE=""
      echo "== resume (expect drained) =="
      run_lock "$PLATFORM" bash "$SELF" _drive "$PLATFORM"
      ;;
    workspace-stale-recovery)
      SCAFFOLD_MOBILE=0
      setup_workspace backend mobile contracts
      create_plan full
      approve
      export PAUSE_SLICE="oauth-backend" DIRTY_MARKER="eval-dirty-uncommitted.txt"
      echo "== drive (expect interrupt at oauth-backend build) =="
      run_lock "$PLATFORM" bash "$SELF" _drive "$PLATFORM"
      echo "== recover stale slot =="
      run "$PLATFORM" workspace sync
      rm -f "$PLATFORM/workspace/backend/$DIRTY_MARKER"
      git_commit_all "$PLATFORM/workspace/backend" "specify: triage stale oauth-backend"
      export PAUSE_SLICE="" DIRTY_MARKER=""
      echo "== resume (expect drained) =="
      run_lock "$PLATFORM" bash "$SELF" _drive "$PLATFORM"
      ;;
    *)
      echo "unknown scenario: $SCENARIO" >&2
      return 2
      ;;
  esac
  local action
  action="$(status_action "$PLATFORM")"
  echo "FINAL action=$action done=$(status_count "$PLATFORM" done)"
  [ "$action" = "drained" ] || { echo "expected drained" >&2; return 1; }
}

case "${1:-}" in
  _drive) PLATFORM="$2"; _drive "$PLATFORM" ;;
  _breakout) PLATFORM="$2"; _breakout_build "$3" ;;
  workspace-two-projects | workspace-fail-resume | workspace-stale-recovery)
    SCENARIO="$1"; main ;;
  *)
    echo "usage: $(basename "$0") <workspace-two-projects|workspace-fail-resume|workspace-stale-recovery>" >&2
    exit 2
    ;;
esac
