---
id: CORE-037
title: Envelope JSON In Body
severity: important
trigger: A `SKILL.md` body embeds a fenced `json` / `jsonc` block that matches the CLI envelope shape (`ok`, `data` / `error`, or `envelope_version`).
rule_hints:
  - kind: path-pattern
    value: "plugins/**/skills/**/SKILL.md"
  - kind: fenced-block
    value: skill-envelope-json-in-body
    description: Fence-aware check over indexed fenced blocks; flags envelope-shaped JSON instead of substring scan.
---

## Rule

Skill bodies must not embed CLI envelope JSON examples. Link to [docs/reference/cli-output-shapes.md](../../../../docs/reference/cli-output-shapes.md) instead.

## Look For

- Fenced `json` blocks containing `"ok":`, `"data":`, `"error":`, or `"envelope_version":`.

## Fix

Remove the inline envelope and reference the canonical CLI output shapes document.
