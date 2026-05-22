# Scenario #11 -- Workspace breakout after build failure in a slot

## Source

[`rfcs/archive/rfc-25-workflow.md` §Acceptance scenarios](../../../../rfcs/archive/rfc-25-workflow.md#acceptance-scenarios), row #11.

> `/spec:execute` parks on `auth-rotate` in `project-a`; operator stays at workspace root and runs `/spec:build`.

**Stress-tests:** Project-routing rule for breakout verbs; active-slice resolution across the workspace/slot boundary; correct `chdir` without operator intervention.

## Run-summary

Status: **pending**

Operator: copy the field-set from [`tests/cross-repo/run-summary-template.md`](../../run-summary-template.md) into this file, fill every section against the live run, and update the **Status:** line above.
