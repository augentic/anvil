# Scenario #5e -- Cross-source propose-time merge

## Source

[`docs/contributing/acceptance.md` §Scenario IDs](../../../../docs/contributing/acceptance.md#scenario-ids), scenario `5e`.

> Two adapters surface the same candidate; the `/spec:plan` agent merges them automatically at `propose`.

**Stress-tests:** `specify plan propose --from` writes slices with combined `sources:` without operator ceremony; uncertain merges surfaced in `change.md` under `## Tentative merges`; operator overrides via `specify plan amend` at Gate 1 if the merge is wrong; downstream `extract` runs against every contributing source.

## Run-summary

Status: **pending**

Operator: copy the field-set from [`operator/run-summary-template.md`](../operator/run-summary-template.md) into this file, fill every section against the live run, and update the **Status:** line above.
