---
id: CORE-047
title: Skill Unknown Tool
severity: important
trigger: SKILL.md allowed-tools lists a tool name that is not recognized by the framework tool registry.
rule_hints:
  - kind: authoring-predicate
    value: skill.unknown-tool
    description: Run the retired imperative `skill.unknown-tool` predicate via the RFC-31 bridge until native hint parity lands.
---

## Rule

This rule delegates to the closed imperative predicate `skill.unknown-tool` through `kind: authoring-predicate`. Behaviour matches the former `framework::check` row; migrate to native deterministic hints when parity tests cover the fact-iterating form.

## Look For

Violations surfaced by `skill.unknown-tool` on the framework tree.

## Fix

Resolve the violation described in the finding message for `skill.unknown-tool`.
