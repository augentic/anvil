#!/usr/bin/env bash

# Documentation consistency checks for the Augentic Plugins repository.
# Run via: make checks
# Resolves deno, installs if missing, then runs checks.ts.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

if command -v deno >/dev/null 2>&1; then
  DENO="$(command -v deno)"
elif [ -x "$HOME/.deno/bin/deno" ]; then
  DENO="$HOME/.deno/bin/deno"
else
  echo "Deno not found — installing..."
  curl -fsSL https://deno.land/install.sh | sh
  DENO="$HOME/.deno/bin/deno"
fi

"$DENO" run --allow-read "$SCRIPT_DIR/checks.ts"
