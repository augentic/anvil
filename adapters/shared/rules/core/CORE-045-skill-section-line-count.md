---
id: CORE-045
title: Skill Section Line Count
severity: important
trigger: A skill section exceeds the line budget.
rule_hints:
  - kind: authoring-predicate
    value: skill.section-line-count
    description: Run the retired imperative `skill.section-line-count` predicate via the RFC-31 bridge until native hint parity lands.
---

## Rule

This rule delegates to the closed imperative predicate `skill.section-line-count` through `kind: authoring-predicate`. Behaviour matches the former `framework::check` row; migrate to native deterministic hints when parity tests cover the fact-iterating form.

## Look For

Violations surfaced by `skill.section-line-count` on the framework tree.

## Fix

Resolve the violation described in the finding message for `skill.section-line-count`.
