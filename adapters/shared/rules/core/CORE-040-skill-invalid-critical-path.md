---
id: CORE-040
title: Skill Invalid Critical Path
severity: important
trigger: Skill critical-path frontmatter is invalid.
rule_hints:
  - kind: authoring-predicate
    value: skill.invalid-critical-path
    description: Run the retired imperative `skill.invalid-critical-path` predicate via the RFC-31 bridge until native hint parity lands.
---

## Rule

This rule delegates to the closed imperative predicate `skill.invalid-critical-path` through `kind: authoring-predicate`. Behaviour matches the former `framework::check` row; migrate to native deterministic hints when parity tests cover the fact-iterating form.

## Look For

Violations surfaced by `skill.invalid-critical-path` on the framework tree.

## Fix

Resolve the violation described in the finding message for `skill.invalid-critical-path`.
