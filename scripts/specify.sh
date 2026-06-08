#!/usr/bin/env bash
#
# Central resolver + runner that binds the framework repo to a `specify`
# binary per SPECIFY_VERSION. See docs/contributing/checks.md#binding-to-a-specify-binary.
#
# Usage:
#   scripts/specify.sh fcheck                 # → lint framework --framework-root .
#   scripts/specify.sh <specify-subcommand>…  # passthrough to the resolved binary
#   scripts/specify.sh --mode emit-cmd        # print resolved command prefix (debug)
#   scripts/specify.sh --mode verify-only     # exit 0 when a usable binary resolves
#
# Env:
#   SPECIFY_VERSION  next | latest | X.Y.Z | system   (default: next)
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
find_specify_cli_manifest() {
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

# Mirror the specify-cli release probe order: SPECIFY_RELEASE_TAG → gh → REST.
# Echoes the resolved semver with any leading `v` stripped.
resolve_latest_tag() {
  local tag=""
  if [ -n "${SPECIFY_RELEASE_TAG:-}" ]; then
    tag="$SPECIFY_RELEASE_TAG"
  elif command -v gh >/dev/null 2>&1; then
    tag="$(gh release view --repo augentic/specify-cli --json tagName --jq .tagName 2>/dev/null || true)"
  fi
  if [ -z "$tag" ]; then
    tag="$(curl -fsSL https://api.github.com/repos/augentic/specify-cli/releases/latest 2>/dev/null \
      | awk -F'"' '/"tag_name"/{print $4; exit}' || true)"
  fi
  [ -n "$tag" ] || return 1
  printf '%s\n' "${tag#v}"
}

# Install the requested semver into ./.bin (cargo when present, else curl
# installer). Idempotent: skips work when ./.bin/specify already satisfies the pin.
acquire() {
  local want="$1" have
  if [ -x "$SPECIFY_LOCAL" ]; then
    have="$(path_specify_version "$SPECIFY_LOCAL" || true)"
    if { version_satisfies "$have" eq "$want" || version_satisfies "$have" ge "$want"; } \
      && "$SPECIFY_LOCAL" lint framework --help >/dev/null 2>&1; then
      return 0
    fi
  fi

  mkdir -p "$BIN_DIR"
  if try_acquire_cargo_registry "$want" \
    || try_acquire_cargo_git "$want" \
    || try_acquire_release_asset "$want" \
    || try_acquire_curl "$want"; then
    :
  else
    die "failed to acquire specify $want into ./.bin (tried crates.io, git tag, release asset, curl installer)"
  fi

  have="$(path_specify_version "$SPECIFY_LOCAL" || true)"
  [ -n "$have" ] || die "acquired specify at $SPECIFY_LOCAL is not runnable"
  if ! version_satisfies "$have" eq "$want" && ! version_satisfies "$have" ge "$want"; then
    die "acquired specify reports '${have}', expected pin '$want' or newer"
  fi
  if [ "$have" != "$want" ]; then
    note "acquired specify $have for pin $want (release tag may ship a newer workspace version)"
  fi
  ensure_framework_lint_capable
}

# When the pinned release predates `lint framework`, fall back to main from git.
ensure_framework_lint_capable() {
  if "$SPECIFY_LOCAL" lint framework --help >/dev/null 2>&1; then
    return 0
  fi
  command -v cargo >/dev/null 2>&1 \
    || die "acquired specify lacks 'lint framework' and no Rust toolchain is available to install a newer build"
  note "pinned release lacks 'lint framework'; acquiring specify from main → ./.bin"
  cargo install specify \
    --git https://github.com/augentic/specify-cli \
    --root "$BIN_DIR" \
    --locked \
    --force
  stage_cargo_install || die "failed to install specify from main into ./.bin"
}

try_acquire_cargo_registry() {
  local want="$1"
  command -v cargo >/dev/null 2>&1 || return 1
  note "acquiring specify $want via cargo install → ./.bin"
  if ! cargo install specify --version "$want" --root "$BIN_DIR" --locked 2>/dev/null; then
    return 1
  fi
  stage_cargo_install
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

# When release assets exist, download the matching platform archive from GitHub.
try_acquire_release_asset() {
  local want="$1" target archive url tmpdir
  case "$(uname -s)-$(uname -m)" in
    Linux-x86_64) target=x86_64-unknown-linux-gnu ;;
    Linux-aarch64 | Linux-arm64) target=aarch64-unknown-linux-gnu ;;
    Darwin-x86_64) target=x86_64-apple-darwin ;;
    Darwin-arm64) target=aarch64-apple-darwin ;;
    *) return 1 ;;
  esac
  archive="specify-v${want}-${target}.tar.gz"
  url="https://github.com/augentic/specify-cli/releases/download/v${want}/${archive}"
  tmpdir="$(mktemp -d)"
  note "acquiring specify $want via GitHub release asset → ./.bin"
  if ! curl -fsSL "$url" | tar -xz -C "$tmpdir" 2>/dev/null; then
    rm -rf "$tmpdir"
    return 1
  fi
  if [ -x "$tmpdir/specify" ]; then
    install -m 755 "$tmpdir/specify" "$SPECIFY_LOCAL"
    rm -rf "$tmpdir"
    return 0
  fi
  rm -rf "$tmpdir"
  return 1
}

