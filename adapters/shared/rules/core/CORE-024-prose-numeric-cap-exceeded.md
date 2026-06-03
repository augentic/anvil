---
id: CORE-024
title: Prose Numeric Cap Exceeded
severity: important
trigger: Prose exceeds a configured numeric cap in a skill section.
rule_hints:
  - kind: authoring-predicate
    value: prose.numeric-cap-exceeded
    description: Run the retired imperative `prose.numeric-cap-exceeded` predicate via the RFC-31 bridge until native hint parity lands.
---

## Rule

This rule delegates to the closed imperative predicate `prose.numeric-cap-exceeded` through `kind: authoring-predicate`. Behaviour matches the former `framework::check` row; migrate to native deterministic hints when parity tests cover the fact-iterating form.

## Look For

Violations surfaced by `prose.numeric-cap-exceeded` on the framework tree.

## Fix

Resolve the violation described in the finding message for `prose.numeric-cap-exceeded`.
