---
id: CORE-030
title: Scenarios Duplicate Id
severity: important
trigger: Duplicate scenario ids across files.
rule_hints:
  - kind: path-pattern
    value: adapters/shared/prose/rules/core/CORE-030-scenarios-duplicate-id.md
    description: Sentinel include so the rule carries a candidate set; the `scenario` unique selector evaluates the whole scenario fact family and ignores the candidate set.
  - kind: unique
    value: scenario
    config:
      field: id
    description: Flag any frontmatter `id` shared by more than one discovered scenario, across the whole tree.
---

## Rule

Each scenario's frontmatter `id` must be unique across the whole tree. A duplicate id makes scenario citations ambiguous and breaks cross-references.

This check is whole-tree: the `kind: unique` hint with `value: scenario` and `config: { field: id }` groups the `scenario` fact family by frontmatter `id` and flags any id claimed by more than one file. The rule's `path-pattern` is a sentinel include; the scenario unique selector evaluates the whole fact family regardless of the candidate set.

## Look For

- Two or more scenario files declaring the same frontmatter `id`.

## Fix

Rename the colliding scenarios so each frontmatter `id` is unique across the tree.
