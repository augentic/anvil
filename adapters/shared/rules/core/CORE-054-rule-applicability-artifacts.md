---
id: CORE-054
title: Core Rule Applicability Artifacts Footgun
severity: important
trigger: A framework `CORE-*` rule declares a populated `applicability.artifacts` set, which the framework-profile resolver silently drops before any hint runs.
rule_hints:
  - kind: path-pattern
    value: "adapters/shared/rules/core/CORE-*.md"
  - kind: regex
    value: "^\\s*artifacts:\\s*"
    config:
      suffix-must-not-start-with: "[]"
    description: "Flag a populated `applicability.artifacts` key in a core rule; admit only the degenerate empty `artifacts: []` form."
---

## Rule

Framework `CORE-*` rules must scope themselves with `kind: path-pattern` hints, never `applicability.artifacts`. The framework-profile resolver passes `include_unmatched: false` into `artifact_dimension_matches`, so any rule that declares a populated `applicability.artifacts` set is dropped from the resolved output *before any hint runs* — turning the rule into a silent no-op. A populated artifacts block therefore neither scopes nor guards anything; it only hides the rule. This guard catches the footgun at author time, before a dead rule ships.

## Look For

- An `applicability:` block whose `artifacts:` list is non-empty (block list or inline `[doc]`) in any `adapters/shared/rules/core/CORE-*.md` file.
- A core rule that resolves but never fires on a known-bad fixture — the usual symptom of the silent drop.

## Fix

Delete the `applicability.artifacts` block and narrow the candidate file set with `kind: path-pattern` hints instead (see [`CORE-001-adapter-schema.md`](CORE-001-adapter-schema.md) for the worked example and [`README.md`](README.md#applicability-tokens) for the chassis-quirk rationale). The degenerate empty `artifacts: []` form is permitted because it filters nothing.
