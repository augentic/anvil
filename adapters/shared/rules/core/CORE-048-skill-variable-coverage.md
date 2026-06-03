---
id: CORE-048
title: Skill Variable Coverage
severity: important
trigger: Template variables in the skill body lack coverage.
rule_hints:
  - kind: authoring-predicate
    value: skill.variable-coverage
    description: Run the retired imperative `skill.variable-coverage` predicate via the RFC-31 bridge until native hint parity lands.
---

## Rule

This rule delegates to the closed imperative predicate `skill.variable-coverage` through `kind: authoring-predicate`. Behaviour matches the former `framework::check` row; migrate to native deterministic hints when parity tests cover the fact-iterating form.

## Look For

Violations surfaced by `skill.variable-coverage` on the framework tree.

## Fix

Resolve the violation described in the finding message for `skill.variable-coverage`.
