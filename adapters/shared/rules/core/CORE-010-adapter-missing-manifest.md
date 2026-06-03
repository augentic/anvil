---
id: CORE-010
title: Adapter Missing Manifest
severity: important
trigger: An adapter directory under adapters/sources or adapters/targets lacks adapter.yaml.
rule_hints:
  - kind: authoring-predicate
    value: adapter.missing-manifest
    description: Run the retired imperative `adapter.missing-manifest` predicate via the RFC-31 bridge until native hint parity lands.
---

## Rule

This rule delegates to the closed imperative predicate `adapter.missing-manifest` through `kind: authoring-predicate`. Behaviour matches the former `framework::check` row; migrate to native deterministic hints when parity tests cover the fact-iterating form.

## Look For

Violations surfaced by `adapter.missing-manifest` on the framework tree.

## Fix

Resolve the violation described in the finding message for `adapter.missing-manifest`.
