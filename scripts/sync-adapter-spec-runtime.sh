#!/usr/bin/env bash
# Materialise deployable spec-runtime references into each adapter's
# references/spec-runtime/ so monorepo link checks match post-init cache layout.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
RUNTIME_SRC="$ROOT/adapters/shared/references/runtime"
PLUGINS_REF="$ROOT/plugins/spec/references"
SYNTHESIS_RUNTIME_FILES=(authority.md tags.md provenance.md claim-reconciliation.md)

resolve_path() {
  local path="$1"
  local dir
  while [[ -L "$path" ]]; do
    dir="$(dirname "$path")"
    path="$(readlink "$path")"
    [[ "$path" != /* ]] && path="$dir/$path"
  done
  echo "$(cd "$(dirname "$path")" && pwd)/$(basename "$path")"
}

materialise_adapter() {
  local dest="$1/references/spec-runtime"
  rm -rf "$dest"
  mkdir -p "$dest/synthesis" "$dest/cli"

  for entry in "$RUNTIME_SRC"/*; do
    local name
    name="$(basename "$entry")"
    [[ "$name" == "README.md" ]] && continue
    [[ "$name" == "cli" || "$name" == "synthesis" ]] && continue
    local target="$dest/$name"
    if [[ -L "$entry" ]]; then
      cp "$(resolve_path "$entry")" "$target"
    elif [[ -f "$entry" ]]; then
      cp "$entry" "$target"
    fi
  done

  for file in "${SYNTHESIS_RUNTIME_FILES[@]}"; do
    cp "$PLUGINS_REF/synthesis/$file" "$dest/synthesis/$file"
  done
  cp "$PLUGINS_REF/cli/plan-propose.md" "$dest/cli/plan-propose.md"
  cp "$ROOT/plugins/spec/skills/execute/references/stop-conditions.md" "$dest/stop-conditions.md"
  cp "$PLUGINS_REF/plan-lock.md" "$dest/plan-lock.md"
  sed -i '' 's|../../../references/plan-lock.md|./plan-lock.md|g' "$dest/stop-conditions.md" 2>/dev/null || \
    sed -i 's|../../../references/plan-lock.md|./plan-lock.md|g' "$dest/stop-conditions.md"
  cp "$ROOT/docs/reference/review-team-protocol.md" "$dest/review-team-protocol.md"
}

for axis in sources targets; do
  for adapter in "$ROOT/adapters/$axis"/*/; do
    [[ -d "$adapter" ]] || continue
    name="$(basename "$adapter")"
    [[ "$name" == "shared" ]] && continue
    materialise_adapter "$adapter"
  done
done

echo "synced spec-runtime into adapters/{sources,targets}/*/references/"
