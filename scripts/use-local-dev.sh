#!/usr/bin/env bash

# Build the specify-cli binaries and WASI tools from source, install them,
# and populate the Cursor plugin cache from the local working tree.  Run this
# once after cloning (or after pulling changes) to work against your local
# repos instead of the published versions.
#
# Assumes the standard sibling layout:
#   augentic/specify/      (this repo — plugins, skills, references)
#   augentic/specify-cli/  (the CLI workspace — specify, wasi-tools)
#
# Usage: bash ./scripts/use-local-dev.sh [--skip-wasi]
#
# Options:
#   --skip-wasi       Skip building WASI tools (faster iteration on CLI/skills only)
#
# Environment:
#   SPECIFY_BIN_DIR   — where to install specify (default: ~/.local/bin)

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
WASI_TOOLS_DIR="$CLI_ROOT/wasi-tools"

# ── Helpers ──────────────────────────────────────────────────

adapter_tool_version() {
  awk '/^tools:/{found=1} found && /version:/{gsub(/[" ]/, "", $2); print $2; exit}' "$1"
}

# Write a tools.yaml sidecar with the correct first-party permissions.
# Permissions must match specify_tool::manifest::first_party_permissions().
write_sidecar() {
  local tool_name="$1" version="$2" source_path="$3" dest="$4"
  case "$tool_name" in
    vectis)
      cat > "$dest" <<YAML
tools:
  - name: vectis
    version: "${version}"
    source: "${source_path}"
    permissions:
      read:
        - \$PROJECT_DIR
        - \$CAPABILITY_DIR
      write:
        - \$PROJECT_DIR
YAML
      ;;
    contract)
      cat > "$dest" <<YAML
tools:
  - name: contract
    version: "${version}"
    source: "${source_path}"
    permissions:
      read:
        - \$PROJECT_DIR/contracts
YAML
      ;;
    *) echo "Warning: unknown tool $tool_name, skipping sidecar" >&2; return 1 ;;
  esac
}

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

# ── Build and install CLI binaries ────────────────────────────

echo "Building specify from $CLI_ROOT …"
cargo build --release --manifest-path "$CLI_ROOT/Cargo.toml"

bin=specify
src="$CLI_ROOT/target/release/$bin"
if [ ! -f "$src" ]; then
  echo "Error: $bin not found at $src" >&2
  exit 1
fi
cp "$src" "$INSTALL_DIR/$bin"
echo "Installed $bin → $INSTALL_DIR/$bin"

if ! echo "$PATH" | tr ':' '\n' | grep -qx "$INSTALL_DIR"; then
  echo "Warning: $INSTALL_DIR is not on your PATH."
  echo "         Add to your shell profile: export PATH=\"$INSTALL_DIR:\$PATH\""
else
  resolved="$(command -v "$bin" 2>/dev/null || true)"
  if [ -n "$resolved" ] && [ "$resolved" != "$INSTALL_DIR/$bin" ]; then
    echo "Warning: $bin resolves to $resolved, which shadows $INSTALL_DIR/$bin."
    echo "         Move $INSTALL_DIR earlier on your PATH or remove the other copy."
  fi
fi

# ── Build WASI tools ─────────────────────────────────────────
#
# First-party WASI tools declared by target adapters.  Each entry is
# cargo_pkg|bin_name|adapter_dir|tool_name.  Omnia declares no tools.

WASI_TOOLS=(
  "specify-vectis|vectis|vectis|vectis"
  "specify-contract|specify-contract|contracts|contract"
)

if [ "$SKIP_WASI" = "1" ]; then
  echo "Skipping WASI tool build (--skip-wasi)."
elif [ ! -d "$WASI_TOOLS_DIR" ]; then
  echo "Warning: wasi-tools/ not found in specify-cli, skipping WASI build" >&2
elif ! rustup target list --installed 2>/dev/null | grep -q wasm32-wasip2; then
  echo "Warning: wasm32-wasip2 target not installed, skipping WASI build" >&2
  echo "         Install with: rustup target add wasm32-wasip2"
else
  for entry in "${WASI_TOOLS[@]}"; do
    IFS='|' read -r cargo_pkg bin_name adapter_dir tool_name <<< "$entry"
    adapter_path="$REPO_ROOT/adapters/targets/$adapter_dir"
    wasm_file="$WASI_TOOLS_DIR/target/wasm32-wasip2/release/$bin_name.wasm"

    echo "Building $tool_name WASI tool …"
    (cd "$WASI_TOOLS_DIR" && cargo build -p "$cargo_pkg" --target wasm32-wasip2 --release)

    if [ ! -f "$wasm_file" ]; then
      echo "Warning: $bin_name.wasm not found after build, skipping sidecar" >&2
      continue
    fi

    wasm_abs="$(cd "$(dirname "$wasm_file")" && pwd)/$(basename "$wasm_file")"
    version=$(adapter_tool_version "$adapter_path/adapter.yaml")
    write_sidecar "$tool_name" "$version" "$wasm_abs" "$adapter_path/tools.yaml"
    echo "Installed $tool_name sidecar → $adapter_path/tools.yaml"
  done
fi

# ── Populate plugin cache ─────────────────────────────────────

bash "$REPO_ROOT/scripts/use-local-plugins.sh"

# ── Summary ───────────────────────────────────────────────────

echo ""
echo "Local dev environment ready."
specify_path="$(command -v specify 2>/dev/null || echo "$INSTALL_DIR/specify")"
echo "  specify: $specify_path ($("$specify_path" --version 2>/dev/null || echo "version unknown"))"
for entry in "${WASI_TOOLS[@]}"; do
  IFS='|' read -r _ _ adapter_dir tool_name <<< "$entry"
  sidecar="$REPO_ROOT/adapters/targets/$adapter_dir/tools.yaml"
  if [ -f "$sidecar" ]; then
    echo "  $tool_name:  $(grep 'source:' "$sidecar" | sed 's/.*source: *//' | tr -d '"')"
  fi
done
echo ""
echo "Next steps:"
echo "  1. Restart Cursor to pick up local plugin changes."
echo "  2. Open your project (e.g. ../todo-app) in Cursor."
echo "  3. Run /spec:init to scaffold .specify/ and bind adapters."
