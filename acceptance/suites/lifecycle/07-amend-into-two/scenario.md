---
id: amend-into-two
owner: lifecycle
kind: suite
backend: manual
entrypoint: /spec:plan
stages: [plan]
isolation: fresh-project
assertions:
  - plan-exists
  - plan-validates
  - amend-splits-into-two
  - dependencies-coherent-after-amend
  - gate-1-reentry
negative-expectations:
  - automated-runner-added
  - fake-forge-added
  - transcript-replay-added
  - ci-target-added
  - golden-output-required
expected-artifacts:
  - plan.yaml
---

# Operator amends a one-slice plan into two at Gate 1

Scenario ID: `amend-into-two`

## Intent

Prove the Gate-1 amendment flow: an operator splits a one-slice plan into two slices via `specify plan amend`, the resulting dependencies stay coherent, and the plan re-enters Gate 1 at `pending` after the amend.

## Setup

Follow the **single-project setup** in [`shared/setup.md`](../../shared/setup.md) with `specify init omnia@v1`. Author a one-slice plan named `profile-page` from a brief that plausibly decomposes into two slices.

## Invocation

1. **Plan** — `/spec:plan profile-page source brief=docs/profile-page.md`; confirm a single slice and `pending`.
2. **Amend** — `specify plan amend` to split the slice into two with a coherent dependency edge; re-validate.
3. Confirm the plan is back at Gate 1 (`pending`) printing the transition command.

## Assertions

- `plan-exists`: `plan.yaml` exists after `/spec:plan`.
- `plan-validates`: `specify plan validate` exits cleanly before and after amend.
- `amend-splits-into-two`: `plan.yaml` holds two slices after the amend.
- `dependencies-coherent-after-amend`: the dependency edge between the two slices is coherent (no cycle, correct order).
- `gate-1-reentry`: the plan remains/returns to `pending` after the amend.

## Negative expectations

Manual by design — see [`docs/contributing/acceptance.md`](../../../../docs/contributing/acceptance.md). No automated runner, fake forge, recorded transcript, CI target, or golden comparison.

## Recording

Capture with [`shared/run-summary-template.md`](../../shared/run-summary-template.md) under [`acceptance/runs/`](../../../runs/README.md).
