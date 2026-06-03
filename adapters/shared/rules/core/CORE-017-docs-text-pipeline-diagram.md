---
id: CORE-017
title: Docs Text Pipeline Diagram
severity: important
trigger: Documentation uses a text pipeline diagram where an asset is required.
rule_hints:
  - kind: authoring-predicate
    value: docs.text-pipeline-diagram
    description: Run the retired imperative `docs.text-pipeline-diagram` predicate via the RFC-31 bridge until native hint parity lands.
---

## Rule

This rule delegates to the closed imperative predicate `docs.text-pipeline-diagram` through `kind: authoring-predicate`. Behaviour matches the former `framework::check` row; migrate to native deterministic hints when parity tests cover the fact-iterating form.

## Look For

Violations surfaced by `docs.text-pipeline-diagram` on the framework tree.

## Fix

Resolve the violation described in the finding message for `docs.text-pipeline-diagram`.
