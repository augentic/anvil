#!/usr/bin/env bash

# Symlink local plugins into Cursor's marketplace so edits are picked up on reload.
# Usage: ./scripts/dev-plugins.sh

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PLUGINS_DIR="$HOME/.cursor/plugins"
MARKETPLACES_DIR="$PLUGINS_DIR/marketplaces"

# get all augentic-* directories in the marketplaces directory
shopt -s nullglob
dirs=("$MARKETPLACES_DIR"/augentic-*/)
shopt -u nullglob

# if there are no augentic-* directories, create a local directory
if [ ${#dirs[@]} -eq 0 ]; then
  mkdir -p "$MARKETPLACES_DIR/augentic-local"
  dirs=("$MARKETPLACES_DIR/augentic-local/")
fi

# symlink this repo to each augentic-*/main
for dir in "${dirs[@]}"; do
  rm -rf "$dir/main"
  ln -sfn "$REPO_ROOT" "$dir/main"
done

# clear the cache
rm -rf "$PLUGINS_DIR/cache/augentic"

echo ""
echo "Reload Cursor (or restart) to pick up local plugins."

