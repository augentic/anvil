# Scenario #9 -- `/spec:execute` parks on a build failure, operator fixes, resumes

## Source

[`docs/contributing/acceptance.md` §Scenario IDs](../../../../docs/contributing/acceptance.md#scenario-ids), scenario `9`.

> Slice's `cargo test` fails; operator patches the crate; runs `/spec:execute`.

**Stress-tests:** Build-failure stop hint; build resumes from the failed task; loop continues to merge.

## Run-summary

Status: **pending**

Operator: copy the field-set from [`operator/run-summary-template.md`](../operator/run-summary-template.md) into this file, fill every section against the live run, and update the **Status:** line above.
