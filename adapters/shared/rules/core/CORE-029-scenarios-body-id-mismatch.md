---
id: CORE-029
title: Scenarios Body Id Mismatch
severity: important
trigger: Scenario body id disagrees with frontmatter id.
rule_hints:
  - kind: authoring-predicate
    value: scenarios.body-id-mismatch
    description: Run the retired imperative `scenarios.body-id-mismatch` predicate via the RFC-31 bridge until native hint parity lands.
---

## Rule

This rule delegates to the closed imperative predicate `scenarios.body-id-mismatch` through `kind: authoring-predicate`. Behaviour matches the former `framework::check` row; migrate to native deterministic hints when parity tests cover the fact-iterating form.

## Look For

Violations surfaced by `scenarios.body-id-mismatch` on the framework tree.

## Fix

Resolve the violation described in the finding message for `scenarios.body-id-mismatch`.
