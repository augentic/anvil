---
id: CORE-028
title: Scenarios Artifact Path Unsafe
severity: important
trigger: A scenario references an unsafe artifact path.
rule_hints:
  - kind: authoring-predicate
    value: scenarios.artifact-path-unsafe
    description: Run the retired imperative `scenarios.artifact-path-unsafe` predicate via the RFC-31 bridge until native hint parity lands.
---

## Rule

This rule delegates to the closed imperative predicate `scenarios.artifact-path-unsafe` through `kind: authoring-predicate`. Behaviour matches the former `framework::check` row; migrate to native deterministic hints when parity tests cover the fact-iterating form.

## Look For

Violations surfaced by `scenarios.artifact-path-unsafe` on the framework tree.

## Fix

Resolve the violation described in the finding message for `scenarios.artifact-path-unsafe`.
