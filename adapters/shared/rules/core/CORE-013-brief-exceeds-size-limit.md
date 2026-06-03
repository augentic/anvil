---
id: CORE-013
title: Brief Exceeds Size Limit
severity: important
trigger: An adapter brief body exceeds the configured line budget.
rule_hints:
  - kind: authoring-predicate
    value: brief.exceeds-size-limit
    description: Run the retired imperative `brief.exceeds-size-limit` predicate via the RFC-31 bridge until native hint parity lands.
---

## Rule

This rule delegates to the closed imperative predicate `brief.exceeds-size-limit` through `kind: authoring-predicate`. Behaviour matches the former `framework::check` row; migrate to native deterministic hints when parity tests cover the fact-iterating form.

## Look For

Violations surfaced by `brief.exceeds-size-limit` on the framework tree.

## Fix

Resolve the violation described in the finding message for `brief.exceeds-size-limit`.
