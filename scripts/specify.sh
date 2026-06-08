#!/usr/bin/env bash
#
# Central resolver + runner that binds the framework repo to a `specify`
# binary per SPECIFY_VERSION. See docs/contributing/checks.md#binding-to-a-specify-binary.
#
# Usage:
#   scripts/specify.sh lint                   # → lint framework --framework-root .
#   scripts/specify.sh --mode bin-path        # print resolved binary path (SPECIFY_VERSION-driven)
#
# Env:
#   SPECIFY_VERSION  next | X.Y.Z   (default: next)
#   REPO_ROOT        optional; defaults to git root / script parent

set -euo pipefail

SPECIFY_VERSION="${SPECIFY_VERSION:-next}"

# ── Repo root ────────────────────────────────────────────────

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
if [ -z "${REPO_ROOT:-}" ]; then
  REPO_ROOT="$(git -C "$script_dir" rev-parse --show-toplevel 2>/dev/null || true)"
  [ -n "$REPO_ROOT" ] || REPO_ROOT="$(cd "$script_dir/.." && pwd)"
fi

BIN_DIR="$REPO_ROOT/.bin"
SPECIFY_LOCAL="$BIN_DIR/specify"
PIN_FILE="$REPO_ROOT/.specify-version"

# ── Helpers ──────────────────────────────────────────────────

note() { printf 'specify.sh: %s\n' "$1" >&2; }
die() { printf 'specify.sh: error: %s\n' "$1" >&2; exit 1; }

# firstword(wildcard specify-cli/Cargo.toml ../specify-cli/Cargo.toml)
find_manifest() {
  local candidate
  for candidate in "$REPO_ROOT/specify-cli/Cargo.toml" "$REPO_ROOT/../specify-cli/Cargo.toml"; do
    if [ -f "$candidate" ]; then
      printf '%s\n' "$candidate"
      return 0
    fi
  done
  return 1
}

# Trim .specify-version to the bare semver pin.
read_pin() {
  [ -f "$PIN_FILE" ] || die ".specify-version not found at $PIN_FILE"
  local pin
  pin="$(tr -d '[:space:]' < "$PIN_FILE")"
  [ -n "$pin" ] || die ".specify-version is empty"
  printf '%s\n' "$pin"
}

# Last whitespace token of `<bin> --version` (e.g. "specify 0.1.0" → "0.1.0").
path_specify_version() {
  "$1" --version 2>/dev/null | awk '{print $NF}'
}

validate_semver() {
  [[ "$1" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]
}

# Echo -1 / 0 / 1 for (a<b) / (a==b) / (a>b) using version sort.
version_cmp() {
  local a="$1" b="$2" highest
  if [ "$a" = "$b" ]; then echo 0; return; fi
  highest="$(printf '%s\n%s\n' "$a" "$b" | sort -V | tail -n1)"
  if [ "$highest" = "$a" ]; then echo 1; else echo -1; fi
}

# version_satisfies <have> <op> <want>   op ∈ eq | ge
version_satisfies() {
  local have="$1" op="$2" want="$3"
  [ -n "$have" ] || return 1
  case "$op" in
    eq) [ "$have" = "$want" ] ;;
    ge) [ "$(version_cmp "$have" "$want")" != "-1" ] ;;
    *) return 1 ;;
  esac
}

# Install the requested semver into ./.bin via cargo install --git.
# Idempotent: skips work when ./.bin/specify already satisfies the pin.
acquire() {
  local want="$1" have
  if [ -x "$SPECIFY_LOCAL" ]; then
    have="$(path_specify_version "$SPECIFY_LOCAL" || true)"
    if version_satisfies "$have" ge "$want"; then
      return 0
    fi
  fi

  mkdir -p "$BIN_DIR"
  if try_acquire_cargo_git "$want"; then
    :
  else
    die "failed to acquire specify $want into ./.bin (tried git tag, release asset, curl installer)"
  fi

  have="$(path_specify_version "$SPECIFY_LOCAL" || true)"
  [ -n "$have" ] || die "acquired specify at $SPECIFY_LOCAL is not runnable"
  if ! version_satisfies "$have" ge "$want"; then
    die "acquired specify reports '${have}', expected pin '$want' or newer"
  fi
  if [ "$have" != "$want" ]; then
    note "acquired specify $have for pin $want (release tag may ship a newer workspace version)"
  fi
}

