---
id: invalid-evidence
owner: lifecycle
kind: suite
backend: manual
entrypoint: /spec:plan
stages: [plan, refine]
isolation: fresh-project
assertions:
  - plan-exists
  - validation-fails-before-synthesis
  - structured-error
  - slice-stays-refining
negative-expectations:
  - automated-runner-added
  - fake-forge-added
  - transcript-replay-added
  - ci-target-added
  - golden-output-required
expected-artifacts:
  - plan.yaml
---

# Invalid Evidence schema rejection

Scenario ID: `invalid-evidence`

## Intent

Prove the Evidence-validation gate: when an adapter emits `Evidence` that fails `evidence.schema.json`, validation fails before synthesis runs, a structured error is returned, and the slice stays in `refining`.

## Setup

Follow the **single-project setup** in [`shared/setup.md`](../../shared/setup.md) with `specify init omnia@v1`. Bind a source configured to emit schema-invalid Evidence (e.g. a fixture adapter, or staged Evidence missing required fields). Plan a one-slice change named `bad-evidence`.

## Invocation

1. **Plan** — `/spec:plan bad-evidence` binding the source; stamp Gate 1.
2. **Refine** — `/spec:refine` (or `/spec:execute` to refine) and capture the validation failure.

## Assertions

- `plan-exists`: `plan.yaml` exists after `/spec:plan`.
- `validation-fails-before-synthesis`: schema validation rejects the Evidence before any synthesis.
- `structured-error`: the failure surfaces as a structured error, not a panic or silent skip.
- `slice-stays-refining`: the slice remains in `refining`.

## Negative expectations

Manual by design — see [`docs/contributing/acceptance.md`](../../../../docs/contributing/acceptance.md). No automated runner, fake forge, recorded transcript, CI target, or golden comparison.

## Recording

Capture with [`shared/run-summary-template.md`](../../shared/run-summary-template.md) under [`acceptance/runs/`](../../../runs/README.md).
