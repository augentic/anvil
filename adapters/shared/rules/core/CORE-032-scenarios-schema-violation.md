---
id: CORE-032
title: Scenarios Schema Violation
severity: important
trigger: Scenario frontmatter fails scenario.schema.json.
rule_hints:
  - kind: path-pattern
    value: adapters/shared/rules/core/CORE-032-scenarios-schema-violation.md
    description: Sentinel path so the whole-tree scenarios tool runs exactly once; the tool walks PROJECT_DIR itself rather than the passed candidate.
  - kind: tool
    value: scenarios
    description: Run the `scenarios` framework checker, which discovers scenario packs under PROJECT_DIR and validates each one's frontmatter against the scenario schema.
---

## Rule

Every scenario file's YAML frontmatter must satisfy the scenario schema (`scenario.schema.json`): valid YAML, the required fields, and the declared field shapes. The scenario files live under the un-indexed `acceptance/` tree, so the framework tool performs its own filesystem discovery and embeds its own copy of the schema.

This check is whole-tree: the `scenarios` framework tool discovers every scenario file under the acceptance scenario pack, target adapter tests, and plugin skill fixtures, then validates each one's frontmatter against the embedded scenario schema, emitting one finding per schema error. The rule's `path-pattern` names a single sentinel file so the tool runs exactly once per lint; the tool reads `PROJECT_DIR` and walks the tree itself.

## Look For

- Frontmatter that is not valid YAML.
- Missing required fields or values that violate the scenario schema's shapes.

## Fix

Correct the scenario frontmatter to satisfy `scenario.schema.json`; the finding message names the failing field and constraint.
