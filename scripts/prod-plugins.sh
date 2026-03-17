#!/usr/bin/env bash

# Restore the original augentic marketplace.
# Usage: ./scripts/prod-plugins.sh

set -euo pipefail

SPECIFY_DIR="$HOME/.cursor/plugins/marketplaces/github.com/augentic/specify"

rm -rf "$SPECIFY_DIR"/*

echo ""
echo "Reload Cursor (or restart) to pick up production plugins."
