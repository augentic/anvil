# Scenario #8 -- Step-through breakout mid-execute

## Source

[`rfcs/archive/rfc-25-workflow.md` §Acceptance scenarios](../../../../rfcs/archive/rfc-25-workflow.md#acceptance-scenarios), row #8.

> Operator starts `/spec:execute`; on the second slice they cancel, run `/spec:build` directly to investigate, then re-invoke `/spec:execute`.

**Stress-tests:** Stop/resume contract; step-through verbs leave on-disk state consistent for `/spec:execute` to resume without flags.

## Run-summary

Status: **pending**

Operator: copy the field-set from [`tests/cross-repo/run-summary-template.md`](../../run-summary-template.md) into this file, fill every section against the live run, and update the **Status:** line above.
