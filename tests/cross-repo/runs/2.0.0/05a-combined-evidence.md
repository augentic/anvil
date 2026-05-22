# Scenario #5a -- Combined evidence (code + documentation), one slice

## Source

[`rfcs/archive/rfc-25-workflow.md` §Acceptance scenarios](../../../../rfcs/archive/rfc-25-workflow.md#acceptance-scenarios), row #5a.

> Operator binds a legacy repo and a design-notes path on the same slice.

**Stress-tests:** Synthesis end-to-end: serial `extract` per source; two-entry `Evidence[]`; `Sources:` line carrying both keys; `claim-id` correlation produces deterministic fusion; lifecycle reaches `refined` cleanly when sources agree.

## Run-summary

Status: **pending**

Operator: copy the field-set from [`tests/cross-repo/run-summary-template.md`](../../run-summary-template.md) into this file, fill every section against the live run, and update the **Status:** line above.
