#!/usr/bin/env bash
#
# Central resolver + runner that binds the framework repo to a `specify`
# binary per SPECIFY_VERSION / Specify.toml. See docs/contributing/checks.md#binding-to-a-specify-binary.
#
# Usage:
#   scripts/specify.sh lint                   # → lint framework
#   scripts/specify.sh --mode bin-path        # print materialized binary path
#   scripts/specify.sh --mode config-key KEY  # print a [cli] value from Specify.toml
#
# Env:
#   SPECIFY_VERSION  next | latest | X.Y.Z   (Make default: next; overrides Specify.toml)
#   REPO_ROOT        optional; defaults to git root / script parent

set -euo pipefail

# ── Repo root ────────────────────────────────────────────────

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
if [ -z "${REPO_ROOT:-}" ]; then
  REPO_ROOT="$(git -C "$script_dir" rev-parse --show-toplevel 2>/dev/null || true)"
  [ -n "$REPO_ROOT" ] || REPO_ROOT="$(cd "$script_dir/.." && pwd)"
fi

SPECIFY_FILE="$REPO_ROOT/Specify.toml"

# ── Helpers ──────────────────────────────────────────────────

note() { printf 'specify.sh: %s\n' "$1" >&2; }
die() { printf 'specify.sh: error: %s\n' "$1" >&2; exit 1; }

# Read a flat key from [cli] in Specify.toml (no nested tables).
read_cli_key() {
  local key="$1" default="${2:-}"
  local val
  if [ ! -f "$SPECIFY_FILE" ]; then
    [ -n "$default" ] && printf '%s\n' "$default"
    return 0
  fi
  val="$(awk -v key="$key" '
    BEGIN { in_cli = 0 }
    /^\[cli\][[:space:]]*$/ { in_cli = 1; next }
    /^\[/ { in_cli = 0 }
    in_cli && $1 == key {
      line = $0
      sub(/^[^=]+=[[:space:]]*/, "", line)
      sub(/[[:space:]]+#.*$/, "", line)
      gsub(/^[[:space:]]+|[[:space:]]+$/, "", line)
      gsub(/^["'\''"]|["'\''"]$/, "", line)
      print line
      exit
    }
  ' "$SPECIFY_FILE")"
  if [ -n "$val" ]; then
    printf '%s\n' "$val"
  elif [ -n "$default" ]; then
    printf '%s\n' "$default"
  fi
}

expand_tilde() {
  local path="$1"
  if [[ "$path" == "~/"* ]]; then
    printf '%s/%s\n' "$HOME" "${path:2}"
  elif [[ "$path" == "~" ]]; then
    printf '%s\n' "$HOME"
  else
    printf '%s\n' "$path"
  fi
}

cli_binary_rel() {
  read_cli_key binary ".bin/specify"
}

cli_binary_abs() {
  printf '%s\n' "$REPO_ROOT/$(cli_binary_rel)"
}

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

resolved_version() {
  if [ -n "${SPECIFY_VERSION:-}" ]; then
    printf '%s\n' "$SPECIFY_VERSION"
    return 0
  fi
  local from_file
  from_file="$(read_cli_key version "")"
  if [ -n "$from_file" ]; then
    printf '%s\n' "$from_file"
  else
    printf '%s\n' "next"
  fi
}

file_fallback_version() {
  local pin
  pin="$(read_cli_key version "")"
  if validate_semver "$pin"; then
    printf '%s\n' "$pin"
    return 0
  fi
  return 1
}

probe_latest_release() {
  local tag
  tag="$(curl -fsSL "https://api.github.com/repos/augentic/specify-cli/releases/latest" \
    | awk -F'"' '/"tag_name"/ { gsub(/^v/, "", $4); print $4; exit }')" \
    || die "failed to probe latest specify-cli release from GitHub"
  validate_semver "$tag" || die "latest release tag is not semver: $tag"
  printf '%s\n' "$tag"
}

# Install the requested semver into cli.binary via `cargo install --git`.
acquire_published() {
  local want="$1" dest bin_root have
  dest="$(cli_binary_abs)"
  bin_root="$(dirname "$dest")"

  if [ -x "$dest" ]; then
    have="$(path_specify_version "$dest" || true)"
    if [ "$want" = "latest" ]; then
      version_satisfies "$have" ge "$(probe_latest_release)" && return 0
    else
      version_satisfies "$have" ge "$want" && return 0
    fi
  fi

  if [ "$want" = "latest" ]; then
    want="$(probe_latest_release)"
  fi

  if command -v specify >/dev/null 2>&1; then
    have="$(path_specify_version specify || true)"
    if version_satisfies "$have" eq "$want"; then
      mkdir -p "$bin_root"
      cp -f "$(command -v specify)" "$dest"
      return 0
    fi
  fi

  command -v cargo >/dev/null 2>&1 \
    || die "cargo not found; install Rust to bootstrap specify $want into $(cli_binary_rel), or put a matching specify on PATH"

  mkdir -p "$bin_root"
  note "acquiring specify $want via cargo install --git --tag v$want → $(cli_binary_rel)"
  cargo install specify \
    --git https://github.com/augentic/specify-cli \
    --tag "v$want" \
    --root "$bin_root" \
    --locked >&2 \
    || die "cargo install --git failed for specify $want"

  if [ -x "$bin_root/bin/specify" ] && [ "$dest" != "$bin_root/bin/specify" ]; then
    cp -f "$bin_root/bin/specify" "$dest"
  fi

  have="$(path_specify_version "$dest" || true)"
  [ -n "$have" ] || die "acquired specify at $dest is not runnable"
  version_satisfies "$have" ge "$want" \
    || die "acquired specify reports '${have}', expected pin '$want' or newer"
  [ "$have" = "$want" ] \
    || note "acquired specify $have for pin $want (release tag may ship a newer workspace version)"
}

materialize_from_checkout() {
  local manifest built dest
  manifest="$(find_manifest)" || return 1
  note "building specify from $manifest"
  cargo build --release --manifest-path "$manifest" --bin specify >&2
  built="$(cd "$(dirname "$manifest")" && pwd)/target/release/specify"
  [ -x "$built" ] || die "expected built binary at $built"
  dest="$(cli_binary_abs)"
  mkdir -p "$(dirname "$dest")"
  cp -f "$built" "$dest"
}

materialize_binary() {
  local channel dest have
  channel="$(resolved_version)"
  dest="$(cli_binary_abs)"

  case "$channel" in
    next)
      if materialize_from_checkout; then
        return 0
      fi
      if file_fallback_version; then
        note "no specify-cli checkout found; falling back to published specify $(file_fallback_version) in $(cli_binary_rel)"
        acquire_published "$(file_fallback_version)"
      else
        note "no specify-cli checkout found; falling back to latest published specify in $(cli_binary_rel)"
        acquire_published "latest"
      fi
      ;;
    latest)
      acquire_published "latest"
      ;;
    *)
      validate_semver "$channel" \
        || die "unrecognised version channel '$channel' (expected next|latest|X.Y.Z)"
      acquire_published "$channel"
      ;;
  esac

  [ -x "$dest" ] || die "materialized binary is not executable at $dest"
  have="$(path_specify_version "$dest" || true)"
  [ -n "$have" ] || die "materialized binary at $dest is not runnable"
}

