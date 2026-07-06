---
id: CORE-032
title: Scenarios Schema Violation
severity: important
trigger: Scenario frontmatter fails scenario.schema.json.
rule_hints:
  - kind: path-pattern
    value: codex/rules/core/CORE-032-scenarios-schema-violation.md
    description: Sentinel include so the rule carries a candidate set; the `scenario` schema selector validates the whole scenario fact family and ignores the candidate set.
  - kind: schema
    value: scenario
    description: Validate every discovered scenario's frontmatter against the registered `scenario` schema, whole-tree, emitting one finding per schema error.
---

## Rule

Every scenario file's YAML frontmatter must satisfy the scenario schema (`scenario.schema.json`): valid YAML, the required fields, and the declared field shapes. The scenario files live partly under the un-indexed `evals/` tree, so the lint indexer runs a dedicated scenario discovery pass that walks the scenario roots itself and emits a `scenario` fact family carrying each file's parsed frontmatter.

This check is whole-tree: the `kind: schema` hint with `value: scenario` validates every discovered scenario's frontmatter against the registered scenario schema, emitting one finding per schema error. The rule's `path-pattern` is a sentinel include; the scenario schema selector evaluates the whole fact family regardless of the candidate set.

## Look For

- Frontmatter that is not valid YAML.
- Missing required fields or values that violate the scenario schema's shapes.

## Fix

Correct the scenario frontmatter to satisfy `scenario.schema.json`; the finding message names the failing field and constraint.
