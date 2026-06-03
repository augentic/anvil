---
id: CORE-027
title: Rules Schema Violation
severity: critical
trigger: A rules markdown file fails rule.schema.json validation or lacks ## Rule.
rule_hints:
  - kind: authoring-predicate
    value: rules.schema-violation
    description: Run the retired imperative `rules.schema-violation` predicate via the RFC-31 bridge until native hint parity lands.
---

## Rule

This rule delegates to the closed imperative predicate `rules.schema-violation` through `kind: authoring-predicate`. Behaviour matches the former `framework::check` row; migrate to native deterministic hints when parity tests cover the fact-iterating form.

## Look For

Violations surfaced by `rules.schema-violation` on the framework tree.

## Fix

Resolve the violation described in the finding message for `rules.schema-violation`.
