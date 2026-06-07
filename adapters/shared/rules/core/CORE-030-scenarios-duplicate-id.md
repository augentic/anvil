---
id: CORE-030
title: Scenarios Duplicate Id
severity: important
trigger: Duplicate scenario ids across files.
rule_hints:
  - kind: path-pattern
    value: adapters/shared/rules/core/CORE-030-scenarios-duplicate-id.md
    description: Sentinel path so the whole-tree scenarios tool runs exactly once; the tool walks PROJECT_DIR itself rather than the passed candidate.
  - kind: tool
    value: scenarios
    description: Run the `scenarios` framework checker, which discovers scenario packs under PROJECT_DIR and flags any frontmatter `id` shared by more than one scenario file.
---

## Rule

Each scenario's frontmatter `id` must be unique across the whole tree. A duplicate id makes scenario citations ambiguous and breaks cross-references.

This check is whole-tree: the `scenarios` framework tool discovers every scenario file under the acceptance scenario pack, target adapter tests, and plugin skill fixtures, then groups them by `id` and flags any id claimed by more than one file. The rule's `path-pattern` names a single sentinel file so the tool runs exactly once per lint; the tool reads `PROJECT_DIR` and walks the tree itself.

## Look For

- Two or more scenario files declaring the same frontmatter `id`.

## Fix

Rename the colliding scenarios so each frontmatter `id` is unique across the tree.
