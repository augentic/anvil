---
id: CORE-025
title: Operational Vocabulary
severity: important
trigger: Retired Specify vocabulary appears outside the allowlisted fixtures and archive carve-outs.
rule_hints:
  - kind: path-pattern
    value: "docs/**/*.md"
  - kind: path-pattern
    value: "plugins/**/*.md"
  - kind: path-pattern
    value: ".cursor/**/*.md"
  - kind: path-pattern
    value: "**/AGENTS.md"
  - kind: path-pattern
    value: "**/README.md"
  - kind: path-pattern
    value: "!**/fixtures/**"
  - kind: path-pattern
    value: "!**/archive/**"
  - kind: regex
    value: "\\.specify/changes/"
  - kind: regex
    value: "\\bspecrun\\b"
  - kind: regex
    value: "\\bspecify validate\\b"
  - kind: regex
    value: "\\bspecify merge\\b"
  - kind: regex
    value: "\\bspecify change plan\\b"
  - kind: regex
    value: "\\bspecify change draft\\b"
  - kind: regex
    value: "\\b[Ii]nitiative\\b"
---

## Rule

Scan framework prose for retired Specify vocabulary. Path exclusions cover generated fixtures (`/fixtures/`) and the archive (`/archive/`). Each forbidden pattern is a separate `regex` hint so findings stay line-scoped.

## Look For

- `.specify/changes/` paths instead of `.specify/slices/`
- `specify validate` instead of `specify slice validate`
- `specrun` instead of the shipped `specify` binary name
- `Initiative` instead of `change` / `slice`

## Fix

Replace with the current vocabulary (`.specify/slices/`, `specify slice validate`, `specify`, `change` / `slice`).