try_acquire_cargo_git() {
  local want="$1"
  command -v cargo >/dev/null 2>&1 || return 1
  note "acquiring specify $want via cargo install --git v$want → ./.bin"
  if ! cargo install specify \
    --git https://github.com/augentic/specify-cli \
    --tag "v$want" \
    --root "$BIN_DIR" \
    --locked 2>/dev/null; then
    return 1
  fi
  stage_cargo_install
}

stage_cargo_install() {
  if [ -x "$BIN_DIR/bin/specify" ]; then
    cp -f "$BIN_DIR/bin/specify" "$SPECIFY_LOCAL"
  fi
  [ -x "$SPECIFY_LOCAL" ]
}

# Run the published curl installer, pinning the version into ./.bin.
curl_install() {
  local want="$1" url
  for url in \
    "https://specify.sh/install.sh" \
    "https://raw.githubusercontent.com/augentic/specify-cli/main/install.sh"; do
    if curl -sSfL "$url" | SPECIFY_INSTALL_DIR="$BIN_DIR" SPECIFY_VERSION="v$want" sh >&2; then
      [ -x "$SPECIFY_LOCAL" ] && return 0
    fi
  done
  return 1
}

# Prefer an already-installed PATH `specify` matching the pin, else acquire ./.bin.
resolve_published() {
  local want="$1" pv
  if command -v specify >/dev/null 2>&1; then
    pv="$(path_specify_version specify || true)"
    if version_satisfies "$pv" eq "$want"; then
      CMD=(specify)
      return 0
    fi
  fi
  acquire "$want"
  CMD=("$SPECIFY_LOCAL")
}

# Resolve SPECIFY_VERSION into the CMD[] command prefix.
resolve() {
  case "$SPECIFY_VERSION" in
    next)
      local manifest pin
      if manifest="$(find_manifest)"; then
        CMD=(cargo run --release --manifest-path "$manifest" --bin specify --)
      else
        pin="$(read_pin)"
        note "no specify-cli checkout found; falling back to published specify $pin in ./.bin"
        acquire "$pin"
        CMD=("$SPECIFY_LOCAL")
      fi
      ;;
    *)
      validate_semver "$SPECIFY_VERSION" \
        || die "unrecognised SPECIFY_VERSION '$SPECIFY_VERSION' (expected next|X.Y.Z)"
      resolve_published "$SPECIFY_VERSION"
      ;;
  esac
}

# Resolve SPECIFY_VERSION to a concrete binary PATH for the acceptance symlink.
# Same channels and fallbacks as `resolve` — a checkout is the preferred source
# under `next`, never a requirement; absent one we acquire the published binary
# into ./.bin just like `lint`. The only translation needed is for the
# `next` + checkout case, whose CMD is an ephemeral `cargo run` prefix: build
# once and emit the stable target path instead.
bin_path() {
  resolve
  case "${CMD[0]}" in
    cargo)
      local manifest
      manifest="$(find_manifest)" \
        || die "internal: cargo resolution without a specify-cli manifest"
      note "building specify from $manifest"
      cargo build --release --manifest-path "$manifest" --bin specify >&2
      local bin
      bin="$(cd "$(dirname "$manifest")" && pwd)/target/release/specify"
      [ -x "$bin" ] || die "expected built binary at $bin"
      printf '%s\n' "$bin"
      ;;
    specify)
      command -v specify
      ;;
    *)
      printf '%s\n' "${CMD[0]}"
      ;;
  esac
}

# ── Argument parsing ─────────────────────────────────────────

mode="run"
args=()
if [ "${1:-}" = "--mode" ]; then
  [ "${2:-}" = "bin-path" ] || die "unknown --mode '${2:-}' (expected bin-path)"
  mode="$2"
  shift 2
  [ "$#" -eq 0 ] || die "--mode $mode takes no further arguments"
elif [ "${1:-}" = "lint" ]; then
  shift
  args=(lint framework --framework-root . "$@")
else
  die "usage: specify.sh lint [args…] | --mode bin-path"
fi

# ── Resolve and dispatch ─────────────────────────────────────

CMD=()

# bin-path drives resolution itself (it may build) and emits a concrete path.
if [ "$mode" = "bin-path" ]; then
  bin_path
  exit 0
fi

resolve
cd "$REPO_ROOT"
exec "${CMD[@]}" "${args[@]}"
