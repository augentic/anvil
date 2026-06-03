---
id: CORE-031
title: Scenarios Recorded Trace Violation
severity: important
trigger: Recorded trace content violates scenario contract.
rule_hints:
  - kind: authoring-predicate
    value: scenarios.recorded-trace-violation
    description: Run the retired imperative `scenarios.recorded-trace-violation` predicate via the RFC-31 bridge until native hint parity lands.
---

## Rule

This rule delegates to the closed imperative predicate `scenarios.recorded-trace-violation` through `kind: authoring-predicate`. Behaviour matches the former `framework::check` row; migrate to native deterministic hints when parity tests cover the fact-iterating form.

## Look For

Violations surfaced by `scenarios.recorded-trace-violation` on the framework tree.

## Fix

Resolve the violation described in the finding message for `scenarios.recorded-trace-violation`.
