---
id: CORE-026
title: Rules Duplicate Rule Id
severity: important
trigger: The same rule id appears in more than one rules markdown file.
rule_hints:
  - kind: path-pattern
    value: adapters/codex/rules/core/CORE-026-rules-duplicate-rule-id.md
    description: Sentinel path so the whole-tree rules tool runs exactly once; the tool walks PROJECT_DIR itself rather than the passed candidate.
  - kind: tool
    value: rules
    description: Run the `rules` framework checker scoped to CORE-026. It walks the whole rules tree, collects every `id`, and flags any id declared in more than one file. No policy is required — duplicate detection is structural.
---

## Rule

Every rule id is globally unique across the first-party rule set: a `CORE-`, `UNI-`, `OMNIA-`, `VECTIS-`, `IFACE-`, or `SRC-` id must be declared by exactly one rules markdown file. A duplicate id means two files claim the same codex entry, so consumers of the resolved codex cannot resolve a single rule.

The check is whole-tree: the `rules` framework tool walks `PROJECT_DIR` itself, reads each rule file's `id` frontmatter across the target / source axes and the shared `universal` / `core` packs, and flags any id that appears in more than one file. The rule's `path-pattern` names a single sentinel file so the tool runs exactly once per lint; the duplicate scan needs no policy from `config:`.

## Look For

- Two rule files under the same pack sharing an id after a copy-paste.
- A rule renumbered in one place but not another, leaving the old id duplicated.

## Fix

Rename the colliding rules so each frontmatter `id` is unique across the rules tree.
