# Scenario #5e -- Cross-source propose-time merge

## Source

[`rfcs/done/rfc-25-workflow.md` §Acceptance scenarios](../../../../rfcs/done/rfc-25-workflow.md#acceptance-scenarios), row #5e.

> Two adapters surface the same candidate; the `/spec:plan` agent merges them automatically at `propose`.

**Stress-tests:** `specrun plan add` writes one slice with combined `sources:` without operator ceremony; uncertain merges annotated `tentative: true` and surfaced in `change.md`; operator overrides via `specrun plan amend` at Gate 1 if the merge is wrong; downstream `extract` runs against every contributing source.

## Run-summary

Status: **pending**

Operator: copy the field-set from [`tests/cross-repo/run-summary-template.md`](../../run-summary-template.md) into this file, fill every section against the live run, and update the **Status:** line above.
