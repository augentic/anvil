#!/usr/bin/env bash

# Restore the original augentic marketplace.
# Usage: ./scripts/prod-plugins.sh

set -euo pipefail

# remove all marketplaces/github.com/augentic/specify/* directories
SPECIFY_DIR="$HOME/.cursor/plugins/marketplaces/github.com/augentic/specify"

rm -rf "$SPECIFY_DIR"/*/

# clear the cache
# rm -rf "$PLUGINS_DIR/cache/augentic"

echo ""
echo "Reload Cursor (or restart) to pick up production plugins."