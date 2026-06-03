---
id: CORE-049
title: Tools Invalid Declaration
severity: important
trigger: A WASI tool declaration in the framework tree is invalid.
rule_hints:
  - kind: authoring-predicate
    value: tools.invalid-declaration
    description: Run the retired imperative `tools.invalid-declaration` predicate via the RFC-31 bridge until native hint parity lands.
---

## Rule

This rule delegates to the closed imperative predicate `tools.invalid-declaration` through `kind: authoring-predicate`. Behaviour matches the former `framework::check` row; migrate to native deterministic hints when parity tests cover the fact-iterating form.

## Look For

Violations surfaced by `tools.invalid-declaration` on the framework tree.

## Fix

Resolve the violation described in the finding message for `tools.invalid-declaration`.
