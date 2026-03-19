#!/usr/bin/env bash

# Symlink local plugins into Cursor's marketplace so edits are picked up on reload.
# Also seeds the plugin cache for any plugins not yet published to the remote.
# Usage: ./scripts/dev-plugins.sh

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SPECIFY_DIR="$HOME/.cursor/plugins/marketplaces/github.com/augentic/specify"
CACHE_DIR="$HOME/.cursor/plugins/cache/augentic"

# Remove only local symlinks from previous runs; preserve hash-based entries
# that Cursor created from the remote (needed to mirror unpublished plugins).
if [ -d "$CACHE_DIR" ]; then
  find "$CACHE_DIR" -maxdepth 2 -name local -type l -delete
fi

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

# Mirror hash-based entries for plugins not yet on the remote.
# Cursor discovers skills only from hash-named directories, so unpublished
# plugins need an entry at the same hash the remote uses.
EXISTING_HASH=""
for dir in "$CACHE_DIR"/*/; do
  for entry in "$dir"*/; do
    name=$(basename "$entry")
    if [ "$name" != "local" ]; then
      EXISTING_HASH="$name"
      break 2
    fi
  done
done

if [ -n "$EXISTING_HASH" ]; then
  for plugin in $PLUGINS; do
    hash_dest="$CACHE_DIR/$plugin/$EXISTING_HASH"
    if [ ! -e "$hash_dest" ]; then
      ln -sfn "$REPO_ROOT/$PLUGIN_ROOT/$plugin" "$hash_dest"
      echo "Mirrored $plugin at hash $EXISTING_HASH"
    fi
  done
fi

echo ""
echo "Seeded cache for: $PLUGINS"
if [ -z "$EXISTING_HASH" ]; then
  echo "No remote hash found yet. Reload Cursor, then run this script again"
  echo "to mirror unpublished plugins at the correct hash."
else
  echo "Remote hash: $EXISTING_HASH"
fi
echo "Reload Cursor (or restart) to pick up local plugins."
