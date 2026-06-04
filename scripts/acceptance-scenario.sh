#!/usr/bin/env bash

# Scaffold one acceptance scenario's disposable environment up to its
# pre-invocation state, then stop.
#
# SETUP HELPER ONLY. It runs the scenario's real `specify` setup steps (a fresh
# temp project + the named `specify init`) and then stops at the invocation
# point. It never drives a /spec:* command, never self-grades, and never stubs a
# forge, a runner, or a CI step — so every scenario `negative-expectation` still
# holds. For cross-repo workspace scenarios it prints the steps to run by hand
# rather than replaying arbitrary prose.
#
# Usage:
#   make acceptance-scenario ID=pure-intent
#   bash ./scripts/acceptance-scenario.sh pure-intent
#
# The build under test is the bare `specify` on PATH — run `make acceptance`
# first. First-party adapter shorthand (e.g. `omnia@v1`) resolves offline
# because this script exports SPECIFY_FRAMEWORK_ROOT to the repo root.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
LIFECYCLE_DIR="$REPO_ROOT/acceptance/lifecycle"

ID="${1:-${ID:-}}"
if [ -z "$ID" ]; then
  echo "Usage: $0 <scenario-id>   (or: make acceptance-scenario ID=<scenario-id>)" >&2
  echo "Available scenarios:" >&2
  for f in "$LIFECYCLE_DIR"/*.md; do
    base="$(basename "$f")"
    [ "$base" = "README.md" ] && continue
    echo "  ${base%.md}" >&2
  done
  exit 2
fi

# Resolve the scenario file from a flexible id: exact filename, filename that
# contains the id, or a file whose frontmatter `id:` matches.
resolve_scenario() {
  local id="$1" f base
  if [ -f "$LIFECYCLE_DIR/$id.md" ]; then
    printf '%s\n' "$LIFECYCLE_DIR/$id.md"
    return 0
  fi
  local matches=()
  for f in "$LIFECYCLE_DIR"/*.md; do
    base="$(basename "$f")"
    [ "$base" = "README.md" ] && continue
    if [[ "$base" == *"$id"* ]] || grep -qE "^id:[[:space:]]+${id}[[:space:]]*$" "$f"; then
      matches+=("$f")
    fi
  done
  case "${#matches[@]}" in
    1) printf '%s\n' "${matches[0]}"; return 0 ;;
    0) return 1 ;;
    *)
      printf 'Ambiguous id %s matches:\n' "$id" >&2
      printf '  %s\n' "${matches[@]}" >&2
      return 1
      ;;
  esac
}

if ! SCENARIO="$(resolve_scenario "$ID")"; then
  echo "No unique scenario found for id '$ID' under $LIFECYCLE_DIR" >&2
  exit 2
fi
echo "Scenario file: $SCENARIO"

# Require the build under test on PATH.
if ! command -v specify >/dev/null 2>&1; then
  echo "Error: 'specify' is not on PATH. Run 'make acceptance' first (see acceptance/shared/setup.md)." >&2
  exit 1
fi
echo "Using specify:  $(command -v specify) ($(specify --version 2>/dev/null || echo 'version unknown'))"

# First-party adapter shorthand resolves against this checkout, offline.
export SPECIFY_FRAMEWORK_ROOT="$REPO_ROOT"

# Fresh disposable root.
TMP_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/specify-acceptance-${ID}.XXXXXX")"
echo "Temp root:      $TMP_ROOT"

# The '## Setup' / '## Invocation' section bodies (between the heading and the
# next '## ' heading).
section_body() {
  awk -v want="## $1" '
    $0 == want { f = 1; next }
    /^## / { f = 0 }
    f { print }
  ' "$SCENARIO"
}

print_invocation() {
  echo
  echo "--- Scenario Invocation (run these by hand; this helper drives no /spec:* command) ---"
  section_body "Invocation"
}

# Cross-repo / workspace scenarios script their own mkdir/cd/registry steps in
# prose (often under headings that vary per scenario); this helper does not
# replay arbitrary multi-repo setup. Detect from the whole file, hand back the
# canonical instructions, and stop after creating the temp root.
if grep -qiE 'specify init --workspace|registry add|cross-repo workspace' "$SCENARIO"; then
  echo
  echo "This is a cross-repo / workspace scenario; this helper does not auto-replay multi-repo setup."
  echo "Set it up by hand inside the temp root:"
  echo "  $TMP_ROOT"
  echo "Follow the cross-repo workspace block in acceptance/shared/setup.md and the steps in:"
  echo "  $SCENARIO"
  echo "(SPECIFY_FRAMEWORK_ROOT=$REPO_ROOT is exported for offline adapter resolution.)"
  exit 0
fi

# Single-project scenario: create a project dir and run the named init.
PROJECT_DIR="$TMP_ROOT/$ID"
mkdir -p "$PROJECT_DIR"

# Adapter the scenario names in its first `specify init <adapter>`, preferring
# the `## Setup` section and falling back to the whole file (defaults to
# omnia@v1). The token must start with a letter so `--workspace` never matches.
adapter_token() {
  grep -oE 'specify init [A-Za-z][A-Za-z0-9@._-]*' | head -n1 | awk '{print $3}'
}
ADAPTER="$(section_body "Setup" | adapter_token || true)"
if [ -z "$ADAPTER" ]; then
  ADAPTER="$(adapter_token < "$SCENARIO" || true)"
fi
ADAPTER="${ADAPTER:-omnia@v1}"

echo
echo "+ cd $PROJECT_DIR"
echo "+ specify init $ADAPTER"
( cd "$PROJECT_DIR" && specify init "$ADAPTER" )

echo
echo "Scaffolded to the pre-invocation state:"
echo "  project: $PROJECT_DIR"
echo "  adapter: $ADAPTER"
echo
echo "If the scenario's '## Setup' names a brief file, create it now before invoking."
print_invocation
