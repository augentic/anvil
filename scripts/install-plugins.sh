#!/usr/bin/env bash

# Install plugins from this repository for local development and testing.
# This preserves plugin namespacing (spec:apply, omnia:crate-writer) and interdependencies.
#
# Usage:
# 
# ```bash
#  chmod +x ./scripts/install-plugins.sh
#  ./scripts/install-plugins.sh
# ```
#
# Plugins will be available from any project as the scipt modifies files in the $HOME directory.
# 
# After running, restart Cursor to load the local plugins. Any edits in this repo are picked up
# on next restart as plugins are symlinked.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CURSOR_PLUGINS="${CURSOR_PLUGINS:-$HOME/.cursor/plugins/local}"
CLAUDE_DIR="${CLAUDE_DIR:-$HOME/.claude}"
CLAUDE_PLUGINS="$CLAUDE_DIR/plugins/installed_plugins.json"
CLAUDE_SETTINGS="$CLAUDE_DIR/settings.json"

PLUGINS=(spec omnia rt plan)

command -v python3 >/dev/null || { echo "python3 required"; exit 1; }

# 1. Symlink plugins
echo "Symlinking plugins to $CURSOR_PLUGINS"
mkdir -p "$CURSOR_PLUGINS"
for plugin in "${PLUGINS[@]}"; do
  source="$REPO_ROOT/plugins/$plugin"
  if [ ! -d "$source" ]; then
    echo "Warning: $source not found, skipping"
    continue
  fi
  target="$CURSOR_PLUGINS/$plugin"
  if [ -L "$target" ] || [ -e "$target" ]; then
    rm -rf "$target"
  fi
  ln -sf "$source" "$target"
  echo "  $plugin -> $source"
done

# 2. Upsert `installed_plugins.json`` (merge with existing)
#    in ~/.claude/plugins/installed_plugins.json
echo
echo "Updating $CLAUDE_PLUGINS"
python3 - "$CLAUDE_PLUGINS" "$CURSOR_PLUGINS" "${PLUGINS[@]}" <<'PY'
import json, os, sys

path = sys.argv[1]
base = sys.argv[2]
plugins_to_install = sys.argv[3:]

data = {}
if os.path.exists(path):
    try:
        with open(path) as f:
            data = json.load(f)
    except json.JSONDecodeError:
        pass

plugins = data.get("plugins", {})

for name in plugins_to_install:
    pid = f"{name}@local"
    # Remove existing user-scope entries for this plugin
    entries = [e for e in plugins.get(pid, [])
               if not (isinstance(e, dict) and e.get("scope") == "user")]
    entries.insert(0, {"scope": "user", "installPath": f"{base}/{name}"})
    plugins[pid] = entries

data["plugins"] = plugins
os.makedirs(os.path.dirname(path), exist_ok=True)
with open(path, "w") as f:
    json.dump(data, f, indent=2)
PY

# 3. Upsert `enabledPlugins` in `settings.json`
#    Enable @local plugins and disable conflicting Augentic marketplace plugins
echo "Updating $CLAUDE_SETTINGS"
python3 - "$CLAUDE_SETTINGS" "${PLUGINS[@]}" <<'PY'
import json, os, sys

path = sys.argv[1]
plugins_to_enable = sys.argv[2:]

data = {}
if os.path.exists(path):
    try:
        with open(path) as f:
            data = json.load(f)
    except json.JSONDecodeError:
        pass

enabled = data.setdefault("enabledPlugins", {})
local_plugins = set(plugins_to_enable)

# Enable local plugins
for name in local_plugins:
    enabled[f"{name}@local"] = True

# Disable Augentic marketplace plugins that conflict with local versions
# (e.g. spec@augentic, omnia@augentic)
for pid in list(enabled.keys()):
    base = pid.split("@")[0] if "@" in pid else ""
    suffix = pid.split("@")[-1] if "@" in pid else ""
    if base in local_plugins and suffix != "local":
        enabled[pid] = False

data["enabledPlugins"] = enabled

os.makedirs(os.path.dirname(path), exist_ok=True)
with open(path, "w") as f:
    json.dump(data, f, indent=2)
PY

echo
echo "Done. Restart Cursor (or Reload Window) to load local plugins."
echo
echo "Installed: ${PLUGINS[*]}"
echo "Skills will appear as /spec:apply, /omnia:crate-writer, etc."
echo "Marketplace versions of these plugins have been disabled to avoid conflicts."
