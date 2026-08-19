#!/usr/bin/env bash
# ADR-required-paths gate (CONSTITUTION.md invariant 5, remediation
# Phase 2): a pull request touching a gated path — or raising a ratchet
# ceiling — must carry a decision record under rfcs/decisions/ in the
# same diff. Run from the repository root with the PR base commit.
set -euo pipefail

base="${1:?usage: scripts/adr-required.sh <base-commit>}"
list="scripts/adr-required-paths.txt"
ratchet="scripts/ratchet.toml"

# One `<status>\t<path>` (or `R<n>\t<old>\t<new>`) line per change.
changed=$(git diff --name-status "$base"...HEAD)
files=$(printf '%s\n' "$changed" | awk -F'\t' 'NF { print $NF }')

gated=""
while IFS=$'\t' read -r status file renamed; do
  [ -z "$file" ] && continue
  # A rename gates on its destination path.
  [ -n "$renamed" ] && file="$renamed"
  # The ratchet baseline is answered by the raise detector below, so
  # shrinks and entry removals stay free of the plain prefix match.
  [ "$file" = "$ratchet" ] && continue
  while IFS= read -r entry; do
    case "$entry" in "" | "#"*) continue ;; esac
    marker=""
    prefix="$entry"
    case "$entry" in
      "new-ok "*)
        marker="new-ok"
        prefix="${entry#new-ok }"
        ;;
    esac
    case "$file" in
      "$prefix"*)
        if [ "$marker" = "new-ok" ] && [ "${status:0:1}" = "A" ]; then
          break
        fi
        gated="${gated}  ${file} (gated by ${prefix})"$'\n'
        break
        ;;
    esac
  done <"$list"
done <<<"$changed"

# Ratchet ceilings: a raised value or a new entry is policy; a shrink,
# a removed entry, or the file's introduction is free. Keys are
# section-qualified before comparing, and the base/head switch keys on
# FILENAME so an empty base cannot alias the head.
if printf '%s\n' "$files" | grep -qxF "$ratchet" && [ -f "$ratchet" ] \
  && git show "$base:$ratchet" >/dev/null 2>&1; then
  raises=$(awk -F'=' '
    /^\[/ { section = $0 }
    /^[[:space:]]*"/ {
      key = section $1
      gsub(/[" \t]/, "", key)
      value = $2 + 0
      if (FILENAME == ARGV[1]) { old[key] = value }
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

if printf '%s\n' "$files" | grep -q '^rfcs/decisions/'; then
  echo "ADR-gated paths touched; decision record present:"
  printf '%s' "$gated"
  exit 0
fi

echo "ADR-gated paths touched with no rfcs/decisions/ change in the diff"
echo "(CONSTITUTION.md invariant 5 — policy changes are decisions):"
printf '%s' "$gated"
exit 1
