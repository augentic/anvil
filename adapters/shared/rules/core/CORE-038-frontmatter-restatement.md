---
id: CORE-038
title: Frontmatter Restatement
severity: important
trigger: A `SKILL.md` body contains a `## Input` section that restates argument-hint frontmatter already rendered on every invocation.
rule_hints:
  - kind: path-pattern
    value: "plugins/**/skills/**/SKILL.md"
  - kind: regex
    value: "(?m)^## Input\\s*$"
    description: Flag body lines that restate frontmatter as a dedicated Input H2.
---

## Rule

The `argument-hint` frontmatter field is rendered on every invocation. A `## Input` H2 duplicates that surface and must not appear in the skill body.

## Look For

- `## Input` heading in `plugins/<plugin>/skills/<skill>/SKILL.md` after the frontmatter fence.

## Fix

Drop the `## Input` section; move inference or prompt instructions into Critical Path step 1.
