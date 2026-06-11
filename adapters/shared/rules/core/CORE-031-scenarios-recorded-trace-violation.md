---
id: CORE-031
title: Scenarios Recorded Trace Violation
severity: important
trigger: Recorded trace content violates scenario contract.
rule_hints:
  - kind: path-pattern
    value: adapters/shared/rules/core/CORE-031-scenarios-recorded-trace-violation.md
    description: Sentinel path so the whole-tree scenarios tool runs exactly once; the tool walks PROJECT_DIR itself rather than the passed candidate.
  - kind: tool
    value: scenarios
    description: Run the `scenarios` framework checker, which validates every `evals/recorded/**/*.jsonl` trace's first line as a well-formed `recorded-trace-header`.
---

## Rule

Each recorded-trace file under `evals/recorded/` must begin with a single-line JSON `recorded-trace-header` object whose `schemaVersion` is `1` and whose required fields (`kind`, `schemaVersion`, `sourceBackend`, `sourceRunId`, `sourceTimestamp`, `scenarioId`) are present and non-empty. A malformed header makes the trace unreplayable and breaks provenance.

This check is whole-tree and opt-in: it fires nothing until an `evals/recorded/` tree exists. The `scenarios` framework tool reads `PROJECT_DIR`, walks `evals/recorded/`, and validates every `.jsonl` trace header. The rule's `path-pattern` names a single sentinel file so the tool runs exactly once per lint.

## Look For

- A `.jsonl` trace whose first line is empty, not valid JSON, or not a JSON object.
- A first line whose `kind` is not `recorded-trace-header`.
- A header with `schemaVersion` other than `1`, or a missing/empty required field.

## Fix

Make the first line a JSON `recorded-trace-header` object with `schemaVersion: 1` and every required field populated.
