---
id: CORE-026
title: Rules Duplicate Rule Id
severity: important
trigger: The same rule id appears in more than one rules markdown file.
rule_hints:
  - kind: authoring-predicate
    value: rules.duplicate-rule-id
    description: Run the retired imperative `rules.duplicate-rule-id` predicate via the RFC-31 bridge until native hint parity lands.
---

## Rule

This rule delegates to the closed imperative predicate `rules.duplicate-rule-id` through `kind: authoring-predicate`. Behaviour matches the former `framework::check` row; migrate to native deterministic hints when parity tests cover the fact-iterating form.

## Look For

Violations surfaced by `rules.duplicate-rule-id` on the framework tree.

## Fix

Resolve the violation described in the finding message for `rules.duplicate-rule-id`.
