---
id: CORE-018
title: Links Brief Schema Link Resolve
severity: important
trigger: An adapter brief references an unknown schemas.specify.dev tool schema URL.
rule_hints:
  - kind: authoring-predicate
    value: links.brief-schema-link-resolve
    description: Run the retired imperative `links.brief-schema-link-resolve` predicate via the RFC-31 bridge until native hint parity lands.
---

## Rule

This rule delegates to the closed imperative predicate `links.brief-schema-link-resolve` through `kind: authoring-predicate`. Behaviour matches the former `framework::check` row; migrate to native deterministic hints when parity tests cover the fact-iterating form.

## Look For

Violations surfaced by `links.brief-schema-link-resolve` on the framework tree.

## Fix

Resolve the violation described in the finding message for `links.brief-schema-link-resolve`.
