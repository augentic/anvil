---
id: CORE-042
title: Skill Missing Frontmatter
severity: important
trigger: SKILL.md is missing YAML frontmatter.
rule_hints:
  - kind: path-pattern
    value: plugins/**/SKILL.md
    description: Candidate set of every SKILL.md the `presence` frontmatter selector then narrows on.
  - kind: presence
    value: frontmatter
    description: Flag any candidate SKILL.md absent from the frontmatter fact family (no leading block, unparseable YAML, or an empty block). Presence-only; carries no policy.
---

## Rule

Every skill ships as a `plugins/<plugin>/skills/<skill>/SKILL.md` file with a leading YAML frontmatter block delimited by `---`. The runtime reads that block to register the skill, so a SKILL.md with no parseable frontmatter cannot be loaded at all.

This rule is presence-only and stays disjoint from CORE-044 (`skill.schema-violation`): CORE-042 flags a SKILL.md whose frontmatter block is absent, unparseable, or empty, while CORE-044 validates the *present* frontmatter against `skill.schema.json` (and structurally skips files with no frontmatter). The two never flag the same file.

This check runs natively: the `path-pattern` hint selects every `SKILL.md` under `plugins/`, and the `kind: presence` hint with `value: frontmatter` flags each candidate that the lint indexer did not record a non-empty frontmatter fact for (absent, unparseable, or an empty block).

## Look For

- A `SKILL.md` with no leading `---` … `---` frontmatter block.
- A `SKILL.md` whose frontmatter block is present but not parseable as YAML.

## Fix

Add a leading YAML frontmatter block delimited by `---` carrying at least the required `name` and `description` keys, and ensure it parses as valid YAML.
