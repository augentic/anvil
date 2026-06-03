---
id: CORE-046
title: Skill Step Body Duplicates Critical Path
severity: important
trigger: Step body duplicates critical-path content.
rule_hints:
  - kind: authoring-predicate
    value: skill.step-body-duplicates-critical-path
    description: Run the retired imperative `skill.step-body-duplicates-critical-path` predicate via the RFC-31 bridge until native hint parity lands.
---

## Rule

This rule delegates to the closed imperative predicate `skill.step-body-duplicates-critical-path` through `kind: authoring-predicate`. Behaviour matches the former `framework::check` row; migrate to native deterministic hints when parity tests cover the fact-iterating form.

## Look For

Violations surfaced by `skill.step-body-duplicates-critical-path` on the framework tree.

## Fix

Resolve the violation described in the finding message for `skill.step-body-duplicates-critical-path`.