# ── Argument parsing ─────────────────────────────────────────

mode="run"
config_key=""
args=()
if [ "${1:-}" = "--mode" ]; then
  mode="${2:-}"
  shift 2
  case "$mode" in
    bin-path)
      [ "$#" -eq 0 ] || die "--mode bin-path takes no further arguments"
      ;;
    config-key)
      config_key="${1:-}"
      [ -n "$config_key" ] || die "--mode config-key requires a key name"
      shift
      [ "$#" -eq 0 ] || die "--mode config-key takes exactly one argument"
      ;;
    *)
      die "unknown --mode '$mode' (expected bin-path|config-key)"
      ;;
  esac
elif [ "${1:-}" = "lint" ]; then
  shift
  args=(lint framework "$@")
else
  die "usage: specify.sh lint [args…] | --mode bin-path | --mode config-key <key>"
fi

# ── Resolve and dispatch ─────────────────────────────────────

if [ "$mode" = "config-key" ]; then
  case "$config_key" in
    version) resolved_version ;;
    binary) cli_binary_rel ;;
    path)
      val="$(read_cli_key path "")"
      [ -n "$val" ] || die "Specify.toml has no cli.path"
      expand_tilde "$val"
      ;;
    *) die "unknown config key '$config_key' (expected version|binary|path)" ;;
  esac
  exit 0
fi

if [ "$mode" = "bin-path" ]; then
  materialize_binary
  cli_binary_abs
  exit 0
fi

materialize_binary
cd "$REPO_ROOT"
exec "$(cli_binary_abs)" "${args[@]}"
