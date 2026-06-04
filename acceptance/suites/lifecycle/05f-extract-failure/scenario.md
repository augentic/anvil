---
id: extract-failure
owner: lifecycle
kind: suite
backend: manual
entrypoint: /spec:plan
stages: [plan, refine]
isolation: fresh-project
assertions:
  - plan-exists
  - slice-stays-refining
  - no-synthesis-runs
  - structured-error-names-source
negative-expectations:
  - automated-runner-added
  - fake-forge-added
  - transcript-replay-added
  - ci-target-added
  - golden-output-required
expected-artifacts:
  - plan.yaml
---

# Extract failure

Scenario ID: `extract-failure`

## Intent

Prove the extract-failure path: when a bound source's `extract` fails, the slice stays in `refining`, no synthesis runs, and a structured error names the failing source key.

## Setup

Follow the **single-project setup** in [`shared/setup.md`](../../shared/setup.md) with `specify init omnia@v1`. Bind a source whose `extract` will fail (e.g. a path that the adapter cannot read, or a deliberately malformed input). Plan a one-slice change named `broken-extract`.

## Invocation

1. **Plan** — `/spec:plan broken-extract` binding the failing source; stamp Gate 1.
2. **Refine** — `/spec:refine` (or `/spec:execute` to refine) and capture the failure.

## Assertions

- `plan-exists`: `plan.yaml` exists after `/spec:plan`.
- `slice-stays-refining`: the slice remains in `refining` after the failed extract.
- `no-synthesis-runs`: no `spec.md` / `design.md` is synthesised.
- `structured-error-names-source`: the structured error identifies the failing source key.

## Negative expectations

Manual by design — see [`docs/contributing/acceptance.md`](../../../../docs/contributing/acceptance.md). No automated runner, fake forge, recorded transcript, CI target, or golden comparison.

## Recording

Capture with [`shared/run-summary-template.md`](../../shared/run-summary-template.md) under [`acceptance/runs/`](../../../runs/README.md).
