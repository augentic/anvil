# Scenario #13 -- Stale-workspace recovery

## Source

[`docs/contributing/acceptance.md` §Scenario IDs](../../../../docs/contributing/acceptance.md#scenario-ids), scenario `13`.

> Operator interrupts `/spec:execute` mid-run; a workspace slot is left dirty with uncommitted work; a fresh `specify workspace sync` + resume must reconcile cleanly without losing slice state.

**Stress-tests:** Re-entry after an interrupted execute; dirty-slot detection at `specify workspace sync`; slice-state preservation across the interruption; resume continues from the in-progress entry rather than restarting or dropping work.

## Run-summary

Status: **pending**

Operator: copy the field-set from [`operator/run-summary-template.md`](../operator/run-summary-template.md) into this file, fill every section against the live run, and update the **Status:** line above.
