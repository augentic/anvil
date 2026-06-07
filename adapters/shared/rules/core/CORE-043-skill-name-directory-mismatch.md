---
id: CORE-043
title: Skill Name Directory Mismatch
severity: important
trigger: SKILL.md name does not match parent directory.
rule_hints:
  - kind: path-pattern
    value: "plugins/**/SKILL.md"
    description: Narrow the candidate set to plugin skill manifests before the prefix check fires.
  - kind: constant-eq
    value: skill-name-plugin-prefix
    description: Every well-formed skill `name` must begin with its plugin's discovery prefix (`<plugin>-`), modulo the `config.overrides` map. One finding per offending skill.
    config:
      overrides:
        spec: specify
---

## Rule

Every skill `name` must begin with the discovery prefix of the plugin that owns it: a skill under `plugins/<plugin>/skills/.../SKILL.md` must be named `<plugin>-<skill>`. Cursor discovers skills by this prefix, so a mismatch makes the skill undiscoverable under its plugin. Where a plugin directory differs from its published prefix the rule carries an override (the `spec` plugin publishes the `specify-` prefix).

The deterministic-hint interpreter consumes the `Skill` facts the framework indexer already produced (`name` plus owning `plugin` slug), restricted to the `plugins/**/SKILL.md` candidate set. The override map is policy carried in the rule's `config:`, not the engine. Names that are not well-formed kebab-case are left to the schema and grammar predicates.

## Look For

- A skill under `plugins/capture/skills/wiretapper/` named `wiretapper` instead of `capture-wiretapper`.
- A skill under the `spec` plugin named `spec-init` instead of `specify-init`.

## Fix

Rename the skill's `name` to carry its plugin prefix (`<plugin>-<skill>`), or add a `config.overrides` entry when a plugin directory deliberately publishes a different prefix.
