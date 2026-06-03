---
id: CORE-014
title: Brief Frontmatter Forbidden
severity: important
trigger: An adapter brief under `adapters/**/briefs/` opens with a YAML frontmatter fence even though briefs are resolved by path only.
rule_hints:
  - kind: path-pattern
    value: "adapters/**/briefs/shape.md"
  - kind: path-pattern
    value: "adapters/**/briefs/build.md"
  - kind: path-pattern
    value: "adapters/**/briefs/merge.md"
  - kind: path-pattern
    value: "adapters/**/briefs/survey.md"
  - kind: path-pattern
    value: "adapters/**/briefs/extract.md"
  - kind: path-pattern
    value: "adapters/**/briefs/build/**/*.md"
  - kind: path-pattern
    value: "adapters/**/briefs/extract/**/*.md"
  - kind: regex
    value: "^---"
    description: Flag line 1 when the brief opens with a frontmatter delimiter.
---

## Rule

Adapter briefs are not skills. The loader resolves them from `adapter.yaml` paths and never reads brief frontmatter. A leading `---` block is always drift.

## Look For

- `---` on the first line of `adapters/sources/<name>/briefs/*.md` or `adapters/targets/<name>/briefs/*.md`.

## Fix

Strip the frontmatter fence and rely on the body H1 for the brief title. See [docs/standards/skill-authoring.md](../../../../docs/standards/skill-authoring.md#brief-authoring).
