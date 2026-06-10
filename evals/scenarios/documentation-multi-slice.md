---
id: documentation-multi-slice
owner: scenarios
kind: suite
backend: manual
entrypoint: /spec:plan
stages: [plan]
isolation: fresh-project
assertions:
  - plan-exists
  - plan-validates
  - multiple-slices-proposed
  - propose-edit-reject-loop
  - gate-1-amendment
expected-artifacts:
  - plan.yaml
  - discovery.md
negative-expectations:
  - automated-runner-added
  - fake-forge-added
  - transcript-replay-added
  - ci-target-added
  - golden-output-required
---

# Documentation, multi-slice

Scenario ID: `documentation-multi-slice`

## Intent

Prove the propose / edit / reject loop and the Gate-1 amendment flow: a docs path that maps to several candidates yields multiple proposed slices the operator can amend before approval. The scenario stops at Gate 1.

## Setup

Follow the **single-project setup** in [`shared/setup.md`](../shared/setup.md) with `specify init omnia@v1`. Create a multi-feature brief at `docs/catalog-revamp.md` that clearly describes N (3+) separable behaviors.

## Invocation

1. **Plan** — `/spec:plan catalog-revamp source brief=docs/catalog-revamp.md`; confirm the propose step yields multiple slices and stops at `pending`.
2. **Review + amend** — inspect `plan.yaml`; use `specify plan amend` to edit or reject at least one proposed slice; re-validate.
3. Stop at Gate 1 (do not stamp `approved`).

## Assertions

- `plan-exists`: `plan.yaml` exists after `/spec:plan`.
- `plan-validates`: `specify plan validate` exits cleanly before and after amendment.
- `multiple-slices-proposed`: the docs path maps to more than one proposed slice.
- `propose-edit-reject-loop`: an operator edit/reject via `specify plan amend` is reflected in `plan.yaml`.
- `gate-1-amendment`: the plan remains at `pending` after amendment and prints the transition command.

## Negative expectations

Manual by design — see [`docs/contributing/acceptance.md`](../../docs/contributing/acceptance.md). No automated runner, fake forge, recorded transcript, CI target, or golden comparison.

## Recording

Capture with [`shared/run-template.md`](../shared/run-template.md) as [`acceptance/runs/<id>.<result>.md`](../runs/README.md).
