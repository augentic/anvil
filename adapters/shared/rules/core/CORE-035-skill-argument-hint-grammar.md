---
id: CORE-035
title: Skill Argument Hint Grammar
severity: important
trigger: SKILL.md argument-hint violates authoring grammar.
rule_hints:
  - kind: authoring-predicate
    value: skill.argument-hint-grammar
    description: Run the retired imperative `skill.argument-hint-grammar` predicate via the RFC-31 bridge until native hint parity lands.
---

## Rule

This rule delegates to the closed imperative predicate `skill.argument-hint-grammar` through `kind: authoring-predicate`. Behaviour matches the former `framework::check` row; migrate to native deterministic hints when parity tests cover the fact-iterating form.

## Look For

Violations surfaced by `skill.argument-hint-grammar` on the framework tree.

## Fix

Resolve the violation described in the finding message for `skill.argument-hint-grammar`.
