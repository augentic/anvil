# shellcheck shell=bash
# Single-repo `/spec:execute` replay shared by the execute-* scenarios.
# Drives refine -> build -> merge under `specify plan lock --` via the lib.sh
# _drive loop. Sourced after the scenario sets SCENARIO / PLAN / PARK_SLICE
# and one of FAIL_SLICE / PAUSE_SLICE.

source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib.sh"

SANDBOX="$SANDBOX_ROOT/$SCENARIO"
FX="$FIXTURES_DIR/$SCENARIO"
TMPL="$FIXTURES_DIR/single-repo"
# Absolute path to the entry script, so lock re-entry survives `run_lock`'s cd.
SELF="$(cd "$(dirname "$0")" && pwd)/$(basename "$0")"

# --- project lifecycle -------------------------------------------------------

setup_project() {
  rm -rf "$SANDBOX"
  mkdir -p "$SANDBOX"
  run "$SANDBOX" init "$FRAMEWORK/adapters/targets/omnia"
  mkdir -p "$SANDBOX/adapters/sources"
  ln -sfn "$FRAMEWORK/adapters/sources/intent" "$SANDBOX/adapters/sources/intent"
  git -C "$SANDBOX" init -b main -q
}

gen_leads_md() {
  local first=1 name syn
  while IFS=$'\t' read -r name syn; do
    [ -n "$name" ] || continue
    [ "$first" -eq 1 ] || printf '\n'
    first=0
    printf '### %s\n\n- lead: %s\n- synopsis: %s\n' "$name" "$name" "$syn"
  done < "$FX/leads.tsv"
}

survey() {
  local intent md
  intent="$(cat "$FX/intent.txt")"
  run "$SANDBOX" plan create "$PLAN" --source "intent=intent:value:$intent"
  capture "$SANDBOX" source survey intent --phase prepare --format json >/dev/null
  md="$SANDBOX/.specify/scratch/intent/survey/leads.md"
  mkdir -p "$(dirname "$md")"
  gen_leads_md > "$md"
  run "$SANDBOX" source survey intent --phase finalize
}

propose() {
  capture "$SANDBOX" plan propose --dry-run --format json >/dev/null
  copy_fixture "$FX/propose-response.json" "$SANDBOX/.specify/scratch/plan/propose-response.json"
  run "$SANDBOX" plan propose --from ".specify/scratch/plan/propose-response.json"
}

approve() { run "$SANDBOX" plan transition "$PLAN" approved --actor agent; }

# --- helpers -----------------------------------------------------------------

synopsis_for() { awk -F'\t' -v s="$1" '$1==s{print $2}' "$FX/leads.tsv"; }
crate_name()   { printf '%s' "$1" | tr '-' '_'; }

ensure_workspace_root() {
  local root="$SANDBOX/Cargo.toml"
  [ -f "$root" ] || printf '[workspace]\nresolver = "2"\nmembers = ["crates/*"]\n' > "$root"
}

# --- _drive scenario hooks ---------------------------------------------------

drive_refine() {
  local cwd="$1" slice="$2" project="$3" sources="$4"
  local src lead stmt scratch
  src="$(printf '%s' "$sources" | jq -r '.[0].source')"
  lead="$(printf '%s' "$sources" | jq -r '.[0].lead')"
  stmt="$(synopsis_for "$slice")"
  run "$cwd" slice create "$slice"
  run "$cwd" source extract "$src" "$lead" --slice "$slice" --phase prepare
  render "$TMPL/evidence.yaml.tmpl" \
    "$cwd/.specify/scratch/$src/$slice/evidence.yaml" SLICE="$slice" STATEMENT="$stmt"
  run "$cwd" source extract "$src" "$lead" --slice "$slice" --phase finalize
  capture "$cwd" slice synthesize "$slice" --dry-run --format json >/dev/null
  scratch="$cwd/.specify/scratch/$slice/synthesize-response.json"
  render "$TMPL/synthesize-response.json.tmpl" "$scratch" SLICE="$slice" STATEMENT="$stmt"
  run "$cwd" slice synthesize "$slice" --from ".specify/scratch/$slice/synthesize-response.json"
  try "$cwd" slice validate "$slice"
  run "$cwd" slice transition "$slice" refined
}

