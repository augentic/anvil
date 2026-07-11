#!/bin/bash
# Unified developer loop for the specify + specify-adapters sibling
# checkouts. Canonical implementation behind the `make dev-*` targets at
# both repo roots (the adapters root delegates here with
# SPECIFY_ADAPTERS pointed at itself). bash 3.2-safe: no associative
# arrays, no ${var,,}.
#
# Usage: dev.sh <command> [args]
#
#   doctor [--live]            validate sibling layout, toolchain, WASI
#                              target, and cursor-agent; --live adds a
#                              command-mode credential probe (a real,
#                              billable model call)
#   check [<adapter>]          fastest model-free rung: the named
#                              adapter's native tests (in the adapters
#                              checkout) plus the native harness
#                              seam/replay suite; no WASM, no model
#   run <project> [args...]    run specify-dev against any consumer
#                              project without changing directory
#   live [<adapter> [<test>]]  with no adapter, the repeated native-live
#                              workflow profile and structured report;
#                              with an adapter, exactly one live quality
#                              case (prose overlay on once artifacts exist)
#   full                       the explicit outer gate: doctor --live,
#                              deterministic checks, composed WASM/WIT
#                              coverage, and the repeated wasm-live
#                              workflow profile — never the default loop
#
# Overrides: SPECIFY_FRAMEWORK (this checkout), SPECIFY_ADAPTERS
# (sibling adapters checkout).
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
FRAMEWORK="${SPECIFY_FRAMEWORK:-$(cd "$SCRIPT_DIR/.." && pwd)}"
ADAPTERS="${SPECIFY_ADAPTERS:-$FRAMEWORK/../specify-adapters}"

say()  { printf '%s\n' "$*"; }
fail() { printf 'dev.sh: %s\n' "$*" >&2; exit 2; }

# cargo_in <repo-dir> <cargo args...> : run cargo inside one of the two
# sibling workspaces. Both workspaces build the same path crates (the
# harness links the adapter crates directly), so a global
# CARGO_TARGET_DIR shared across them cross-contaminates artifacts and
# yields "expected X, found X" type mismatches — scope it per repo.
cargo_in() {
  local dir="$1"; shift
  if [ -n "${CARGO_TARGET_DIR:-}" ]; then
    ( cd "$dir" && CARGO_TARGET_DIR="$CARGO_TARGET_DIR/$(basename "$dir")" cargo "$@" )
  else
    ( cd "$dir" && cargo "$@" )
  fi
}

# target_dir_of <repo-dir> : the cargo target directory cargo_in builds
# into for that repo.
target_dir_of() {
  if [ -n "${CARGO_TARGET_DIR:-}" ]; then
    printf '%s/%s' "$CARGO_TARGET_DIR" "$(basename "$1")"
  else
    printf '%s/target' "$1"
  fi
}

# --- doctor -----------------------------------------------------------------

# One check line: ok <label> / FAIL <label> plus an exact remediation.
DOCTOR_RC=0
report() { # <ok|fail> <label> [remediation]
  if [ "$1" = ok ]; then
    say "  ok    $2"
  else
    DOCTOR_RC=1
    say "  FAIL  $2"
    [ $# -lt 3 ] || say "        fix: $3"
  fi
}

doctor() {
  local live=0
  [ "${1:-}" != --live ] || live=1

  say "sibling layout"
  if [ -f "$FRAMEWORK/Makefile.toml" ] && [ -d "$FRAMEWORK/crates/workflow" ]; then
    report ok "specify checkout at $FRAMEWORK"
  else
    report fail "specify checkout at $FRAMEWORK" \
      "clone augentic/specify there or set SPECIFY_FRAMEWORK=<path>"
  fi
  if [ -d "$ADAPTERS/targets" ] && [ -d "$ADAPTERS/sources" ]; then
    report ok "specify-adapters checkout at $ADAPTERS"
  else
    report fail "specify-adapters checkout at $ADAPTERS" \
      "clone augentic/specify-adapters as a sibling or set SPECIFY_ADAPTERS=<path>"
  fi

  say "toolchain"
  local tool
  for tool in cargo rustup jq git; do
    if command -v "$tool" >/dev/null 2>&1; then
      report ok "$tool on PATH"
    else
      report fail "$tool on PATH" "install $tool"
    fi
  done
  if cargo make --version >/dev/null 2>&1; then
    report ok "cargo-make"
  else
    report fail "cargo-make" "cargo install cargo-make"
  fi
  if cargo nextest --version >/dev/null 2>&1; then
    report ok "cargo-nextest"
  else
    report fail "cargo-nextest" "cargo install cargo-nextest"
  fi
  if rustup target list --installed 2>/dev/null | grep -q '^wasm32-wasip2$'; then
    report ok "wasm32-wasip2 target"
  else
    report fail "wasm32-wasip2 target" "rustup target add wasm32-wasip2"
  fi

  say "model backend"
  if command -v cursor-agent >/dev/null 2>&1; then
    report ok "cursor-agent on PATH"
    if [ "$live" = 1 ]; then
      # `cursor-agent status` proves an IDE login, not command-mode
      # credentials — the backends spawn `--print` mode, which needs
      # `cursor-agent login` or CURSOR_API_KEY. Probe the real thing.
      say "  ..    live credential probe (one real model call)"
      if out="$(cursor-agent --print 'Reply with the single word OK' 2>&1)" \
        && [ -n "$out" ]; then
        report ok "command-mode credentials"
      else
        report fail "command-mode credentials" \
          "run \`cursor-agent login\` or export CURSOR_API_KEY (\`cursor-agent status\` alone does not prove --print auth)"
      fi
    else
      say "  ..    credential probe skipped (doctor --live / LIVE=1 runs one real model call)"
    fi
  else
    report fail "cursor-agent on PATH" \
      "install from https://cursor.com/docs/cli then \`cursor-agent login\` (only live runs need it)"
  fi

  [ "$DOCTOR_RC" = 0 ] && say "doctor: all checks passed" \
    || say "doctor: failures above — apply the fixes and re-run"
  return "$DOCTOR_RC"
}

# --- check ------------------------------------------------------------------

check() {
  local adapter="${1:-}"
  if [ -n "$adapter" ]; then
    [ -d "$ADAPTERS/targets/$adapter" ] || [ -d "$ADAPTERS/sources/$adapter" ] \
      || fail "no adapter \`$adapter\` under $ADAPTERS/{targets,sources}"
    # A bare `-p <name>` can be ambiguous against same-named upstream
    # dependencies (the `omnia` adapter vs the `omnia` host crate), so
    # pin the workspace member's exact `name@version` spec.
    local spec
    spec="$(cd "$ADAPTERS" && cargo metadata --no-deps --format-version 1 \
      | jq -r --arg n "$adapter" \
        '.packages[] | select(.name == $n) | "\(.name)@\(.version)"' | head -n 1)"
    [ -n "$spec" ] || fail "adapter \`$adapter\` is not a workspace member of $ADAPTERS"
    say "== native tests: $spec (adapters checkout) =="
    cargo_in "$ADAPTERS" nextest run -p "$spec" --no-tests=pass
  else
    say "== no adapter scoped (ADAPTER=<name> adds its native tests) =="
  fi
  say "== native harness seam/replay tests (specify checkout) =="
  cargo_in "$FRAMEWORK" nextest run -p specify-dev
}

# --- run --------------------------------------------------------------------

run() {
  [ $# -ge 1 ] && [ -n "$1" ] || fail "usage: dev.sh run <project-dir> [specify-dev args...] (make dev-run PROJECT=<path> ARGS='...')"
  local project="$1"; shift
  [ -d "$project" ] || fail "project directory not found: $project"
  # Resolve the project to an absolute path first — `cargo run` executes
  # with the framework checkout as its working directory.
  project="$(cd "$project" && pwd)"
  cargo_in "$FRAMEWORK" run -q -p specify-dev -- --project-dir "$project" "$@"
}

# --- live -------------------------------------------------------------------

# Default single live scenario (the live-test name in evals/live.rs)
# per adapter. bash 3.2: a case, not an associative array.
default_scenario() {
  case "$1" in
    contracts) printf design ;;
    vectis)    printf single_screen ;;
    *)         printf '' ;;
  esac
}

