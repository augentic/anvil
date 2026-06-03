---
id: CORE-020
title: Links Unresolved Directive
severity: important
trigger: A skill directive references a path that does not resolve.
rule_hints:
  - kind: authoring-predicate
    value: links.unresolved-directive
    description: Run the retired imperative `links.unresolved-directive` predicate via the RFC-31 bridge until native hint parity lands.
---

## Rule

This rule delegates to the closed imperative predicate `links.unresolved-directive` through `kind: authoring-predicate`. Behaviour matches the former `framework::check` row; migrate to native deterministic hints when parity tests cover the fact-iterating form.

## Look For

Violations surfaced by `links.unresolved-directive` on the framework tree.

## Fix

Resolve the violation described in the finding message for `links.unresolved-directive`.
