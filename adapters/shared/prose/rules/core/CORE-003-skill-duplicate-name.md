---
id: CORE-003
title: Skill Names Unique
severity: important
trigger: "Two or more `plugins/**/SKILL.md` files declare the same `name:` frontmatter value, leaving the marketplace with an ambiguous skill identifier."
rule_hints:
  - kind: path-pattern
    value: "plugins/**/SKILL.md"
    description: Narrow the candidate set to plugin skill manifests before the uniqueness check fires.
  - kind: unique
    value: skill
    config:
      field: skill-name
    description: Walk every `Skill` fact the framework-profile indexer extracted and flag each `name:` value that appears on two or more SKILL.md files. The field selector lives in `config`.
---

## Rule

Every `SKILL.md` under `plugins/<plugin>/skills/<skill>/` declares a single `name:` field in its YAML frontmatter, and that name must be globally unique across the framework repo. The Cursor plugin marketplace, the discovery prefix predicate, and the per-plugin invocation router all key off this value: two skills sharing one name leave the marketplace with an ambiguous slash-command resolution and the prefix-mismatch rule with no stable owner to attribute against.

The path scope covers only well-formed `plugins/<plugin>/skills/<skill>/SKILL.md` paths. Files that the framework-profile indexer drops upstream (non-skill markdown, malformed frontmatter, missing `name:`) never reach the uniqueness check.

The deterministic-hint interpreter consumes the `Skill` facts the framework indexer already produced (`crates/standards/src/lint/index/skill.rs::extract`), so the rule cost is one grouping pass over `WorkspaceModel.skills` at lint time.

## Look For

- Two newly added SKILL.md files in different plugin directories accidentally sharing the same `name:` value (most common during a copy-paste authoring slip).
- A renamed plugin that did not update its skill `name:` prefix, colliding with the original directory's skill.
- A skill being moved between plugins without renaming `name:` to match the new plugin's discovery prefix, leaving the old `name:` in place alongside the moved file's new sibling.

## Fix

Pick which skill keeps the disputed name (typically the one that shipped first) and rename the other to a fresh, prefix-aligned identifier (`<plugin>-<skill>`). Update every downstream invocation site — marketplace manifests, slash-command examples, doc references — in the same change.
