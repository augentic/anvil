---
id: CORE-005
title: Skill Body Line Count
severity: important
trigger: A `plugins/<plugin>/skills/<skill>/SKILL.md` body exceeds the 200-line cap pinned by [docs/standards/skill-authoring.md](../../../../docs/standards/skill-authoring.md), crowding out the operator's request and every other skill body that fires after it.
rule_hints:
  - kind: path-pattern
    value: "plugins/**/SKILL.md"
    description: Narrow the candidate set to plugin skill manifests before the body-size check fires.
  - kind: cardinality
    value: skill-body-line-count
    config:
      max: 200
    description: For each `Skill` fact in the candidate set, assert that `body_line_count` is at most the `config.max` cap. One finding per over-budget skill, with the `(actual, max)` pair surfaced as structured evidence.
---

## Rule

Every `SKILL.md` body under `plugins/<plugin>/skills/<skill>/` stays within the 200-line cap. Skill bodies load into context the moment the skill triggers; a 1,200-line skill crowds out the operator's request, the artifacts under inspection, and every other skill body that fires later in the same turn. 200 specifically leaves room for the algorithm spine, the Critical Path, and a moderate amount of inline prose — but not enough to absorb every example, every flag re-documentation, and every edge case forever. Overflow is the cue to relocate prose to `references/<topic>.md` and link from the SKILL.md body, not to raise the cap.

The path scope covers only well-formed `plugins/<plugin>/skills/<skill>/SKILL.md` paths. Files that the framework-profile indexer drops upstream (non-skill markdown, malformed frontmatter, missing `name:`) never reach the cardinality check.

The deterministic-hint interpreter consumes the `Skill` facts the framework indexer already produced (`crates/standards/src/lint/index/skill.rs::extract`, whose `body_line_count` field counts non-frontmatter body lines verbatim), so the rule cost is one bound check per candidate skill at lint time. The `value` selects the `skill-body-line-count` metric; the 200-line cap is policy carried in the rule's `config: { max }`, never a `const` in the engine arm.

## Look For

- A SKILL.md that grew past 200 lines because every flag, every error path, and every edge case was inlined directly in the body instead of linked from a `references/<topic>.md` sibling.
- A copy-paste authoring slip that duplicated the Critical Path as a flat list AND a parallel `## Steps` restatement, doubling the body's footprint without adding new behaviour.
- A migration that inlined an entire reference document into the SKILL.md body during a refactor and forgot to land the matching deletion in `references/`.

## Fix

Move the long-form prose into `references/<topic>.md` and replace the inline material in the SKILL.md body with a one-line link to the reference. The 200-line cap is a floor, not a budget — see [`docs/standards/skill-authoring.md` "References discipline"](../../../../docs/standards/skill-authoring.md) for the canonical relocate-to-`references/` pattern and which sections (`Critical Path`, the invocation surface, the dispatch table, the canonical decision points) stay in the SKILL.md body.
