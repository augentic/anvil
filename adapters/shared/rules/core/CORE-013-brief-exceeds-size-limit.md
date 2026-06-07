---
id: CORE-013
title: Brief Exceeds Size Limit
severity: important
trigger: An adapter brief body exceeds the configured line budget.
rule_hints:
  - kind: cardinality
    value: brief-parent-body-line-count
    description: For each parent orchestrator brief (`briefs/<op>.md`), assert that its non-blank body line count is at most `config.max`. One finding per over-budget parent brief.
    config:
      max: 150
  - kind: cardinality
    value: brief-phase-body-line-count
    description: For each phase sub-brief (`briefs/{build,extract}/**/*.md`), assert that its non-blank body line count is at most `config.max`. One finding per over-budget phase sub-brief.
    config:
      max: 800
---

## Rule

Adapter briefs stay within their line budget. Parent orchestrator briefs (`adapters/<axis>/<adapter>/briefs/<op>.md`) coordinate phases and stay terse — at most 150 non-blank body lines; operational depth belongs in a phase sub-brief or a `references/` document. Phase sub-briefs (`adapters/<axis>/<adapter>/briefs/{build,extract}/**/*.md`) carry the operational detail but still cap at 800 non-blank body lines; past that, split into sub-phases or move worked examples into `references/`.

The deterministic-hint interpreter consumes the `Brief` facts the framework indexer already produced, each carrying a `parent` / `phase` scope discriminator and a non-blank `body-line-count`. The two caps are policy carried in the rule's `config:`, not the engine; there is no advisory soft-cap finding.

## Look For

- A parent brief that absorbed step-by-step operational detail instead of delegating to a phase sub-brief.
- A phase sub-brief that grew past the hard cap by inlining large worked examples or templates that belong under `references/`.

## Fix

Move operational depth out of an over-budget parent brief into a phase sub-brief under `briefs/<phase>/` or into `references/`; split an over-budget phase sub-brief into sub-phases or relocate its worked examples and templates into `plugins/<name>/references/`.
