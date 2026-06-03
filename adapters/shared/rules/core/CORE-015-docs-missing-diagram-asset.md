---
id: CORE-015
title: Docs Missing Diagram Asset
severity: important
trigger: A documentation diagram references a missing asset file.
rule_hints:
  - kind: authoring-predicate
    value: docs.missing-diagram-asset
    description: Run the retired imperative `docs.missing-diagram-asset` predicate via the RFC-31 bridge until native hint parity lands.
---

## Rule

This rule delegates to the closed imperative predicate `docs.missing-diagram-asset` through `kind: authoring-predicate`. Behaviour matches the former `framework::check` row; migrate to native deterministic hints when parity tests cover the fact-iterating form.

## Look For

Violations surfaced by `docs.missing-diagram-asset` on the framework tree.

## Fix

Resolve the violation described in the finding message for `docs.missing-diagram-asset`.
