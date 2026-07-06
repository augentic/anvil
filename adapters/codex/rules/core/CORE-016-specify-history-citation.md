---
id: CORE-016
title: Specify History Citation In Docs
severity: important
trigger: User-facing prose cites retired Specify design-history RFC numbers below 100 (for example `RFC-5`) instead of live decision topics or IETF standards RFCs.
rule_hints:
  - kind: path-pattern
    value: "docs/**/*.md"
  - kind: path-pattern
    value: "adapters/**/*.md"
  - kind: path-pattern
    value: "plugins/**/*.md"
  - kind: path-pattern
    value: "AGENTS.md"
  - kind: path-pattern
    value: "!docs/assets/**"
  - kind: path-pattern
    value: "!adapters/codex/rules/**"
  - kind: regex
    value: "(?i)RFC[-\\s]+(\\d+)"
    config:
      capture-group: 1
      capture-op: lt
      capture-value: 100
    description: Flag when the captured RFC number is below 100 (design-history citations); admit RFC 3339 / RFC 5322 style references.
---

## Rule

Retired Specify design-history citations (`RFC-N` where `N < 100`, `rfcs/…`, and sibling forms) must not appear in operator-facing prose. Standards RFCs at 100 and above are allowed. Point-in-time review documents (`REVIEW.md`) are out of scope — a review legitimately discusses design history by name.

## Look For

- `RFC-5` or `RFC 5` in docs, plugins, adapters, or `AGENTS.md`.
- Markdown links into `rfcs/` trees with numbered design-history paths.

## Fix

Strip the historical reference, or cite a current reference doc instead.
