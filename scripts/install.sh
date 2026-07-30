#!/bin/sh
# Install the emery CLI from GitHub Release archives.
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/augentic/emery/main/scripts/install.sh | sh
#   curl -fsSL .../install.sh | sh -s -- --version 0.32.0
#
# Flags:
#   --version <semver>   exact release version (default: latest GitHub Release)
#   --target <triple>    override platform detection
#   --dir <path>         install destination (default: $EMERY_INSTALL_DIR or ~/.local/bin)
#   --dry-run            print the resolved URL and destination, then exit
#   -y                   reserved for parity with piped installs; no prompts exist today

set -eu

REPO="augentic/emery"
BIN="emery"

version=""
target=""
dir="${EMERY_INSTALL_DIR:-$HOME/.local/bin}"
dry_run=0

say() { printf '%s\n' "$*"; }
die() { printf 'install.sh: %s\n' "$*" >&2; exit 1; }

while [ $# -gt 0 ]; do
  case "$1" in
    --version) [ $# -ge 2 ] || die "--version needs a value"; version="$2"; shift 2 ;;
    --target)  [ $# -ge 2 ] || die "--target needs a value"; target="$2"; shift 2 ;;
    --dir)     [ $# -ge 2 ] || die "--dir needs a value"; dir="$2"; shift 2 ;;
    --dry-run) dry_run=1; shift ;;
    -y|--yes)  shift ;;
    -h|--help)
      sed -n '2,14p' "$0" 2>/dev/null || true
      exit 0
      ;;
    *) die "unknown flag: $1" ;;
  esac
done

command -v curl >/dev/null 2>&1 || die "curl is required"
command -v tar >/dev/null 2>&1 || die "tar is required"

if [ -z "$target" ]; then
  os="$(uname -s)"
  arch="$(uname -m)"
  case "$os $arch" in
    "Linux x86_64") target="x86_64-unknown-linux-gnu" ;;
    "Darwin arm64") target="aarch64-apple-darwin" ;;
    *) die "unsupported platform: $os $arch — no prebuilt archive; see https://github.com/$REPO#quick-start for source install" ;;
  esac
fi

if [ -z "$version" ]; then
  tag="$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" \
    | sed -n 's/^[[:space:]]*"tag_name":[[:space:]]*"\(.*\)",\{0,1\}$/\1/p' \
    | head -n 1)"
  [ -n "$tag" ] || die "could not resolve the latest release tag; pass --version"
  version="${tag#v}"
fi

archive="$BIN-v$version-$target.tar.gz"
url="https://github.com/$REPO/releases/download/v$version/$archive"

if [ "$dry_run" -eq 1 ]; then
  say "url:  $url"
  say "dest: $dir/$BIN"
  exit 0
fi

if command -v sha256sum >/dev/null 2>&1; then
  sha_cmd="sha256sum"
elif command -v shasum >/dev/null 2>&1; then
  sha_cmd="shasum -a 256"
else
  die "sha256sum or shasum is required"
fi

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT INT TERM

say "downloading $url"
curl -fsSL "$url" -o "$tmp/$archive" \
  || die "download failed — check that v$version exists and ships $target"
curl -fsSL "$url.sha256" -o "$tmp/$archive.sha256" \
  || die "checksum download failed for $archive.sha256"

expected="$(awk '{print $1}' "$tmp/$archive.sha256")"
actual="$(cd "$tmp" && $sha_cmd "$archive" | awk '{print $1}')"
[ "$expected" = "$actual" ] || die "sha256 mismatch for $archive (expected $expected, got $actual)"

tar -xzf "$tmp/$archive" -C "$tmp" "$BIN"
chmod +x "$tmp/$BIN"

mkdir -p "$dir"
mv -f "$tmp/$BIN" "$dir/$BIN"

say "installed $dir/$BIN ($("$dir/$BIN" --version 2>/dev/null || say "emery v$version"))"
case ":$PATH:" in
  *":$dir:"*) ;;
  *)
    say "note: $dir is not on your PATH — add it to your shell profile:"
    say "  export PATH=\"$dir:\$PATH\""
    ;;
esac
