---
id: source-sandbox-denied
owner: lifecycle
kind: suite
backend: manual
entrypoint: /spec:plan
stages: [plan, refine]
isolation: fresh-project
assertions:
  - plan-exists
  - out-of-sandbox-access-denied
  - project-dir-not-preopened
  - slice-stays-refining
  - operator-can-rebind-or-drop
negative-expectations:
  - automated-runner-added
  - fake-forge-added
  - transcript-replay-added
  - ci-target-added
  - golden-output-required
expected-artifacts:
  - plan.yaml
---

# Source-adapter sandbox path-denied

Scenario ID: `source-sandbox-denied`

## Intent

Prove the source-adapter sandbox holds: a `survey` or `extract` that attempts to read or write outside its bound `$SOURCE_DIR` / `$CAPABILITY_DIR` / `$SCRATCH_DIR` grants fails closed. `$PROJECT_DIR` is never a visible preopen, so lifecycle state is unreachable. For a `tool`-execution adapter the host runner denies the WASI access directly; for an `agent`-execution adapter, Evidence staged outside the granted `$SCRATCH_DIR` is rejected at finalize with `extract-evidence-missing`. Either way the slice stays `refining`, no Evidence is persisted, and the operator can rebind via `plan amend` or drop the source.

## Setup

Follow the **single-project setup** in [`shared/setup.md`](../../shared/setup.md) with `specify init omnia@v1`. Bind a source adapter configured to attempt an out-of-sandbox access during `survey`/`extract`. Plan a one-slice change named `escape-attempt`.

## Invocation

1. **Plan** — `/spec:plan escape-attempt` binding the offending source; stamp Gate 1.
2. **Refine** — `/spec:refine` (or `/spec:execute` to refine) and capture the denied access.
3. **Recover** — `specify plan amend` to rebind the source, or drop it.

## Assertions

- `plan-exists`: `plan.yaml` exists after `/spec:plan`.
- `out-of-sandbox-access-denied`: the out-of-sandbox read/write fails closed (host WASI denial or `extract-evidence-missing` at finalize).
- `project-dir-not-preopened`: `$PROJECT_DIR` (e.g. `.specify/project.yaml`, `.metadata.yaml`) is unreachable from the adapter operation.
- `slice-stays-refining`: the slice remains in `refining` with no persisted Evidence.
- `operator-can-rebind-or-drop`: the operator can rebind via `plan amend` or drop the source.

## Negative expectations

Manual by design — see [`docs/contributing/acceptance.md`](../../../../docs/contributing/acceptance.md). No automated runner, fake forge, recorded transcript, CI target, or golden comparison.

## Recording

Capture with [`shared/run-summary-template.md`](../../shared/run-summary-template.md) under [`acceptance/runs/`](../../../runs/README.md).
