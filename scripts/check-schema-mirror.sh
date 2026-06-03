#!/usr/bin/env bash
set -euo pipefail

# Authoritative mirror map (single source of truth).
#
# The editor-facing JSON Schema mirrors under .cursor/schemas/ must stay
# byte-identical to their authoritative sources in the sibling specify-cli repo.
# Each pair below is "<.cursor/schemas/ file>=<CLI source, relative to CLI root>".
#
#   adapter.schema.json     = schemas/adapter.schema.json
#   marketplace.schema.json = schemas/authoring/marketplace.schema.json
#   rule.schema.json        = schemas/rules/rule.schema.json
#   scenario.schema.json    = schemas/authoring/scenario.schema.json
#   skill.schema.json       = schemas/authoring/skill.schema.json
#
# To update a mirror after the CLI source changes, copy the CLI source over the
# .cursor/schemas/ file (do not hand-edit the mirror).

# Resolve the CLI repo root the same way the Makefile resolves SPECIFY_MANIFEST:
# prefer ./specify-cli (the CI layout), else ../specify-cli (the local sibling layout).
if [ -f "specify-cli/Cargo.toml" ]; then
  CLI_ROOT="specify-cli"
elif [ -f "../specify-cli/Cargo.toml" ]; then
  CLI_ROOT="../specify-cli"
else
  echo "error: cannot locate specify-cli (looked for specify-cli/Cargo.toml and ../specify-cli/Cargo.toml)" >&2
  exit 1
fi

pairs=(
  "adapter.schema.json=schemas/adapter.schema.json"
  "marketplace.schema.json=schemas/authoring/marketplace.schema.json"
  "rule.schema.json=schemas/rules/rule.schema.json"
  "scenario.schema.json=schemas/authoring/scenario.schema.json"
  "skill.schema.json=schemas/authoring/skill.schema.json"
)

drift=0
for pair in "${pairs[@]}"; do
  mirror=".cursor/schemas/${pair%%=*}"
  source="${CLI_ROOT}/${pair#*=}"
  if [ ! -f "$source" ]; then
    echo "error: CLI source not found: $source" >&2
    drift=1
    continue
  fi
  if ! diff -q "$mirror" "$source" >/dev/null; then
    echo "DRIFT: $mirror differs from $source" >&2
    echo "  fix: cp \"$source\" \"$mirror\"" >&2
    drift=1
  fi
done

if [ "$drift" -ne 0 ]; then
  echo "schema mirror check failed: .cursor/schemas/ is out of sync with specify-cli" >&2
  exit 1
fi

echo "OK: all ${#pairs[@]} .cursor/schemas/ mirrors match specify-cli sources"
