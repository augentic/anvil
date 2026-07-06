---
id: CORE-022
title: Plugins Marketplace Drift
severity: important
trigger: marketplace.json drifts from on-disk plugin layout.
rule_hints:
  - kind: path-pattern
    value: codex/rules/core/CORE-022-plugins-marketplace-drift.md
    description: Sentinel path so the whole-tree marketplace tool runs exactly once; the tool walks PROJECT_DIR itself rather than the passed candidate.
  - kind: tool
    value: marketplace
    description: Run the `marketplace` framework checker, which validates `.cursor-plugin/marketplace.json` against its schema and checks bidirectional consistency with the plugins/ tree. Whole-tree cross-fact; carries no policy.
---

## Rule

The `.cursor-plugin/marketplace.json` manifest must satisfy its schema and agree bidirectionally with the on-disk `plugins/` layout: every declared plugin has a `skills/` directory and a `.cursor-plugin/plugin.json`, and every on-disk `plugin.json` is declared in the manifest. The manifest schema is embedded in the tool as mechanism; the layout is structural, so this check carries no policy.

This check is whole-tree: the `marketplace` framework tool reads the manifest and walks `PROJECT_DIR/plugins` itself. The rule's `path-pattern` names a single sentinel file so the tool runs exactly once per lint; the tool reads `PROJECT_DIR` and walks the tree itself.

## Look For

- A plugin declared in `marketplace.json` with no `skills/` directory or `plugin.json`.
- An on-disk `plugins/<plugin>/.cursor-plugin/plugin.json` not declared in `marketplace.json`.
- A `marketplace.json` that does not satisfy the marketplace schema.

## Fix

Reconcile `.cursor-plugin/marketplace.json` with the `plugins/` tree: declare every on-disk plugin and ensure each declared plugin has `skills/` and a `plugin.json`.
