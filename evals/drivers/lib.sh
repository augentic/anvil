# shellcheck shell=bash
# Shared helpers for the evals/drivers bash scenarios. bash 3.2-safe (no
# associative arrays). Sourced by each scenario script. Requires jq, git,
# and `specify` on PATH. Operator replay only; never wired into CI.
#
# The loop is driven entirely by `specify plan status` / `plan next`:
#   - plan status -> .action (refine|build|merge|stop|drained), .slice, .project
#   - plan next   -> .sources / .project / .target on a fresh pending->in-progress advance
# so no driver re-derives lifecycle state, source bindings, or routing.

set -euo pipefail

DRIVERS_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
FRAMEWORK="${SPECIFY_FRAMEWORK:-$(cd "$DRIVERS_DIR/../.." && pwd)}"
FIXTURES_DIR="$DRIVERS_DIR/fixtures"
SANDBOX_ROOT="${SPECIFY_SANDBOX:-$FRAMEWORK/evals/.sandbox}"
SPECIFY_BIN="${SPECIFY_BIN:-specify}"

# --- command execution -------------------------------------------------------

# run <cwd> <specify-args...> : run specify, stream output, abort on failure.
run() {
  local cwd="$1"; shift
  ( cd "$cwd" && "$SPECIFY_BIN" "$@" )
}

# capture <cwd> <specify-args...> : echo specify stdout; abort on failure.
capture() {
  local cwd="$1"; shift
  ( cd "$cwd" && "$SPECIFY_BIN" "$@" )
}

# try <cwd> <specify-args...> : run capturing rc into TRY_RC and combined
# output into TRY_OUT; never aborts under `set -e`. Use for intentional
# non-zero exits (engineered build failure, dirty-recovery probes).
try() {
  local cwd="$1"; shift
  TRY_RC=0
  TRY_OUT="$( ( cd "$cwd" && "$SPECIFY_BIN" "$@" ) 2>&1 )" || TRY_RC=$?
  [ -z "$TRY_OUT" ] || printf '%s\n' "$TRY_OUT"
}

# run_lock <cwd> <cmd...> : acquire the plan lock for the duration of <cmd>.
run_lock() {
  local cwd="$1"; shift
  ( cd "$cwd" && "$SPECIFY_BIN" plan lock -- "$@" )
}

# git_q <cwd> <git-args...> : best-effort git (never aborts).
git_q() {
  local cwd="$1"; shift
  git -C "$cwd" "$@" || true
}

# --- plan status / next projections -----------------------------------------

plan_status_json() { capture "$1" plan status --format json; }
status_action()    { plan_status_json "$1" | jq -r '.action'; }
status_slice()     { plan_status_json "$1" | jq -r '.slice // .active // empty'; }
status_project()   { plan_status_json "$1" | jq -r '.project // empty'; }
status_stop()      { plan_status_json "$1" | jq -r '.stop.reason // empty'; }
status_count()     { plan_status_json "$1" | jq -r --arg k "$2" '.counts[$k]'; }

# --- fixtures ----------------------------------------------------------------

# render <template> <dest> KEY=VAL ... : copy template to dest substituting
# @@KEY@@ tokens. Keeps envelope/template structure in fixture files; the
# driver supplies only per-slice values.
render() {
  local tmpl="$1" dest="$2"; shift 2
  local content pair k v pat
  content="$(cat "$tmpl")"
  for pair in "$@"; do
    k="${pair%%=*}"; v="${pair#*=}"
    pat="@@${k}@@"
    content="${content//$pat/$v}"
  done
  mkdir -p "$(dirname "$dest")"
  printf '%s\n' "$content" > "$dest"
}

# copy_fixture <src> <dest> : verbatim copy (creates parent dirs).
copy_fixture() {
  mkdir -p "$(dirname "$2")"
  cp "$1" "$2"
}

# --- environment guards ------------------------------------------------------

require_tools() {
  local missing=""
  command -v jq  >/dev/null 2>&1 || missing="$missing jq"
  command -v git >/dev/null 2>&1 || missing="$missing git"
  command -v "$SPECIFY_BIN" >/dev/null 2>&1 || missing="$missing $SPECIFY_BIN"
  if [ -n "$missing" ]; then
    echo "missing required tools:$missing" >&2
    exit 2
  fi
}

# --- shared drive loop -------------------------------------------------------
#
# Runs under a held plan lock (via `run_lock <plan_dir> bash "$0" _drive
# <plan_dir>`). Reads plan status, advances, and dispatches the named phase
# to scenario-defined drive_refine / drive_build / drive_merge functions.
# Handles both the initial execute and any post-breakout resume — the same
# loop, because `plan status .action` already reflects mid-slice lifecycle.
#
# Scenario hooks (functions the sourcing script must define):
#   drive_refine <plan_dir> <slice> <project> <sources-json>
#   drive_build  <plan_dir> <slice> <project>
#   drive_merge  <plan_dir> <slice> <project>
# Optional scenario vars:
#   PAUSE_SLICE  -- when building this slice, stop after prepare and park
_drive() {
  local plan_dir="$1"
  local action slice project next sources
  while true; do
    action="$(status_action "$plan_dir")"
    case "$action" in
      drained) echo "drained"; return 0 ;;
      stop)    echo "stop: $(status_stop "$plan_dir")"; return 0 ;;
    esac
    slice="$(status_slice "$plan_dir")"
    project="$(status_project "$plan_dir")"
    if [ -z "$slice" ]; then
      echo "no slice for action=$action" >&2
      return 1
    fi
    # Idempotent advance: fresh pending->in-progress returns .sources/.project;
    # an already-active entry returns the active name with no bindings.
    next="$(capture "$plan_dir" plan next --format json)"
    case "$action" in
      refine)
        sources="$(printf '%s' "$next" | jq -c '.sources // []')"
        [ -n "$project" ] || project="$(printf '%s' "$next" | jq -r '.project // empty')"
        drive_refine "$plan_dir" "$slice" "$project" "$sources"
        ;;
      build)
        if [ "${PAUSE_SLICE:-}" = "$slice" ]; then
          drive_build_prepare_only "$plan_dir" "$slice" "$project"
          echo "paused: build $slice (after prepare)"
          return 0
        fi
        drive_build "$plan_dir" "$slice" "$project"
        ;;
      merge)
        drive_merge "$plan_dir" "$slice" "$project"
        ;;
      *)
        echo "unexpected action: $action" >&2
        return 1
        ;;
    esac
  done
}
