#!/usr/bin/env bash

# Build the specify-cli binaries and WASI tools from source, install them,
# and populate the Cursor plugin cache from the local working tree.  Run this
# once after cloning (or after pulling changes) to work against your local
# repos instead of the published versions.
#
# Assumes the standard sibling layout:
#   augentic/specify/      (this repo — plugins, skills, references)
#   augentic/specify-cli/  (the CLI workspace — specrun, specdev, wasi-tools)
#
# Usage: bash ./scripts/use-local-dev.sh [--skip-wasi]
#
# Options:
#   --skip-wasi       Skip building WASI tools (faster iteration on CLI/skills only)
#
# Environment:
#   SPECIFY_BIN_DIR   — where to install specrun/specdev (default: ~/.local/bin)

set -euo pipefail

SKIP_WASI=0
for arg in "$@"; do
  case "$arg" in
    --skip-wasi) SKIP_WASI=1 ;;
    *)
      echo "Unknown option: $arg" >&2
      echo "Usage: bash ./scripts/use-local-dev.sh [--skip-wasi]" >&2
      exit 1
      ;;
  esac
done

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

# ── Step 3: Build WASI tools ─────────────────────────────────

VECTIS_ADAPTER_DIR="$REPO_ROOT/adapters/targets/vectis"

if [ "$SKIP_WASI" = "1" ]; then
  echo ""
  echo "Skipping WASI tool build (--skip-wasi)."
else
  WASI_TOOLS_DIR="$CLI_ROOT/wasi-tools"
  if [ ! -d "$WASI_TOOLS_DIR" ]; then
    echo ""
    echo "Warning: wasi-tools/ not found in specify-cli, skipping WASI build" >&2
  elif ! rustup target list --installed 2>/dev/null | grep -q wasm32-wasip2; then
    echo ""
    echo "Warning: wasm32-wasip2 target not installed, skipping WASI build" >&2
    echo "         Install with: rustup target add wasm32-wasip2"
  else
    echo ""
    echo "Building vectis WASI tool …"
    (cd "$WASI_TOOLS_DIR" && cargo build -p specify-vectis --target wasm32-wasip2 --release)
    VECTIS_WASM="$WASI_TOOLS_DIR/target/wasm32-wasip2/release/vectis.wasm"
    if [ -f "$VECTIS_WASM" ]; then
      VECTIS_WASM_ABS="$(cd "$(dirname "$VECTIS_WASM")" && pwd)/$(basename "$VECTIS_WASM")"
      VECTIS_VERSION=$(awk '/^tools:/{found=1} found && /version:/{gsub(/[" ]/, "", $2); print $2; exit}' \
        "$VECTIS_ADAPTER_DIR/adapter.yaml")
      cat > "$VECTIS_ADAPTER_DIR/tools.yaml" <<EOF
tools:
  - name: vectis
    version: "${VECTIS_VERSION}"
    source: "${VECTIS_WASM_ABS}"
    permissions:
      read:
        - \$PROJECT_DIR
        - \$CAPABILITY_DIR
      write:
        - \$PROJECT_DIR
EOF
      echo "Installed vectis.wasm sidecar → $VECTIS_ADAPTER_DIR/tools.yaml"
    else
      echo "Warning: vectis.wasm not found after build, skipping sidecar" >&2
    fi
  fi
fi

# ── Step 4: Populate plugin cache ─────────────────────────────

echo ""
bash "$REPO_ROOT/scripts/use-local-plugins.sh"

# ── Done ──────────────────────────────────────────────────────

echo ""
echo "Local dev environment ready."
specrun_path="$(command -v specrun 2>/dev/null || echo "$INSTALL_DIR/specrun")"
echo "  specrun: $specrun_path ($("$specrun_path" --version 2>/dev/null || echo "version unknown"))"
if [ -f "$VECTIS_ADAPTER_DIR/tools.yaml" ]; then
  echo "  vectis:  $(grep 'source:' "$VECTIS_ADAPTER_DIR/tools.yaml" | sed 's/.*source: *//')"
fi
echo ""
echo "Next steps:"
echo "  1. Restart Cursor to pick up local plugin changes."
echo "  2. Open your project (e.g. ../todo-app) in Cursor."
echo "  3. Run /spec:init to scaffold .specify/ and bind adapters."
