#!/usr/bin/env bash

# Symlink local plugins into Cursor's marketplace so edits are picked up on reload.
# Usage: ./scripts/dev-plugins.sh

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SPECIFY_DIR="$HOME/.cursor/plugins/marketplaces/github.com/augentic/specify"

rm -rf "$SPECIFY_DIR"/*
mkdir -p "$SPECIFY_DIR"
ln -sfn "$REPO_ROOT" "$SPECIFY_DIR/local"

# clear cache
rm -rf "$HOME/.cursor/plugins/cache/augentic"

echo ""
echo "Reload Cursor (or restart) to pick up local plugins."
