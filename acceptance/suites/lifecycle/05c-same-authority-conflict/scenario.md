---
id: same-authority-conflict
owner: lifecycle
kind: suite
backend: manual
entrypoint: /spec:plan
stages: [plan, refine]
isolation: fresh-project
assertions:
  - plan-exists
  - conflict-tag-written
  - both-values-preserved
  - lifecycle-reaches-refined
  - operator-must-reconcile
negative-expectations:
  - automated-runner-added
  - fake-forge-added
  - transcript-replay-added
  - ci-target-added
  - golden-output-required
expected-artifacts:
  - plan.yaml
  - .specify/slices/retry-policy/spec.md
---

# Conflict from same-authority disagreement

Scenario ID: `same-authority-conflict`

## Intent

Prove that when two sources of the *same* authority class (two `documentation` sources) disagree on one claim, synthesis writes `[conflict]` with both values preserved as inline commentary, the lifecycle still transitions to `refined`, and the operator must reconcile (by editing or amending sources) before the requirement is meaningful.

## Setup

Follow the **single-project setup** in [`shared/setup.md`](../../shared/setup.md) with `specify init omnia@v1`. Bind two `documentation` sources that disagree on the same claim. Plan a one-slice change named `retry-policy`.

## Invocation

1. **Plan** — `/spec:plan retry-policy` binding both documentation sources; stamp Gate 1.
2. **Refine** — `/spec:refine` (or `/spec:execute` to refine); inspect `.specify/slices/retry-policy/spec.md`.

## Assertions

- `plan-exists`: `plan.yaml` exists after `/spec:plan`.
- `conflict-tag-written`: the disagreeing requirement carries `[conflict]`.
- `both-values-preserved`: both source values survive as inline commentary.
- `lifecycle-reaches-refined`: the slice transitions to `refined` despite the conflict.
- `operator-must-reconcile`: the requirement is not operative until the operator edits or amends a source.

## Negative expectations

Manual by design — see [`docs/contributing/acceptance.md`](../../../../docs/contributing/acceptance.md). No automated runner, fake forge, recorded transcript, CI target, or golden comparison.

## Recording

Capture with [`shared/run-summary-template.md`](../../shared/run-summary-template.md) under [`acceptance/runs/`](../../../runs/README.md).
