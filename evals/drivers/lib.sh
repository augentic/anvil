# shellcheck shell=bash
# Shared helpers for the evals/drivers bash scenarios. bash 3.2-safe (no
# associative arrays). Sourced by each scenario script. Requires jq and git;
# the `specify` binary under test is built from this checkout by default.
# Operator replay only; never wired into CI.
#
# The loop is guest-owned: `specify plan execute` drives refine -> build ->
# merge to drained (or a stop), so no driver re-derives lifecycle state,
# source bindings, or routing. Drivers do the clerical setup (init, manifest,
# brief), stamp Gate 1 on the operator's behalf (`--actor agent`), invoke
# `plan author` / `plan execute`, and run the post-run probes.
#
# Dev overrides keep every run off the network: SPECIFY_CORE_PATH pins the
# in-tree workflow guest as the core, and adapters resolve as release-built
# components from the sibling augentic/specify-adapters checkout — never a
# registry fetch.

set -euo pipefail

DRIVERS_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
FRAMEWORK="${SPECIFY_FRAMEWORK:-$(cd "$DRIVERS_DIR/../.." && pwd)}"
FIXTURES_DIR="$DRIVERS_DIR/fixtures"
SANDBOX_ROOT="${SPECIFY_SANDBOX:-$FRAMEWORK/evals/.sandbox}"
ADAPTERS="${SPECIFY_ADAPTERS:-$FRAMEWORK/../specify-adapters}"
SPECIFY_BIN="${SPECIFY_BIN:-}"

# --- build under test ----------------------------------------------------

# ensure_binary : build the branch-head `specify` binary and workflow guest
# once, point SPECIFY_BIN at the build, and pin the guest as the core via
# SPECIFY_CORE_PATH (development override) so nothing hydrates
# `specify:core` from the registry. Honors a pre-set SPECIFY_BIN /
# SPECIFY_CORE_PATH to replay against a different build.
ensure_binary() {
  if [ -z "$SPECIFY_BIN" ]; then
    ( cd "$FRAMEWORK" && cargo build -q -p specify-cli )
    SPECIFY_BIN="$FRAMEWORK/target/debug/specify"
  fi
  if [ -z "${SPECIFY_CORE_PATH:-}" ]; then
    ( cd "$FRAMEWORK" && cargo build -q -p specify-cli --lib --target wasm32-wasip2 )
    SPECIFY_CORE_PATH="$FRAMEWORK/target/wasm32-wasip2/debug/specify.wasm"
  fi
  export SPECIFY_CORE_PATH
}

# adapter_component <name> : the sibling checkout's release-built component
# for a first-party adapter (`cargo make release` there populates it).
adapter_component() {
  local wasm="$ADAPTERS/target/wasm32-wasip2/release/$(printf '%s' "$1" | tr '-' '_').wasm"
  if [ ! -f "$wasm" ]; then
    echo "adapter component not found at $wasm; run \`cargo make release\` in the sibling specify-adapters checkout (override the root with SPECIFY_ADAPTERS=)" >&2
    return 2
  fi
  printf '%s' "$wasm"
}

# write_manifest <root> <axis:name>... : write a project-root omnia.toml —
# the workflow guest plus the named release-built adapter components, each
# adapter's MCP references at /mcp/<name>, one writable "." mount at <root>.
# The forwarding leg honors a project-root omnia.toml over the generated
# manifest in the project cache, which lets `plan author` see source
# adapters before plan.yaml binds them (the documented dev pattern; see the
# checked-in repo-root omnia.toml).
write_manifest() {
  local root="$1"; shift
  local id axis name
  {
    printf '[[guest]]\nid = "workflow"\nsource.path = "%s"\n' "$SPECIFY_CORE_PATH"
    printf 'link = ["specify:adapter/source@0.1.0", "specify:adapter/target@0.1.0"]\n\n'
    for id in "$@"; do
      name="${id#*:}"
      printf '[[guest]]\nid = "%s"\nsource.path = "%s"\n\n' "$id" "$(adapter_component "$name")"
    done
    printf '[[mount]]\nname = "."\npath = "%s"\nwritable = true\n\n' "$root"
    for id in "$@"; do
      name="${id#*:}"
      printf '[[route.http]]\nprefix = "/mcp/%s"\nguest = "%s"\n\n' "$name" "$id"
    done
    printf '[transport]\ndefault = "in-process"\n'
  } > "$root/omnia.toml"
}

# --- command execution ----------------------------------------------------

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
# non-zero exits (a parked execute, dirty-recovery probes).
try() {
  local cwd="$1"; shift
  TRY_RC=0
  TRY_OUT="$( ( cd "$cwd" && "$SPECIFY_BIN" "$@" ) 2>&1 )" || TRY_RC=$?
  [ -z "$TRY_OUT" ] || printf '%s\n' "$TRY_OUT"
}

# git_q <cwd> <git-args...> : best-effort git (never aborts).
git_q() {
  local cwd="$1"; shift
  git -C "$cwd" "$@" || true
}

# --- plan status projections ----------------------------------------------

plan_status_json() { capture "$1" plan status --format json; }
status_action()    { plan_status_json "$1" | jq -r '.action'; }
status_slice()     { plan_status_json "$1" | jq -r '.slice // .active // empty'; }
status_stop()      { plan_status_json "$1" | jq -r '.stop.reason // empty'; }
status_count()     { plan_status_json "$1" | jq -r --arg k "$2" '.counts[$k]'; }

# report_final <root> : print the closing status line and fail unless drained.
report_final() {
  local root="$1" action
  action="$(status_action "$root")"
  echo "FINAL action=$action done=$(status_count "$root" done)"
  if [ "$action" = "stop" ]; then
    echo "stop: $(status_stop "$root") — fix per the hint, then re-run the driver's resume leg" >&2
    return 1
  fi
  [ "$action" = "drained" ] || { echo "expected drained" >&2; return 1; }
}

# --- fixtures ----------------------------------------------------------------

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
  if [ -n "$missing" ]; then
    echo "missing required tools:$missing" >&2
    exit 2
  fi
}

# require_model : the judgment legs (`plan author`, `plan execute`) spawn
# cursor-agent through the composed runtime; setup-only legs skip this.
require_model() {
  command -v cursor-agent >/dev/null 2>&1 || {
    echo "cursor-agent not found on PATH; the model-driven legs need a logged-in cursor backend" >&2
    exit 2
  }
}
