---
id: CORE-019
title: Links Broken Reference
severity: important
trigger: A markdown link target does not resolve to an existing file.
rule_hints:
  - kind: authoring-predicate
    value: links.broken-reference
    description: Run the retired imperative `links.broken-reference` predicate via the RFC-31 bridge until native hint parity lands.
---

## Rule

This rule delegates to the closed imperative predicate `links.broken-reference` through `kind: authoring-predicate`. Behaviour matches the former `framework::check` row; migrate to native deterministic hints when parity tests cover the fact-iterating form.

## Look For

Violations surfaced by `links.broken-reference` on the framework tree.

## Fix

Resolve the violation described in the finding message for `links.broken-reference`.
