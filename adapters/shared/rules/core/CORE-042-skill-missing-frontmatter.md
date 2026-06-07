---
id: CORE-042
title: Skill Missing Frontmatter
severity: important
trigger: SKILL.md is missing YAML frontmatter.
rule_hints:
  - kind: path-pattern
    value: adapters/shared/rules/core/CORE-042-skill-missing-frontmatter.md
    description: Sentinel path so the whole-tree skill tool runs exactly once; the tool walks PROJECT_DIR/plugins itself rather than the passed candidate.
  - kind: tool
    value: skill
    description: Run the `skill` framework checker, which flags any SKILL.md whose leading YAML frontmatter block is absent or unparseable. Presence-only; carries no policy.
---

## Rule

Every skill ships as a `plugins/<plugin>/skills/<skill>/SKILL.md` file with a leading YAML frontmatter block delimited by `---`. The runtime reads that block to register the skill, so a SKILL.md with no parseable frontmatter cannot be loaded at all.

This rule is presence-only and stays disjoint from CORE-044 (`skill.schema-violation`): CORE-042 flags a SKILL.md whose frontmatter block is absent or unparseable, while CORE-044 validates the *present* frontmatter against `skill.schema.json` (and structurally skips files with no frontmatter). The two never flag the same file.

This check is whole-tree: the `skill` framework tool discovers every `SKILL.md` under `plugins/`. The rule's `path-pattern` names a single sentinel file so the tool runs exactly once per lint; the tool reads `PROJECT_DIR` and walks the tree itself.

## Look For

- A `SKILL.md` with no leading `---` … `---` frontmatter block.
- A `SKILL.md` whose frontmatter block is present but not parseable as YAML.

## Fix

Add a leading YAML frontmatter block delimited by `---` carrying at least the required `name` and `description` keys, and ensure it parses as valid YAML.
