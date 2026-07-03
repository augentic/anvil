---
id: CORE-063
title: Guardrails Consumer Boundary Missing
severity: important
trigger: The forked spec-runtime guardrails bundle is missing the Consumer tooling boundary section agents rely on during Vectis build/merge.
rule_hints:
  - kind: path-pattern
    value: shared/references/runtime/guardrails.md
  - kind: regex
    value: '^## Consumer tooling boundary$'
    description: Flag when the forked guardrails bundle lacks the required H2 section.
---

## Rule

The specify-adapters repository carries a forked copy of the spec-runtime guardrails bundle. Vectis build and merge briefs link `../references/spec-runtime/guardrails.md#consumer-tooling-boundary` — when that section is absent, agents read stale rules and may patch upstream templates in-band.

Specify-adapters CI enforces this via `check-guardrails-consumer-boundary` in `Makefile.toml`. When `specify lint framework` is available against this tree, the same `regex` hint fires there too.

## Look For

- `shared/references/runtime/guardrails.md` missing `## Consumer tooling boundary`.

## Fix

Sync `shared/references/runtime/guardrails.md` from `plugins/spec/references/guardrails.md` in the specify repository (at minimum the `## Consumer tooling boundary` section).
