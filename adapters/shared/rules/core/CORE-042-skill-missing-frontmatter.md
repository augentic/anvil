---
id: CORE-042
title: Skill Missing Frontmatter
severity: important
trigger: SKILL.md is missing YAML frontmatter.
rule_hints:
  - kind: authoring-predicate
    value: skill.missing-frontmatter
    description: Run the retired imperative `skill.missing-frontmatter` predicate via the RFC-31 bridge until native hint parity lands.
---

## Rule

This rule delegates to the closed imperative predicate `skill.missing-frontmatter` through `kind: authoring-predicate`. Behaviour matches the former `framework::check` row; migrate to native deterministic hints when parity tests cover the fact-iterating form.

## Look For

Violations surfaced by `skill.missing-frontmatter` on the framework tree.

## Fix

Resolve the violation described in the finding message for `skill.missing-frontmatter`.
