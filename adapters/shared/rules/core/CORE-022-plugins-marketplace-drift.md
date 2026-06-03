---
id: CORE-022
title: Plugins Marketplace Drift
severity: important
trigger: marketplace.json drifts from on-disk plugin layout.
rule_hints:
  - kind: authoring-predicate
    value: plugins.marketplace-drift
    description: Run the retired imperative `plugins.marketplace-drift` predicate via the RFC-31 bridge until native hint parity lands.
---

## Rule

This rule delegates to the closed imperative predicate `plugins.marketplace-drift` through `kind: authoring-predicate`. Behaviour matches the former `framework::check` row; migrate to native deterministic hints when parity tests cover the fact-iterating form.

## Look For

Violations surfaced by `plugins.marketplace-drift` on the framework tree.

## Fix

Resolve the violation described in the finding message for `plugins.marketplace-drift`.
