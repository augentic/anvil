---
id: CORE-041
title: Skill Missing Critical Path
severity: important
trigger: Skill is missing required critical-path frontmatter.
rule_hints:
  - kind: path-pattern
    value: adapters/shared/rules/core/CORE-041-skill-missing-critical-path.md
    description: Sentinel path so the whole-tree skill-body tool runs exactly once; the tool walks PROJECT_DIR/plugins itself rather than the passed candidate.
  - kind: tool
    value: skill-body
    config:
      min-body-lines: 150
    description: Run the `skill-body` framework checker, which flags any SKILL.md whose body reaches min-body-lines but omits a `## Critical Path` section. The threshold is policy carried here, not in the tool.
---

## Rule

A skill whose body is at least `min-body-lines` lines long must carry a `## Critical Path` section. Long skills need a table of contents so a reader can navigate the body; the threshold gates the requirement so short skills are exempt.

This check is whole-tree: the `skill-body` framework tool discovers every `SKILL.md` under `plugins/`, then flags any long skill missing the section. The rule's `path-pattern` names a single sentinel file so the tool runs exactly once per lint; the tool reads `PROJECT_DIR` and walks the tree itself. The line threshold is supplied in `config:` so the policy lives in this rule file, not the tool.

## Look For

- A `SKILL.md` whose body reaches `min-body-lines` lines but has no `## Critical Path` heading.

## Fix

Add a `## Critical Path` section summarising the skill's steps as a short table of contents, or shorten the body below the threshold if the skill does not warrant one.