try_acquire_curl() {
  local want="$1"
  note "acquiring specify $want via curl installer → ./.bin"
  curl_install "$want"
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

# Prefer an already-installed PATH `specify` that satisfies, else acquire ./.bin.
# resolve_published <op> <want>   op ∈ eq | ge
resolve_published() {
  local op="$1" want="$2" pv
  if command -v specify >/dev/null 2>&1; then
    pv="$(path_specify_version specify || true)"
    if version_satisfies "$pv" "$op" "$want" \
      && specify lint framework --help >/dev/null 2>&1; then
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
    system)
      command -v specify >/dev/null 2>&1 \
        || die "SPECIFY_VERSION=system but 'specify' is not on PATH"
      CMD=(specify)
      ;;
    next)
      local manifest pin
      if manifest="$(find_specify_cli_manifest)"; then
        CMD=(cargo run --release --manifest-path "$manifest" --bin specify --)
      else
        pin="$(read_pin)"
        note "no specify-cli checkout found; falling back to published specify $pin in ./.bin"
        acquire "$pin"
        CMD=("$SPECIFY_LOCAL")
      fi
      ;;
    latest)
      local tag
      tag="$(resolve_latest_tag)" || die "could not resolve latest specify release tag"
      resolve_published ge "$tag"
      ;;
    *)
      validate_semver "$SPECIFY_VERSION" \
        || die "unrecognised SPECIFY_VERSION '$SPECIFY_VERSION' (expected next|latest|X.Y.Z|system)"
      resolve_published eq "$SPECIFY_VERSION"
      ;;
  esac
}

# ── Argument parsing ─────────────────────────────────────────

mode="run"
args=()
if [ "${1:-}" = "--mode" ]; then
  case "${2:-}" in
    emit-cmd | verify-only) mode="$2" ;;
    *) die "unknown --mode '${2:-}' (expected emit-cmd|verify-only)" ;;
  esac
  shift 2
  [ "$#" -eq 0 ] || die "--mode $mode takes no further arguments"
elif [ "${1:-}" = "fcheck" ]; then
  shift
  args=(lint framework --framework-root . "$@")
else
  args=("$@")
fi

# ── Resolve and dispatch ─────────────────────────────────────

CMD=()
resolve

case "$mode" in
  emit-cmd)
    printf '%s\n' "${CMD[*]}"
    exit 0
    ;;
  verify-only)
    exit 0
    ;;
esac

cd "$REPO_ROOT"
exec "${CMD[@]}" "${args[@]}"
