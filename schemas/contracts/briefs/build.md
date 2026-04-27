---
id: build
description: Validate contract artifacts for structural correctness
needs: [specs, contracts, tasks]
tracks: tasks
---

The build phase for contract-only changes validates structural correctness of the contract artifacts. There is no code generation — the sole output is a validation pass/fail.

Arguments:
- CHANGE_ID: the name of this change (from specify status)

## Validation

1. `/contracts:validator` — Run the full validation suite:
   - `$ref` resolution: all `$ref` pointers in OpenAPI and AsyncAPI files resolve
   - Schema metadata: every JSON Schema file has `$id`, `title`, `description`
   - Binding completeness: every spec-referenced schema has at least one protocol binding

## Verify-repair loop (max 2 iterations)

If the validator reports failures:
1. Re-enter `/contracts:writer` with the validation output for targeted repair
2. Re-run `/contracts:validator`
3. If still failing after 2 iterations, stop and surface issues for human review. Do not mark the task complete. Report the remaining failures with full output and escalate for guidance.

A clean validation pass with zero issues is the expected outcome.
