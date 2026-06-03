---
id: CORE-033
title: Scenarios Stages Not Contiguous
severity: important
trigger: Scenario stages are not a contiguous prefix.
rule_hints:
  - kind: authoring-predicate
    value: scenarios.stages-not-contiguous-prefix
    description: Run the retired imperative `scenarios.stages-not-contiguous-prefix` predicate via the RFC-31 bridge until native hint parity lands.
---

## Rule

This rule delegates to the closed imperative predicate `scenarios.stages-not-contiguous-prefix` through `kind: authoring-predicate`. Behaviour matches the former `framework::check` row; migrate to native deterministic hints when parity tests cover the fact-iterating form.

## Look For

Violations surfaced by `scenarios.stages-not-contiguous-prefix` on the framework tree.

## Fix

Resolve the violation described in the finding message for `scenarios.stages-not-contiguous-prefix`.
