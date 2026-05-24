#!/usr/bin/env bash
# Install (or upgrade) every binary the docs/ mdbook build relies on.
#
# Pinned versions match the ones documented in docs/README.md. Bump both files
# together when you upgrade a tool.

set -euo pipefail

# The mdbook 0.4 line is the one the preprocessor ecosystem currently aligns
# with (mdbook-linkcheck / mdbook-template / mdbook-pagetoc all target it).
# Bump these in lock-step when the ecosystem catches up with mdbook 0.5.
MDBOOK_VERSION="${MDBOOK_VERSION:-0.4.52}"
MDBOOK_D2_VERSION="${MDBOOK_D2_VERSION:-0.3.4}"
MDBOOK_LINKCHECK_VERSION="${MDBOOK_LINKCHECK_VERSION:-0.7.7}"
MDBOOK_PAGETOC_VERSION="${MDBOOK_PAGETOC_VERSION:-0.2.0}"
MDBOOK_TEMPLATE_VERSION="${MDBOOK_TEMPLATE_VERSION:-1.1.1}"

cargo_install() {
  local crate="$1" version="$2"
  if command -v "$crate" >/dev/null 2>&1; then
    echo "[docs-prereqs] $crate already installed; re-running cargo install for version pin"
  fi
  cargo install --locked --version "$version" "$crate"
}

cargo_install mdbook            "$MDBOOK_VERSION"
cargo_install mdbook-d2         "$MDBOOK_D2_VERSION"
cargo_install mdbook-linkcheck  "$MDBOOK_LINKCHECK_VERSION"
cargo_install mdbook-pagetoc    "$MDBOOK_PAGETOC_VERSION"
cargo_install mdbook-template   "$MDBOOK_TEMPLATE_VERSION"

if ! command -v d2 >/dev/null 2>&1; then
  echo "[docs-prereqs] d2 not found; installing via the upstream script"
  curl -fsSL https://d2lang.com/install.sh | sh -s --
else
  echo "[docs-prereqs] d2 already installed ($(d2 --version 2>/dev/null | head -1))"
fi

echo "[docs-prereqs] All tools installed. Run 'make docs-serve' from the repo root."
