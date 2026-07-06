---
id: CORE-048
title: Skill Variable Coverage
severity: important
trigger: Template variables in the skill body lack coverage.
rule_hints:
  - kind: path-pattern
    value: adapters/shared/prose/rules/core/CORE-048-skill-variable-coverage.md
    description: Sentinel path so the whole-tree skill-body tool runs exactly once; the tool walks PROJECT_DIR/plugins itself rather than the passed candidate.
  - kind: tool
    value: skill-body
    config:
      builtin-vars:
        - ARGUMENTS
        - HOME
    description: Run the `skill-body` framework checker, which flags `$VAR`s defined in a skill's Arguments section but never referenced, and all-caps `$VAR`s used in the body but never defined. The built-in exempt set is policy carried here, not in the tool.
---

## Rule

A skill that defines `$VAR`s in its `## Arguments` (or `## Derived Arguments`) section must reference each one in the body, and any all-caps `$VAR` used in the body must be defined in Arguments. Built-in variables in the configured allow-list are exempt from both directions. A defined-but-unused or used-but-undefined variable is a broken contract between the Arguments section and the skill body.

This check is whole-tree: the `skill-body` framework tool discovers every `SKILL.md` under `plugins/`, then performs the definition / reference analysis on each. The rule's `path-pattern` names a single sentinel file so the tool runs exactly once per lint; the tool reads `PROJECT_DIR` and walks the tree itself. The built-in variable allow-list is supplied in `config:` so the policy lives in this rule file, not the tool.

## Look For

- A `$VAR` defined in the Arguments section but never referenced in the body.
- An all-caps `$VAR` used in the body but never defined in the Arguments section.

## Fix

Reference every defined `$VAR` in the body and define every `$VAR` the body uses in the Arguments section; add genuinely built-in variables to the rule's `builtin-vars` allow-list.
