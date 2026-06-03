---
id: CORE-039
title: Skill Inline Json Too Long
severity: important
trigger: Inline JSON in a skill body exceeds the length cap.
rule_hints:
  - kind: authoring-predicate
    value: skill.inline-json-too-long
    description: Run the retired imperative `skill.inline-json-too-long` predicate via the RFC-31 bridge until native hint parity lands.
---

## Rule

This rule delegates to the closed imperative predicate `skill.inline-json-too-long` through `kind: authoring-predicate`. Behaviour matches the former `framework::check` row; migrate to native deterministic hints when parity tests cover the fact-iterating form.

## Look For

Violations surfaced by `skill.inline-json-too-long` on the framework tree.

## Fix

Resolve the violation described in the finding message for `skill.inline-json-too-long`.