# _build_slice <cwd> <slice> <mode:auto|fixed>
_build_slice() {
  local cwd="$1" slice="$2" mode="$3"
  local crate cdir secure status trc=0
  crate="$(crate_name "$slice")"
  cdir="$cwd/crates/$crate"
  capture "$cwd" slice build "$slice" --phase prepare --format json >/dev/null
  ensure_workspace_root
  render "$TMPL/crate/Cargo.toml.tmpl" "$cdir/Cargo.toml" CRATE="$crate"
  if [ "$slice" = "${FAIL_SLICE:-}" ]; then
    secure="false"; [ "$mode" = "fixed" ] && secure="true"
    render "$TMPL/crate/lib-cookie.rs.tmpl" "$cdir/src/lib.rs" CRATE="$crate" SECURE="$secure"
    render "$TMPL/crate/test-cookie.rs.tmpl" "$cdir/tests/integration.rs" CRATE="$crate"
  else
    render "$TMPL/crate/lib-marker.rs.tmpl" "$cdir/src/lib.rs" CRATE="$crate"
    render "$TMPL/crate/test-marker.rs.tmpl" "$cdir/tests/integration.rs" CRATE="$crate"
  fi
  try "$cwd" slice task mark "$slice" 1.1
  ( cd "$cdir" && cargo fmt ) >/dev/null 2>&1 || true
  ( cd "$cdir" && cargo test ) > "$cwd/.specify/slices/$slice/.build-log" 2>&1 || trc=$?
  status="success"; [ "$trc" -eq 0 ] || status="failure"
  mkdir -p "$cwd/.specify/slices/$slice/build"
  printf 'version: 1\nslice: %s\ntarget: omnia@v1\nstatus: %s\nfindings: []\n' \
    "$slice" "$status" > "$cwd/.specify/slices/$slice/build/report.yaml"
  [ "$trc" -eq 0 ] && try "$cwd" slice task mark "$slice" 1.2
  try "$cwd" slice build "$slice" --phase finalize --format json
}

drive_build()              { _build_slice "$1" "$2" auto; }
drive_build_prepare_only() { capture "$1" slice build "$2" --phase prepare --format json >/dev/null; }

drive_merge() {
  local cwd="$1" slice="$2" crate cdir
  crate="$(crate_name "$slice")"
  cdir="$cwd/crates/$crate"
  if [ -d "$cdir" ]; then
    ( cd "$cdir" && cargo fmt >/dev/null 2>&1; cargo test >/dev/null 2>&1 ) || true
  fi
  run "$cwd" slice merge run "$slice"
}

# --- orchestration -----------------------------------------------------------

main() {
  require_tools
  setup_project
  survey
  propose
  approve
  echo "== drive (expect park at $PARK_SLICE) =="
  run_lock "$SANDBOX" bash "$SELF" _drive "$SANDBOX"
  echo "== breakout: build $PARK_SLICE =="
  run_lock "$SANDBOX" bash "$SELF" _build "$PARK_SLICE"
  echo "== resume (expect drained) =="
  run_lock "$SANDBOX" bash "$SELF" _drive "$SANDBOX"
  local action done
  action="$(status_action "$SANDBOX")"
  done="$(status_count "$SANDBOX" done)"
  echo "FINAL action=$action done=$done"
  [ "$action" = "drained" ] || { echo "expected drained" >&2; return 1; }
}

dispatch() {
  case "${1:-main}" in
    _drive) _drive "$2" ;;
    _build) _build_slice "$SANDBOX" "$2" fixed ;;
    main|"") main ;;
    *) echo "usage: $(basename "$0")" >&2; exit 2 ;;
  esac
}
