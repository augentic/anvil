---
id: CORE-043
title: Skill Name Directory Mismatch
severity: important
trigger: SKILL.md name does not match parent directory.
rule_hints:
  - kind: authoring-predicate
    value: skill.name-directory-mismatch
    description: Run the retired imperative `skill.name-directory-mismatch` predicate via the RFC-31 bridge until native hint parity lands.
---

## Rule

This rule delegates to the closed imperative predicate `skill.name-directory-mismatch` through `kind: authoring-predicate`. Behaviour matches the former `framework::check` row; migrate to native deterministic hints when parity tests cover the fact-iterating form.

## Look For

Violations surfaced by `skill.name-directory-mismatch` on the framework tree.

## Fix

Resolve the violation described in the finding message for `skill.name-directory-mismatch`.
