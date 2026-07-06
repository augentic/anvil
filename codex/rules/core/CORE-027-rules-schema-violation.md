---
id: CORE-027
title: Rules Schema Violation
severity: critical
trigger: A rules markdown file fails rule.schema.json validation.
rule_hints:
  - kind: path-pattern
    value: "{codex,sources,targets}/**/rules/**/*.md"
    description: Narrow the candidate set to rule markdown files under every source, target, and shared rules tree before schema validation.
  - kind: path-pattern
    value: "!**/README.md"
    description: Exclude rules-pack README catalogs; only rule files carry frontmatter.
  - kind: schema
    value: rule
    description: Validate each rule file's frontmatter against the embedded `rule.schema.json` shape (required `id`, `title`, `severity`, `trigger`, closed hint kinds, no unknown keys).
---

## Rule

Every first-party and adapter rule ships as a markdown file with a leading YAML frontmatter block matching `rule.schema.json`: required `id`, `title`, `severity`, and `trigger` keys, a closed `severity` enum, closed `rule_hints[].kind` constants, and no unknown properties. The CLI parses these files at resolve time to build the codex `specify lint` and `specify rules export` consume, so a frontmatter block that does not match the schema is a hard failure for the rest of the standards layer.

`rule.schema.json` validates only the machine-readable frontmatter between the opening and closing `---` delimiters. Markdown body conventions (the `## Rule` heading, enforced by CORE-053) and cross-file duplicate-id detection (CORE-026) are validated separately by `specify lint framework`.

## Look For

- A new rule file missing one of the four required keys (`id`, `title`, `severity`, `trigger`).
- A `severity:` value outside the closed `{critical, important, suggestion, optional}` set.
- A `rule_hints[]` entry whose `kind` is not one of the closed hint constants, or that carries an unknown property.
- An `id:` outside the reserved namespace + three-digit-suffix pattern.

## Fix

Open the failing rule file, compare its frontmatter against the schema fields above, and either populate the missing or malformed key or align the value with the closed enum / pattern. The frontmatter is the canonical authority — codex resolution, `specify lint`, and `specify rules export` all depend on a clean rule shape.
