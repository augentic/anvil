#!/usr/bin/env bash

# Replace the local plugin cache with copies from the working tree so skill,
# rule, and reference changes can be tested before pushing to main.
#
# Cursor only rebuilds the cache when it is missing. By pre-populating it
# with local content, the agent will use your working-tree versions on the
# next restart.
#
# Usage: bash ./scripts/use-local-plugins.sh

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CACHE_DIR="$HOME/.cursor/plugins/cache/augentic"

MARKETPLACE="$REPO_ROOT/.cursor-plugin/marketplace.json"
if [ ! -f "$MARKETPLACE" ]; then
  echo "Error: marketplace.json not found at $MARKETPLACE" >&2
  exit 1
fi

if ! command -v jq &>/dev/null; then
  echo "Error: jq not found — install jq (https://jqlang.github.io/jq/)" >&2
  exit 1
fi

PLUGIN_ROOT=$(jq -r '.metadata.pluginRoot // "plugins"' "$MARKETPLACE")
PLUGINS=$(jq -r '.plugins[].source' "$MARKETPLACE")

# Intentional: clear only the augentic-scoped cache so it is repopulated below;
# CACHE_DIR is fixed to ~/.cursor/plugins/cache/augentic, never the whole cache.
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
