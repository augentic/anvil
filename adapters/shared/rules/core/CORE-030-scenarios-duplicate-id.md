---
id: CORE-030
title: Scenarios Duplicate Id
severity: important
trigger: Duplicate scenario ids across files.
rule_hints:
  - kind: authoring-predicate
    value: scenarios.duplicate-id
    description: Run the retired imperative `scenarios.duplicate-id` predicate via the RFC-31 bridge until native hint parity lands.
---

## Rule

This rule delegates to the closed imperative predicate `scenarios.duplicate-id` through `kind: authoring-predicate`. Behaviour matches the former `framework::check` row; migrate to native deterministic hints when parity tests cover the fact-iterating form.

## Look For

Violations surfaced by `scenarios.duplicate-id` on the framework tree.

## Fix

Resolve the violation described in the finding message for `scenarios.duplicate-id`.
