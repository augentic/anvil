---
id: CORE-035
title: Skill Argument Hint Grammar
severity: important
trigger: SKILL.md argument-hint violates authoring grammar.
rule_hints:
  - kind: path-pattern
    value: plugins/**/SKILL.md
    description: Candidate set of every SKILL.md the `field-grammar` field-tokens mode then narrows on.
  - kind: field-grammar
    value: field-tokens
    config:
      field: argument-hint
      token-pattern: '^(?:<[a-z][a-z0-9]*(?:-[a-z0-9]+)*(?:\|[a-z][a-z0-9]*(?:-[a-z0-9]+)*)*>(?:\.\.\.)?|\[[a-z][a-z0-9]*(?:-[a-z0-9]+)*(?:\|[a-z][a-z0-9]*(?:-[a-z0-9]+)*)*\](?:\.\.\.)?|--[a-z][a-z0-9]*(?:-[a-z0-9]+)*)$'
    description: Flag any SKILL.md whose `argument-hint` carries a whitespace-separated token that does not match the grammar. The field name and token grammar are policy carried here, not in the engine.
---

## Rule

A skill's `argument-hint` frontmatter field, when present, must be a string whose whitespace-separated tokens each match the closed slash-command argument grammar: `<name>`, `[name]`, `<a|b>`, `[a|b]`, `<name>...`, `[name]...`, or `--flag`, with kebab-case names. The grammar is supplied as the `token-pattern` regex in `config:` so the policy lives in this rule file, not the engine.

This check runs natively: the `path-pattern` hint selects every `SKILL.md` under `plugins/`, and the `kind: field-grammar` hint with `value: field-tokens` splits each candidate's `argument-hint` field on whitespace and flags any token that fails the `token-pattern` regex (a present `argument-hint` that is not a string is flagged outright).

## Look For

- An `argument-hint` that is not a string.
- An `argument-hint` token that does not match the `token-pattern` grammar (for example free-form prose).

## Fix

Rewrite each `argument-hint` token using the closed grammar (`<name>`, `[name]`, `<a|b>`, `--flag`, with optional `...`), using kebab-case names.
