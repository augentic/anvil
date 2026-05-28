#!/usr/bin/env bash

# Build the specify-cli binaries from source, install them, and populate the
# Cursor plugin cache from the local working tree.  Run this once after
# cloning (or after pulling changes) to work against your local repos instead
# of the published versions.
#
# Assumes the standard sibling layout:
#   augentic/specify/      (this repo — plugins, skills, references)
#   augentic/specify-cli/  (the CLI workspace — specrun, specdev)
#
# Usage: bash ./scripts/use-local-dev.sh

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CLI_ROOT="$REPO_ROOT/../specify-cli"
INSTALL_DIR="${SPECIFY_BIN_DIR:-$HOME/.local/bin}"

# ── Pre-flight ────────────────────────────────────────────────

if [ ! -f "$CLI_ROOT/Cargo.toml" ]; then
  echo "Error: specify-cli not found at $CLI_ROOT" >&2
  echo "       Expected sibling layout: augentic/specify/ + augentic/specify-cli/" >&2
  exit 1
fi

if ! command -v cargo &>/dev/null; then
  echo "Error: cargo not found — install Rust via https://rustup.rs" >&2
  exit 1
fi

mkdir -p "$INSTALL_DIR"

# ── Step 1: Build the CLI ─────────────────────────────────────

echo "Building specrun + specdev from $CLI_ROOT …"
cargo build --release --manifest-path "$CLI_ROOT/Cargo.toml"

# ── Step 2: Install binaries ──────────────────────────────────

for bin in specrun specdev; do
  src="$CLI_ROOT/target/release/$bin"
  if [ ! -f "$src" ]; then
    echo "Warning: $bin not found at $src, skipping" >&2
    continue
  fi
  cp "$src" "$INSTALL_DIR/$bin"
  echo "Installed $bin → $INSTALL_DIR/$bin"
done

if ! echo "$PATH" | tr ':' '\n' | grep -qx "$INSTALL_DIR"; then
  echo ""
  echo "⚠  $INSTALL_DIR is not on your PATH."
  echo "   Add it to your shell profile:"
  echo "     export PATH=\"$INSTALL_DIR:\$PATH\""
fi

# ── Step 3: Populate plugin cache ─────────────────────────────

echo ""
bash "$REPO_ROOT/scripts/use-local-plugins.sh"

# ── Done ──────────────────────────────────────────────────────

echo ""
echo "Local dev environment ready."
specrun_path="$(command -v specrun 2>/dev/null || echo "$INSTALL_DIR/specrun")"
echo "  specrun: $specrun_path ($("$specrun_path" --version 2>/dev/null || echo "version unknown"))"
echo ""
echo "Next steps:"
echo "  1. Restart Cursor to pick up local plugin changes."
echo "  2. Open your project (e.g. ../todo-app) in Cursor."
echo "  3. Run /spec:init to scaffold .specify/ and bind adapters."
