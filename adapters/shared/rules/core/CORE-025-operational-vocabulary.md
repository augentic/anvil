---
id: CORE-025
title: Operational Vocabulary
severity: important
trigger: Retired Specify vocabulary appears outside the allowlisted fixtures and archive carve-outs.
rule_hints:
  - kind: path-pattern
    value: "{docs/**/*.md,plugins/**/*.md,.cursor/**/*.md,**/AGENTS.md,**/README.md}"
    description: One brace alternation over every prose surface the rule scans.
  - kind: path-pattern
    value: "!**/{fixtures,archive}/**"
  - kind: regex
    value: "\\.specify/changes/|\\bspecrun\\b|\\bspecify validate\\b|\\bspecify merge\\b|\\bspecify change plan\\b|\\bspecify change draft\\b|\\b[Ii]nitiative\\b"
    description: One alternation over every retired vocabulary form; findings stay line-scoped because the evaluator reports each matching line, and the matched alternative is visible in the finding snippet.
---

## Rule

Scan framework prose for retired Specify vocabulary. Path exclusions cover generated fixtures (`/fixtures/`) and the archive (`/archive/`). The forbidden forms ride a single multi-pattern `regex` hint; the evaluator reports each matching line, so findings stay line-scoped and the offending alternative is visible in the snippet.

## Look For

- `.specify/changes/` paths instead of `.specify/slices/`
- `specify validate` instead of `specify slice validate`
- `specrun` instead of the shipped `specify` binary name
- `Initiative` instead of `change` / `slice`

## Fix

Replace with the current vocabulary (`.specify/slices/`, `specify slice validate`, `specify`, `change` / `slice`).
