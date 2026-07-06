---
id: CORE-041
title: Skill Missing Critical Path
severity: important
trigger: Skill is missing required critical-path frontmatter.
rule_hints:
  - kind: path-pattern
    value: adapters/shared/prose/rules/core/CORE-041-skill-missing-critical-path.md
    description: Sentinel include so the rule carries a candidate set; the `presence` markdown-section selector evaluates the whole skill fact family and ignores the candidate set.
  - kind: presence
    value: markdown-section
    config:
      title: Critical Path
      level: 2
      when:
        metric: skill-body-line-count
        min: 149
    description: Flag any skill whose body line count reaches the threshold but carries no `## Critical Path` H2 section. The section title, level, and threshold are policy carried here, not in the engine.
---

## Rule

A skill whose body is long enough must carry a `## Critical Path` section. Long skills need a table of contents so a reader can navigate the body; the threshold gates the requirement so short skills are exempt.

This check runs natively: the `kind: presence` hint with `value: markdown-section` iterates the skill fact family, and for each skill whose `skill-body-line-count` reaches `when.min` flags those lacking a markdown section with the configured `title` and `level`. The rule's `path-pattern` is a sentinel include; the markdown-section selector evaluates the whole skill fact family regardless of the candidate set. The title, level, and threshold are supplied in `config:` so the policy lives in this rule file, not the engine.

The threshold is expressed against the indexer's `skill-body-line-count` metric, which counts the markdown body lines after the closing frontmatter delimiter.

## Look For

- A `SKILL.md` whose `skill-body-line-count` reaches the threshold but has no `## Critical Path` heading.

## Fix

Add a `## Critical Path` section summarising the skill's steps as a short table of contents, or shorten the body below the threshold if the skill does not warrant one.