live() {
  local adapter="${1:-}" scenario="${2:-}"
  if [ -z "$adapter" ]; then
    # Specify workflow iteration: the native shim is the default live
    # rung — no WASM builds, no deployment manifest. Composed mode is
    # dev-full's job.
    say "== live workflow profile: native-live =="
    bash "$FRAMEWORK/quality/run-live.sh" native-live
    return
  fi

  [ -d "$ADAPTERS/targets/$adapter" ] || [ -d "$ADAPTERS/sources/$adapter" ] \
    || fail "no adapter \`$adapter\` under $ADAPTERS/{targets,sources}"
  if [ -z "$scenario" ]; then
    scenario="$(default_scenario "$adapter")"
    [ -n "$scenario" ] || fail "no default live scenario for \`$adapter\`; pass SCENARIO=<live-test name from evals/live.rs>"
  fi

  # Prose-overlay iteration is the point of the adapter live rung: once
  # the three run artifacts exist, a re-run skips cargo entirely and
  # only the model leg repeats. Enable it by default when the artifacts
  # are present; SPECIFY_PROSE_OVERLAY=0 opts out.
  local target wasm overlay
  target="$(target_dir_of "$ADAPTERS")"
  wasm="$target/wasm32-wasip2/debug"
  overlay="${SPECIFY_PROSE_OVERLAY:-}"
  if [ -z "$overlay" ] \
    && [ -f "$wasm/$adapter.wasm" ] \
    && [ -f "$wasm/examples/eval_guest.wasm" ] \
    && [ -f "$target/debug/examples/eval-driver" ]; then
    overlay=1
    say "== prose overlay on (artifacts present; SPECIFY_PROSE_OVERLAY=0 opts out) =="
  fi

  say "== live adapter eval: $adapter::$scenario =="
  (
    cd "$ADAPTERS"
    [ -z "${CARGO_TARGET_DIR:-}" ] || export CARGO_TARGET_DIR="$target"
    [ "$overlay" != 1 ] || export SPECIFY_PROSE_OVERLAY=1
    cargo test -p evals --test live -- \
      --ignored --nocapture --exact "$adapter::$scenario"
  )
}

# --- full -------------------------------------------------------------------

full() {
  say "==== dev-full: the explicit outer gate (WASM + live model) ===="
  doctor --live

  say "== deterministic native rung =="
  check ""

  say "== composed WASM/WIT coverage (adapters checkout) =="
  cargo_in "$ADAPTERS" test -p evals --test composed

  say "== composed workflow profile: wasm-live =="
  bash "$FRAMEWORK/quality/run-live.sh" wasm-live

  say "==== dev-full: complete ===="
}

# --- dispatch ---------------------------------------------------------------

command="${1:-}"
[ $# -eq 0 ] || shift
case "$command" in
  doctor) doctor "$@" ;;
  check)  check "$@" ;;
  run)    run "$@" ;;
  live)   live "$@" ;;
  full)   full "$@" ;;
  *)      fail "unknown command \`$command\` — one of: doctor, check, run, live, full (see the header of scripts/dev.sh)" ;;
esac
