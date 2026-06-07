---
id: CORE-045
title: Skill Section Line Count
severity: important
trigger: A skill section exceeds the line budget.
rule_hints:
  - kind: path-pattern
    value: "plugins/**/SKILL.md"
    description: Narrow the candidate set to plugin skill manifests before the per-section budget check fires.
  - kind: cardinality
    value: markdown-h2-section-body-line-count
    description: For each `##` (level-2) section in the candidate set, assert that its body line count is at most `config.max`. One finding per over-budget section.
    config:
      max: 45
---

## Rule

Every `##` section in a `SKILL.md` body stays within the per-section line budget. A section that grows past the cap is the cue to relocate depth into a `references/<topic>.md` sibling and link to it from the H2, keeping the skill body scannable.

The deterministic-hint interpreter consumes the `MarkdownSection` facts the framework indexer already produced, restricted to level-2 sections in the `plugins/**/SKILL.md` candidate set. The line cap is policy carried in the rule's `config:`, not the engine.

## Look For

- A `## Critical Path` or `## Steps` section that inlined every edge case instead of linking to a reference.
- A section that absorbed a worked example that belongs under `references/`.

## Fix

Move the long-form material into `references/<topic>.md` and replace it in the section body with a one-line link from the H2.
