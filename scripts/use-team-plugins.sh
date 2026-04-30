#!/usr/bin/env bash

# Clear the local plugin cache so Cursor refetches from the server on next
# restart. Use this to switch from local plugins back to the published versions.
#
# Usage: bash ./scripts/use-team-plugins.sh

set -euo pipefail

rm -rf "$HOME/.cursor/plugins/cache/augentic"

echo ""
echo "Restart Cursor to refetch published plugins from the server."
