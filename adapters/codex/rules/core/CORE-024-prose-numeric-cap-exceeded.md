---
id: CORE-024
title: Prose Numeric Cap Exceeded
severity: important
trigger: A documented skill numeric cap drifted from its canonical source.
rule_hints:
  - kind: path-pattern
    value: adapters/shared/prose/rules/core/CORE-024-prose-numeric-cap-exceeded.md
    description: Sentinel path so the whole-tree prose tool runs exactly once; the tool walks PROJECT_DIR itself rather than the passed candidate.
  - kind: tool
    value: prose
    config:
      description-cap: 512
      body-cap: 200
    description: Run the `prose` framework checker, which confirms the documented skill description and body caps stay in sync across the embedded skill schema and the standards document. The cap values are policy carried here, not in the tool.
---

## Rule

The skill description character cap and skill body line cap must stay in sync across the embedded skill schema and `docs/standards/skill-authoring.md`. Both cap *values* live in this rule's `config:` so they are framework-owned policy, not baked into the checker. The tool reads them from the forwarded config; the engine only relays them.

This check is whole-tree: the `prose` framework tool cross-checks the description cap against its embedded copy of the skill schema and reads the standards document under `PROJECT_DIR`. The rule's `path-pattern` names a single sentinel file so the tool runs exactly once per lint; the tool reads `PROJECT_DIR` and walks the tree itself.

## Look For

- The description cap missing from the embedded skill schema.
- The description or body cap missing from `docs/standards/skill-authoring.md`.
- The standards document missing entirely.

## Fix

Restore the cap value in the drifted source so the schema and standards document agree with the cap values declared in this rule's `config:`.
