---
id: CORE-046
title: Skill Step Body Duplicates Critical Path
severity: important
trigger: Step body duplicates critical-path content.
rule_hints:
  - kind: path-pattern
    value: adapters/shared/prose/rules/core/CORE-046-skill-step-body-duplicates-critical-path.md
    description: Sentinel path so the whole-tree skill-body tool runs exactly once; the tool walks PROJECT_DIR/plugins itself rather than the passed candidate.
  - kind: tool
    value: skill-body
    description: Run the `skill-body` framework checker, which flags any SKILL.md whose step bodies repeat a `## Critical Path` entry verbatim. This check is structural and carries no policy.
---

## Rule

The `## Critical Path` section is a skill's table of contents; the step bodies that follow it must be short pointers to references, not a verbatim restatement of the Critical Path entries. A step body line that normalises to the same text as a Critical Path entry duplicates the table of contents and bloats the skill.

This check is whole-tree: the `skill-body` framework tool discovers every `SKILL.md` under `plugins/`, then compares each skill's post-Critical-Path body against its Critical Path entries. The rule's `path-pattern` names a single sentinel file so the tool runs exactly once per lint; the tool reads `PROJECT_DIR` and walks the tree itself.

## Look For

- A list item or heading after the `## Critical Path` section that matches a Critical Path entry verbatim (ignoring list markers, step prefixes, and case).

## Fix

Keep step bodies as short pointers to references; do not restate the Critical Path entries in the body that follows.
