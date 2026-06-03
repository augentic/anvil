# Scenario #11 -- Workspace breakout after build failure in a slot

## Source

[`docs/contributing/acceptance.md` §Scenario IDs](../../../../docs/contributing/acceptance.md#scenario-ids), scenario `11`.

> `/spec:execute` parks on `auth-rotate` in `project-a`; operator stays at workspace and runs `/spec:build`.

**Stress-tests:** Project-routing rule for breakout verbs; active-slice resolution across the workspace/slot boundary; correct `chdir` without operator intervention.

## Run-summary

Status: **pending**

Operator: copy the field-set from [`tests/cross-repo/run-summary-template.md`](../../run-summary-template.md) into this file, fill every section against the live run, and update the **Status:** line above.
