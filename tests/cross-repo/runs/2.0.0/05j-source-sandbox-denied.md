# Scenario #5j -- Source-adapter sandbox path-denied

## Source

[`rfcs/archive/rfc-25-workflow.md` §Acceptance scenarios](../../../../rfcs/archive/rfc-25-workflow.md#acceptance-scenarios), row #5j.

> A source adapter's `extract` (or `enumerate`) attempts a read outside its bound `$SOURCE_DIR` / `$CAPABILITY_DIR` / `$SCRATCH_DIR` grants.

**Stress-tests:** Host runner denies the access and surfaces structured error `source-extract-path-denied` (or `source-enumerate-path-denied`); slice stays `refining`; no Evidence is written; operator can rebind via `plan amend` or drop the source. WASI preopens are the only grant; lifecycle state (`.specify/project.yaml`, `.metadata.yaml`) is unreachable.

## Run-summary

Status: **pending**

Operator: copy the field-set from [`tests/cross-repo/run-summary-template.md`](../../run-summary-template.md) into this file, fill every section against the live run, and update the **Status:** line above.
