---
id: CORE-040
title: Skill Invalid Critical Path
severity: important
trigger: Skill critical-path frontmatter is invalid.
rule_hints:
  - kind: path-pattern
    value: adapters/shared/prose/rules/core/CORE-040-skill-invalid-critical-path.md
    description: Sentinel path so the whole-tree skill-body tool runs exactly once; the tool walks PROJECT_DIR/plugins itself rather than the passed candidate.
  - kind: tool
    value: skill-body
    config:
      min-body-lines: 150
      min-items: 5
      max-items: 7
    description: Run the `skill-body` framework checker, which flags any long SKILL.md whose `## Critical Path` section does not list between min-items and max-items steps. The thresholds are policy carried here, not in the tool.
---

## Rule

A skill whose body is at least `min-body-lines` lines long and carries a `## Critical Path` section must list between `min-items` and `max-items` entries (numbered list, bullet list, or H3 headings). The Critical Path is the skill's table of contents; a section with too few or too many entries no longer maps faithfully to the body.

This check is whole-tree: the `skill-body` framework tool discovers every `SKILL.md` under `plugins/`, then validates the Critical Path shape of each long skill. The rule's `path-pattern` names a single sentinel file so the tool runs exactly once per lint; the tool reads `PROJECT_DIR` and walks the tree itself. The line threshold and item bounds are supplied in `config:` so the policy lives in this rule file, not the tool.

## Look For

- A long skill whose `## Critical Path` lists fewer than `min-items` steps.
- A long skill whose `## Critical Path` lists more than `max-items` steps.

## Fix

Rewrite the `## Critical Path` section to list the configured number of bullets, numbered items, or H3 step headings — a concise table of contents for the skill body.
