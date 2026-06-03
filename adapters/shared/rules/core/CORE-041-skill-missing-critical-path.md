---
id: CORE-041
title: Skill Missing Critical Path
severity: important
trigger: Skill is missing required critical-path frontmatter.
rule_hints:
  - kind: authoring-predicate
    value: skill.missing-critical-path
    description: Run the retired imperative `skill.missing-critical-path` predicate via the RFC-31 bridge until native hint parity lands.
---

## Rule

This rule delegates to the closed imperative predicate `skill.missing-critical-path` through `kind: authoring-predicate`. Behaviour matches the former `framework::check` row; migrate to native deterministic hints when parity tests cover the fact-iterating form.

## Look For

Violations surfaced by `skill.missing-critical-path` on the framework tree.

## Fix

Resolve the violation described in the finding message for `skill.missing-critical-path`.
