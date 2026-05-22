# Scenario #10 -- Workspace `/spec:execute` across two projects

## Source

[`rfcs/rfc-25-workflow.md` §Acceptance scenarios](../../../../rfcs/rfc-25-workflow.md#acceptance-scenarios), row #10.

> Plan with slices targeting `project-a` and `project-b`; operator runs `/spec:execute` from the workspace root.

**Stress-tests:** Per-slice project routing; slot materialisation; `prepare-branch`; `chdir` + residue commit; plan-lock semantics at the workspace root while phase work runs in slots.

## Run-summary

Status: **pending**

Operator: copy the field-set from [`tests/cross-repo/run-summary-template.md`](../../run-summary-template.md) into this file, fill every section against the live run, and update the **Status:** line above.
