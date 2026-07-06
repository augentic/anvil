---
id: CORE-028
title: Scenarios Artifact Path Unsafe
severity: important
trigger: A scenario references an unsafe artifact path.
rule_hints:
  - kind: path-pattern
    value: codex/rules/core/CORE-028-scenarios-artifact-path-unsafe.md
    description: Sentinel path so the whole-tree scenarios tool runs exactly once; the tool walks PROJECT_DIR itself rather than the passed candidate.
  - kind: tool
    value: scenarios
    description: Run the `scenarios` framework checker, which discovers scenario packs under PROJECT_DIR and flags any whose `expected-artifacts` entries are empty, absolute, or escape the scenario workspace.
---

## Rule

Every `expected-artifacts` entry in a scenario's frontmatter must be a non-empty path, relative to the scenario workspace, with no leading `/` and no `..` segment. An empty, absolute, or escaping path cannot be resolved safely against an isolated scenario workspace.

This check is whole-tree: the `scenarios` framework tool discovers every scenario file under the eval scenario pack, target adapter tests, and plugin skill fixtures, then validates each one's `expected-artifacts`. The rule's `path-pattern` names a single sentinel file so the tool runs exactly once per lint; the tool reads `PROJECT_DIR` and walks the tree itself.

## Look For

- An `expected-artifacts` entry that is an empty string.
- An entry beginning with `/` (an absolute path).
- An entry containing a `..` segment (escapes the workspace).

## Fix

Rewrite each `expected-artifacts` entry as a non-empty path relative to the scenario workspace, dropping any leading `/` or `..` segment.
