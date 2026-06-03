---
id: CORE-032
title: Scenarios Schema Violation
severity: important
trigger: Scenario frontmatter fails scenario.schema.json.
rule_hints:
  - kind: authoring-predicate
    value: scenarios.schema-violation
    description: Run the retired imperative `scenarios.schema-violation` predicate via the RFC-31 bridge until native hint parity lands.
---

## Rule

This rule delegates to the closed imperative predicate `scenarios.schema-violation` through `kind: authoring-predicate`. Behaviour matches the former `framework::check` row; migrate to native deterministic hints when parity tests cover the fact-iterating form.

## Look For

Violations surfaced by `scenarios.schema-violation` on the framework tree.

## Fix

Resolve the violation described in the finding message for `scenarios.schema-violation`.
