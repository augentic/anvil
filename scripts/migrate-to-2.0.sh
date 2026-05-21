#!/usr/bin/env bash

# Migrate a Specify 1.x project tree to 2.0 in place.
#
# Usage:
#   ./scripts/migrate-to-2.0.sh [--dry-run] [project-root]
#
# Defaults project-root to $PWD. Idempotent; safe to re-run.
#
# Requires Deno (https://deno.land). Looks on PATH first, then at
# $HOME/.deno/bin/deno -- the same fallback the repo Makefile uses.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

DENO="${DENO:-}"
if [ -z "$DENO" ]; then
  if command -v deno >/dev/null 2>&1; then
    DENO="$(command -v deno)"
  elif [ -x "$HOME/.deno/bin/deno" ]; then
    DENO="$HOME/.deno/bin/deno"
  else
    cat <<'MSG' >&2
migrate-to-2.0: Deno not found on PATH.

Install Deno (https://docs.deno.com/runtime/getting_started/installation/)
or set DENO=/path/to/deno before re-running. The migration is one-shot;
you can uninstall Deno afterwards.
MSG
    exit 1
  fi
fi

# We need read+write on the project root, and read on this script's dir
# to load the .ts entry point. Net is denied so the script cannot phone
# home; --no-prompt keeps a TTY confirmation from blocking the run.
exec "$DENO" run --quiet --no-prompt --allow-read --allow-write \
  "$SCRIPT_DIR/migrate_to_2_0.ts" "$@"
