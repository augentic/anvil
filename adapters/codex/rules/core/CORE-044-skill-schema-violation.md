---
id: CORE-044
title: Skill Schema Violation
severity: important
trigger: SKILL.md frontmatter fails skill.schema.json.
rule_hints:
  - kind: path-pattern
    value: plugins/**/SKILL.md
    description: Narrow the candidate set to skill definition files under every plugin's skills tree before schema validation.
  - kind: schema
    value: skill
    description: Validate each SKILL.md's frontmatter against the embedded `skill.schema.json` shape (required `name` and `description`, closed property set, name/description/argument-hint patterns).
---

## Rule

Every skill ships as a `plugins/<plugin>/skills/<skill>/SKILL.md` file with a leading YAML frontmatter block matching `skill.schema.json`: required `name` and `description` keys, the kebab-case `name` pattern, the `description` "Use when …" clause and length bounds, the `argument-hint` grammar, and no unknown properties. The CLI reads this frontmatter to register the skill, so a block that does not match the schema is a hard failure.

`skill.schema.json` validates only the machine-readable frontmatter between the opening and closing `---` delimiters. A SKILL.md with no frontmatter block is reported separately by `skill.missing-frontmatter`; the directory-prefix invariant on `name`, global skill-name uniqueness, and the curated description-verb allow-list are enforced separately by `specify lint framework`.

## Look For

- A SKILL.md missing the required `name` or `description` key.
- A `name:` value outside the lowercase `^[a-z][a-z0-9-]*$` pattern or over 64 characters.
- A `description:` value missing the `Use when …` clause or outside the 10–512 character bounds.
- An `argument-hint:` value that does not match the closed slash-command grammar, or any unknown frontmatter property.

## Fix

Open the failing SKILL.md, compare its frontmatter against the schema fields above, and either populate the missing key or align the value with the pattern, length, or closed property set. The frontmatter is the canonical authority the runtime reads to register the skill.
