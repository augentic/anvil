---
id: CORE-044
title: Skill Schema Violation
severity: important
trigger: SKILL.md frontmatter fails skill.schema.json.
rule_hints:
  - kind: authoring-predicate
    value: skill.schema-violation
    description: Run the retired imperative `skill.schema-violation` predicate via the RFC-31 bridge until native hint parity lands.
---

## Rule

This rule delegates to the closed imperative predicate `skill.schema-violation` through `kind: authoring-predicate`. Behaviour matches the former `framework::check` row; migrate to native deterministic hints when parity tests cover the fact-iterating form.

## Look For

Violations surfaced by `skill.schema-violation` on the framework tree.

## Fix

Resolve the violation described in the finding message for `skill.schema-violation`.
