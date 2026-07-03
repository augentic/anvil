#!/usr/bin/env sh
# Compare SHA-256 digests of spec-runtime bundle files between specify and
# specify-adapters. Skips gracefully when the sibling checkout is absent.
set -eu

SPECIFY_ROOT="${SPECIFY_ROOT:-.}"
SPECIFY_ADAPTERS_ROOT="${SPECIFY_ADAPTERS_ROOT:-../specify-adapters}"

if [ ! -d "$SPECIFY_ADAPTERS_ROOT/shared/references/runtime" ]; then
  echo "check-adapters-spec-runtime-parity: skip (no sibling at $SPECIFY_ADAPTERS_ROOT)"
  exit 0
fi

README="$SPECIFY_ROOT/adapters/shared/references/runtime/README.md"
if [ ! -f "$README" ]; then
  echo "check-adapters-spec-runtime-parity: missing $README" >&2
  exit 1
fi

pairs=$(awk -F'|' '
  $2 ~ /^ `[^`]+` $/ && $3 ~ /^ `plugins\/spec\/references\// {
    gsub(/^ `/, "", $2); gsub(/` $/, "", $2)
    gsub(/^ `/, "", $3); gsub(/` $/, "", $3)
    print $2 "\t" $3
  }
' "$README")

fail=0
printf '%s\n' "$pairs" | while IFS="$(printf '\t')" read -r bundle canonical; do
  [ -n "$bundle" ] || continue

  left="$SPECIFY_ROOT/$canonical"
  right="$SPECIFY_ADAPTERS_ROOT/shared/references/runtime/$bundle"

  if [ ! -f "$left" ]; then
    echo "missing canonical file: $left" >&2
    exit 1
  fi
  if [ ! -f "$right" ]; then
    echo "missing forked file: $right" >&2
    exit 1
  fi

  left_hash=$(shasum -a 256 "$left" | awk '{print $1}')
  right_hash=$(shasum -a 256 "$right" | awk '{print $1}')
  if [ "$left_hash" != "$right_hash" ]; then
    echo "spec-runtime drift: $bundle (specify $canonical)" >&2
    exit 1
  fi
done || fail=1

if [ "$fail" -ne 0 ]; then
  echo "Sync specify-adapters shared/references/runtime from specify plugins/spec/references/." >&2
  exit 1
fi

echo "check-adapters-spec-runtime-parity: ok"
