---
id: CORE-021
title: Plugins Broken Symlink
severity: important
trigger: A symlink under plugins/ does not resolve.
rule_hints:
  - kind: authoring-predicate
    value: plugins.broken-symlink
    description: Run the retired imperative `plugins.broken-symlink` predicate via the RFC-31 bridge until native hint parity lands.
---

## Rule

This rule delegates to the closed imperative predicate `plugins.broken-symlink` through `kind: authoring-predicate`. Behaviour matches the former `framework::check` row; migrate to native deterministic hints when parity tests cover the fact-iterating form.

## Look For

Violations surfaced by `plugins.broken-symlink` on the framework tree.

## Fix

Resolve the violation described in the finding message for `plugins.broken-symlink`.
