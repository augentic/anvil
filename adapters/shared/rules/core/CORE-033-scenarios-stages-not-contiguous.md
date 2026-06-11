---
id: CORE-033
title: Scenarios Stages Not Contiguous
severity: important
trigger: A scenario's stages list is not a contiguous slice of the slice loop.
rule_hints:
  - kind: path-pattern
    value: adapters/shared/rules/core/CORE-033-scenarios-stages-not-contiguous.md
    description: Sentinel path so the whole-tree scenarios tool runs exactly once; the tool walks PROJECT_DIR itself rather than the passed candidate.
  - kind: tool
    value: scenarios
    description: Run the `scenarios` framework checker, which discovers scenario packs under PROJECT_DIR and flags any whose `stages` list is not a contiguous slice of [plan, refine, build, merge, drop].
---

## Rule

A scenario's `stages` frontmatter list declares the slice-loop window the scenario exercises. The list must be a contiguous slice of the fixed order `[plan, refine, build, merge, drop]`, anchored at any element — for example `[refine, build]` or `[plan, refine, build, merge, drop]`, but never `[plan, build]` (a gap) or `[draft]` (an unknown stage). A non-contiguous list does not describe a runnable lifecycle window.

This check is whole-tree: the `scenarios` framework tool discovers every scenario file under the eval scenario pack, target adapter tests, and plugin skill fixtures, then validates each one's stage contiguity. The rule's `path-pattern` names a single sentinel file so the tool runs exactly once per lint; the tool reads `PROJECT_DIR` and walks the tree itself.

## Look For

- A `stages:` list with a gap, such as `[plan, build]` (missing `refine`).
- A `stages:` list containing a stage outside the closed `{plan, refine, build, merge, drop}` set.
- A `stages:` list whose elements are out of slice-loop order.

## Fix

Reorder the scenario's `stages` list to a contiguous run of `[plan, refine, build, merge, drop]` anchored at the first stage the scenario actually exercises. Drop any unknown stage names.
