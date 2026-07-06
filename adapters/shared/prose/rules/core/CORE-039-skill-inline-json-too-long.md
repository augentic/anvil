---
id: CORE-039
title: Skill Inline Json Too Long
severity: important
trigger: Inline JSON in a skill body exceeds the length cap.
rule_hints:
  - kind: path-pattern
    value: "plugins/**/SKILL.md"
    description: Narrow the candidate set to plugin skill manifests before the inline-JSON length check fires.
  - kind: fenced-block
    value: inline-json-too-long
    description: Flag every `json` / `jsonc` fenced block in the candidate set whose body exceeds `config.max-lines`. One finding per over-budget fence.
    config:
      langs:
        - json
        - jsonc
      max-lines: 30
---

## Rule

Inline `json` / `jsonc` fences in a skill body stay within the length cap. A long output shape inlined in a skill body crowds out the algorithm spine and duplicates material that belongs in a single canonical reference; relocate it to `docs/reference/cli-output-shapes.md` and link to it instead.

The deterministic-hint interpreter consumes the `FencedBlock` facts the framework indexer already produced, restricted to the `json` / `jsonc` info strings in the `plugins/**/SKILL.md` candidate set. The language allow-list and the line cap are policy carried in the rule's `config:`, not the engine.

## Look For

- A `json` fence in a skill body that pasted an entire CLI envelope or large config example.
- A `jsonc` fence inlining a long annotated configuration that belongs in a reference.

## Fix

Move the large output shape to `docs/reference/cli-output-shapes.md` (or the appropriate reference) and replace the fence with a link to it.
