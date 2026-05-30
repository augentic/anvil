# Scenario #5j -- Source-adapter sandbox path-denied

## Source

[`docs/contributing/acceptance.md` §Scenario IDs](../../../../docs/contributing/acceptance.md#scenario-ids), scenario `5j`.

> A source adapter's `survey` or `extract` operation attempts to read or write outside its bound `$SOURCE_DIR` / `$CAPABILITY_DIR` / `$SCRATCH_DIR` grants.

**Stress-tests:** `$PROJECT_DIR` is never a visible preopen to the adapter operation, so lifecycle state (`.specify/project.yaml`, `.metadata.yaml`) is unreachable; WASI preopens are the only grant. An out-of-sandbox access fails closed — for a `tool`-execution adapter the host runner denies the WASI access directly; for an `agent`-execution adapter, Evidence staged outside the granted `$SCRATCH_DIR` is rejected at finalize with structured error `extract-evidence-missing`. Either way the slice stays `refining`, no Evidence is persisted, and the operator can rebind via `plan amend` or drop the source.

## Run-summary

Status: **pending**

Operator: copy the field-set from [`tests/cross-repo/run-summary-template.md`](../../run-summary-template.md) into this file, fill every section against the live run, and update the **Status:** line above.
