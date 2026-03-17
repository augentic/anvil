#!/usr/bin/env bash

# Restore the original augentic marketplace.
# Usage: ./scripts/prod-plugins.sh

set -euo pipefail

PLUGINS_DIR="$HOME/.cursor/plugins"

rm -rf "$PLUGINS_DIR"/marketplaces/github.com/augentic/specify/local
rm -rf "$PLUGINS_DIR"/cache/augentic

echo ""
echo "Reload Cursor (or restart) to pick up production plugins."
