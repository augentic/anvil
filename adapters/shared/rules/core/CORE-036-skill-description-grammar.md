---
id: CORE-036
title: Skill Description Grammar
severity: important
trigger: SKILL.md description violates authoring grammar.
rule_hints:
  - kind: authoring-predicate
    value: skill.description-grammar
    description: Run the retired imperative `skill.description-grammar` predicate via the RFC-31 bridge until native hint parity lands.
---

## Rule

This rule delegates to the closed imperative predicate `skill.description-grammar` through `kind: authoring-predicate`. Behaviour matches the former `framework::check` row; migrate to native deterministic hints when parity tests cover the fact-iterating form.

## Look For

Violations surfaced by `skill.description-grammar` on the framework tree.

## Fix

Resolve the violation described in the finding message for `skill.description-grammar`.
