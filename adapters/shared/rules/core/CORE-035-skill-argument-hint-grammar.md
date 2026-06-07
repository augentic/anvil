---
id: CORE-035
title: Skill Argument Hint Grammar
severity: important
trigger: SKILL.md argument-hint violates authoring grammar.
rule_hints:
  - kind: path-pattern
    value: adapters/shared/rules/core/CORE-035-skill-argument-hint-grammar.md
    description: Sentinel path so the whole-tree skill tool runs exactly once; the tool walks PROJECT_DIR/plugins itself rather than the passed candidate.
  - kind: tool
    value: skill
    config:
      token-pattern: '^(?:<[a-z][a-z0-9]*(?:-[a-z0-9]+)*(?:\|[a-z][a-z0-9]*(?:-[a-z0-9]+)*)*>(?:\.\.\.)?|\[[a-z][a-z0-9]*(?:-[a-z0-9]+)*(?:\|[a-z][a-z0-9]*(?:-[a-z0-9]+)*)*\](?:\.\.\.)?|--[a-z][a-z0-9]*(?:-[a-z0-9]+)*)$'
    description: Run the `skill` framework checker, which flags any SKILL.md whose `argument-hint` carries a whitespace-separated token that does not match the grammar. The token grammar is policy carried here, not in the tool.
---

## Rule

A skill's `argument-hint` frontmatter field, when present, must be a string whose whitespace-separated tokens each match the closed slash-command argument grammar: `<name>`, `[name]`, `<a|b>`, `[a|b]`, `<name>...`, `[name]...`, or `--flag`, with kebab-case names. The grammar is supplied as the `token-pattern` regex in `config:` so the policy lives in this rule file, not the tool.

This check is whole-tree: the `skill` framework tool discovers every `SKILL.md` under `plugins/`, then validates each one's `argument-hint`. The rule's `path-pattern` names a single sentinel file so the tool runs exactly once per lint; the tool reads `PROJECT_DIR` and walks the tree itself.

## Look For

- An `argument-hint` that is not a string.
- An `argument-hint` token that does not match the `token-pattern` grammar (for example free-form prose).

## Fix

Rewrite each `argument-hint` token using the closed grammar (`<name>`, `[name]`, `<a|b>`, `--flag`, with optional `...`), using kebab-case names.
