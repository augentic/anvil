#!/usr/bin/env bash

# Symlink local plugins into Cursor's marketplace so edits are picked up on reload.
# Also seeds the plugin cache for any plugins not yet published to the remote.
# Usage: ./scripts/dev-plugins.sh

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SPECIFY_DIR="$HOME/.cursor/plugins/marketplaces/github.com/augentic/specify"
CACHE_DIR="$HOME/.cursor/plugins/cache/augentic"

# clear stale cache
rm -rf "$CACHE_DIR"

# remove whatever is at SPECIFY_DIR (symlink or directory) and create a fresh directory
rm -rf "$SPECIFY_DIR"
mkdir -p "$SPECIFY_DIR"
ln -sfn "$REPO_ROOT" "$SPECIFY_DIR/local"

# Seed cache for every plugin listed in marketplace.json.
# Cursor discovers plugins from the remote, so plugins not yet merged to main
# won't appear in cache after reload. We pre-populate them here via symlinks
# so they are available immediately.
MARKETPLACE="$REPO_ROOT/.cursor-plugin/marketplace.json"
if [ ! -f "$MARKETPLACE" ]; then
  echo "Warning: marketplace.json not found at $MARKETPLACE" >&2
  echo "Reload Cursor (or restart) to pick up local plugins."
  exit 0
fi

PLUGIN_ROOT=$(python3 -c "
import json, sys
m = json.load(open(sys.argv[1]))
print(m.get('metadata', {}).get('pluginRoot', 'plugins'))
" "$MARKETPLACE")

PLUGINS=$(python3 -c "
import json, sys
m = json.load(open(sys.argv[1]))
for p in m.get('plugins', []):
    print(p['source'])
" "$MARKETPLACE")

HASH="local"

for plugin in $PLUGINS; do
  plugin_src="$REPO_ROOT/$PLUGIN_ROOT/$plugin"
  cache_dest="$CACHE_DIR/$plugin/$HASH"
  if [ -d "$plugin_src" ] && [ ! -d "$cache_dest" ]; then
    mkdir -p "$(dirname "$cache_dest")"
    ln -sfn "$plugin_src" "$cache_dest"
  fi
done

echo ""
echo "Seeded cache for: $PLUGINS"
echo "Reload Cursor (or restart) to pick up local plugins."
