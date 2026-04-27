---
id: contracts
description: Validate spec alignment with baseline contracts; generate delta for uncovered interactions
generates: contracts/**/*.yaml
needs: [specs]
---

Arguments (used by all skills):
- CHANGE_ID: the name of this change (from specify status)

## Algorithm

### Phase 1: Generate

1. /contracts:writer — Read baseline contracts at `.specify/contracts/` and the change's specs under `specs/`. Validate alignment between specs and baseline. Produce the minimal contract delta for interactions not covered by the baseline.

### Phase 2: Validate

2. /contracts:validator — Verify internal consistency of the produced artifacts: `$ref` resolution, schema metadata, binding completeness.

### Phase 3: Verify-repair loop (max 2 iterations)

If the validator reports failures:
1. Re-enter `/contracts:writer` with the validation output, instructing it to make targeted repairs to the flagged issues only.
2. Re-run `/contracts:validator` to check the repairs.
3. If still failing after 2 repair iterations, stop and surface the remaining issues for human review. Do not mark the brief as complete.

A clean validation pass with zero issues is the expected outcome. When pre-existing baseline contracts cover the change's spec interactions, the writer produces a small or empty delta and the validator confirms consistency — the brief completes quickly.

### No-op behavior

When the change's specs describe no API interactions (no HTTP endpoints, no message exchanges, no data types that would warrant contract artifacts), the writer produces an empty delta and the validator has nothing to check. The brief completes as a no-op. This is normal for changes that modify internal logic without affecting API surfaces.
