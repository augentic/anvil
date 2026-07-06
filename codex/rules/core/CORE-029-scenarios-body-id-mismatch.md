---
id: CORE-029
title: Scenarios Body Id Mismatch
severity: important
trigger: Scenario body id disagrees with frontmatter id.
rule_hints:
  - kind: path-pattern
    value: codex/rules/core/CORE-029-scenarios-body-id-mismatch.md
    description: Sentinel path so the whole-tree scenarios tool runs exactly once; the tool walks PROJECT_DIR itself rather than the passed candidate.
  - kind: tool
    value: scenarios
    description: Run the `scenarios` framework checker, which discovers scenario packs under PROJECT_DIR and flags any whose visible `Scenario ID:` body line disagrees with the frontmatter `id`.
---

## Rule

A scenario's visible `Scenario ID:` body line must match its frontmatter `id`. When the two disagree, readers cannot trust the citation that links the prose back to the structured scenario.

This check is whole-tree: the `scenarios` framework tool discovers every scenario file under the eval scenario pack, target adapter tests, and plugin skill fixtures, then compares each one's body id against its frontmatter id. The rule's `path-pattern` names a single sentinel file so the tool runs exactly once per lint; the tool reads `PROJECT_DIR` and walks the tree itself.

## Look For

- A visible `Scenario ID:` body line whose id differs from the frontmatter `id`.

## Fix

Align the body `Scenario ID:` line with the frontmatter `id`.
