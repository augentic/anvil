#!/usr/bin/env bash
# ADR-required-paths gate (CONSTITUTION.md invariant 5, remediation
# Phase 2): a pull request touching a gated path — or raising a ratchet
# ceiling — must carry a decision record under rfcs/decisions/ in the
# same diff. Run from the repository root with the PR base commit.
set -euo pipefail

base="${1:?usage: scripts/adr-required.sh <base-commit>}"
list="scripts/adr-required-paths.txt"
ratchet="scripts/ratchet.toml"

changed=$(git diff --name-only "$base"...HEAD)

gated=""
while IFS= read -r file; do
  [ -z "$file" ] && continue
  while IFS= read -r prefix; do
    case "$prefix" in "" | "#"*) continue ;; esac
    case "$file" in
      "$prefix"*)
        gated="${gated}  ${file} (gated by ${prefix})"$'\n'
        break
        ;;
    esac
  done <"$list"
done <<<"$changed"

# Ratchet ceilings: a raised value or a new entry is policy; a shrink,
# a removed entry, or the file's introduction is free. Keys are
# section-qualified before comparing.
if grep -qx "$ratchet" <<<"$changed" && [ -f "$ratchet" ] \
  && git show "$base:$ratchet" >/dev/null 2>&1; then
  raises=$(awk -F'=' '
    FNR == 1 { file++ }
    /^\[/ { section = $0 }
    /^[[:space:]]*"/ {
      key = section $1
      gsub(/[" \t]/, "", key)
      value = $2 + 0
      if (file == 1) { old[key] = value }
      else if (!(key in old) || value > old[key]) { print "  " key }
    }
  ' <(git show "$base:$ratchet") "$ratchet")
  if [ -n "$raises" ]; then
    gated="${gated}${ratchet} ceilings raised or added:"$'\n'"${raises}"$'\n'
  fi
fi

if [ -z "$gated" ]; then
  echo "No ADR-gated paths touched."
  exit 0
fi

if grep -q '^rfcs/decisions/' <<<"$changed"; then
  echo "ADR-gated paths touched; decision record present:"
  printf '%s' "$gated"
  exit 0
fi

echo "ADR-gated paths touched with no rfcs/decisions/ change in the diff"
echo "(CONSTITUTION.md invariant 5 — policy changes are decisions):"
printf '%s' "$gated"
exit 1
