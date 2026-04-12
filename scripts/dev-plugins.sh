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
DENO="${DENO:-$(command -v deno 2>/dev/null || echo "$HOME/.deno/bin/deno")}"

MARKETPLACE="$REPO_ROOT/.cursor-plugin/marketplace.json"
if [ ! -f "$MARKETPLACE" ]; then
  echo "Error: marketplace.json not found at $MARKETPLACE" >&2
  exit 1
fi

PLUGIN_ROOT=$("$DENO" eval "
  const m = JSON.parse(Deno.readTextFileSync('$MARKETPLACE'));
  console.log(m?.metadata?.pluginRoot ?? 'plugins');
")

PLUGINS=$("$DENO" eval "
  const m = JSON.parse(Deno.readTextFileSync('$MARKETPLACE'));
  for (const p of m?.plugins ?? []) console.log(p.source);
")

rm -rf "$CACHE_DIR"

for plugin in $PLUGINS; do
  src="$REPO_ROOT/$PLUGIN_ROOT/$plugin"

  if [ ! -d "$src" ]; then
    echo "Warning: $src not found, skipping" >&2
    continue
  fi

  # Populate the "main" slot used by dev mode
  dest="$CACHE_DIR/$plugin/main"
  mkdir -p "$dest"
  cp -R "$src/." "$dest/"

  # Also populate any commit-hash slots Cursor may have cached so the
  # active cache entry is overwritten regardless of which slot is live.
  for hashdir in "$CACHE_DIR/$plugin"/*/; do
    [ "$hashdir" = "$dest/" ] && continue
    [ -d "$hashdir" ] || continue
    rm -rf "$hashdir"
    mkdir -p "$hashdir"
    cp -R "$src/." "$hashdir/"
  done

  echo "Cached $plugin from local source"
done

echo ""
echo "Restart Cursor to pick up local plugin changes."
