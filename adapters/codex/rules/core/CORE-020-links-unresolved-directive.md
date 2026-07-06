---
id: CORE-020
title: Links Unresolved Directive
severity: important
trigger: A skill directive references a path that does not resolve.
rule_hints:
  - kind: path-pattern
    value: adapters/shared/prose/rules/core/CORE-020-links-unresolved-directive.md
    description: Sentinel path so the whole-tree links-registry tool runs exactly once; the tool walks PROJECT_DIR itself rather than the passed candidate.
  - kind: tool
    value: links-registry
    description: Run the `links-registry` framework checker, which resolves each skill directive against the skill registry discovered under plugins/. Pure cross-fact join; carries no policy.
---

## Rule

Every `<!-- skill: plugin:skill -->` directive across the tree must resolve against the skill registry discovered on disk under `plugins/<plugin>/skills/<skill>/`. The registry is built from the tree, so this check carries no policy.

This check is whole-tree: the `links-registry` framework tool walks `PROJECT_DIR` for markdown, ignoring directives inside fenced or inline code, and joins each against the discovered registry. The rule's `path-pattern` names a single sentinel file so the tool runs exactly once per lint; the tool reads `PROJECT_DIR` and walks the tree itself.

## Look For

- A directive naming a plugin with no directory under `plugins/`.
- A directive naming a skill that does not exist under its plugin's `skills/`.

## Fix

Fix the `<!-- skill: plugin:skill -->` directive to name an existing plugin and skill, or add the missing skill under `plugins/<plugin>/skills/`.
