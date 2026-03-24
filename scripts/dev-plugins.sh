#!/usr/bin/env bash

# Replace the local plugin cache with copies from the working tree so skill,
# rule, and reference changes can be tested before pushing to main.
#
# Cursor only rebuilds the cache when it is missing. By pre-populating it
# with local content, the agent will use your working-tree versions on the
# next restart.
#
# Usage: ./scripts/dev-plugins.sh

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CACHE_DIR="$HOME/.cursor/plugins/cache/augentic"

MARKETPLACE="$REPO_ROOT/.cursor-plugin/marketplace.json"
if [ ! -f "$MARKETPLACE" ]; then
  echo "Error: marketplace.json not found at $MARKETPLACE" >&2
  exit 1
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

rm -rf "$CACHE_DIR"

for plugin in $PLUGINS; do
  src="$REPO_ROOT/$PLUGIN_ROOT/$plugin"
  dest="$CACHE_DIR/$plugin/main"

  if [ ! -d "$src" ]; then
    echo "Warning: $src not found, skipping" >&2
    continue
  fi

  mkdir -p "$dest"
  cp -R "$src/." "$dest/"
  echo "Cached $plugin from local source"
done

echo ""
echo "Restart Cursor to pick up local plugin changes."
