---
id: CORE-023
title: Invocation Positional
severity: important
trigger: Slash-skill invocations use `--flag` tokens after the skill name instead of positional skill arguments.
rule_hints:
  - kind: path-pattern
    value: "docs/**/*.md"
  - kind: path-pattern
    value: "plugins/**/*.md"
  - kind: path-pattern
    value: "adapters/**/*.md"
  - kind: path-pattern
    value: "**/AGENTS.md"
  - kind: path-pattern
    value: "**/README.md"
  - kind: path-pattern
    value: "rfcs/roadmap.md"
  - kind: path-pattern
    value: ".cursor/rules/**/*.mdc"
  - kind: path-pattern
    value: "!docs/proposals/**"
  - kind: path-pattern
    value: "!adapters/shared/prose/rules/**"
  - kind: regex
    value: "slash-skill-positional"
    config:
      slash-skill-positional: true
      join-backslash-continuations: true
    description: Scan logical lines (including backslash continuations) for flag tokens after `/plugin:skill` without an intervening CLI command.
---

## Rule

Slash skills (`/plugin:skill`) take positional arguments only. Reserve `--flags` for underlying CLI commands (`specify`, `cargo`, `gh`, …), not for skill tokens.

## Look For

- `/spec:refine --force` on one line or split across `\` continuations.
- Flag tokens immediately after a slash-skill token without a CLI verb between.

## Fix

Rewrite as positional skill arguments per [docs/standards/skill-authoring.md](../../../../docs/standards/skill-authoring.md).
