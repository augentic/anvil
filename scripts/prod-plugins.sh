#!/usr/bin/env bash

# Restore the original augentic marketplace.
# Usage: ./scripts/prod-plugins.sh

set -euo pipefail

PLUGINS_DIR="$HOME/.cursor/plugins"
MARKETPLACES_DIR="$PLUGINS_DIR/marketplaces"

# remove all marketplaces/augentic-* directories
shopt -s nullglob
for dir in "$MARKETPLACES_DIR"/augentic-*/; do
  rm -rf "$dir"
done
shopt -u nullglob

# clear the cache
rm -rf "$PLUGINS_DIR/cache/augentic"

echo ""
echo "Reload Cursor (or restart) to pick up production plugins."
