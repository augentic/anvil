---
id: CORE-034
title: Scenarios Stale Recorded Trace
severity: important
trigger: Recorded trace is stale relative to repository HEAD.
rule_hints:
  - kind: authoring-predicate
    value: scenarios.stale-recorded-trace
    description: Run the retired imperative `scenarios.stale-recorded-trace` predicate via the RFC-31 bridge until native hint parity lands.
---

## Rule

This rule delegates to the closed imperative predicate `scenarios.stale-recorded-trace` through `kind: authoring-predicate`. Behaviour matches the former `framework::check` row; migrate to native deterministic hints when parity tests cover the fact-iterating form.

## Look For

Violations surfaced by `scenarios.stale-recorded-trace` on the framework tree.

## Fix

Resolve the violation described in the finding message for `scenarios.stale-recorded-trace`.
