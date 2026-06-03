---
id: CORE-052
title: Links Docs In Deployable Surface
severity: important
trigger: Documentation-only paths appear in a deployable adapter surface.
rule_hints:
  - kind: authoring-predicate
    value: links.docs-in-deployable-surface
    description: Run the retired imperative `links.docs-in-deployable-surface` predicate via the RFC-31 bridge until native hint parity lands.
---

## Rule

This rule delegates to the closed imperative predicate `links.docs-in-deployable-surface` through `kind: authoring-predicate`. Behaviour matches the former `framework::check` row; migrate to native deterministic hints when parity tests cover the fact-iterating form.

## Look For

Violations surfaced by `links.docs-in-deployable-surface` on the framework tree.

## Fix

Resolve the violation described in the finding message for `links.docs-in-deployable-surface`.
