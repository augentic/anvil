---
id: intra-evidence-conflict
owner: lifecycle
kind: suite
backend: manual
entrypoint: /spec:plan
stages: [plan, refine, build]
isolation: fresh-project
assertions:
  - plan-exists
  - conflict-tag-written
  - lifecycle-reaches-refined
  - hand-edit-then-build
negative-expectations:
  - automated-runner-added
  - fake-forge-added
  - transcript-replay-added
  - ci-target-added
  - golden-output-required
expected-artifacts:
  - plan.yaml
  - .specify/slices/session-expiry/spec.md
---

# Intra-Evidence conflict

Scenario ID: `intra-evidence-conflict`

## Intent

Prove that when synthesis cannot reconcile contradictory `claims` within a single `Evidence` document, it writes a `[conflict]` tag into `spec.md`, the lifecycle still transitions to `refined` (no parking ceremony), and the operator can hand-edit then run `/spec:build`.

## Setup

Follow the **single-project setup** in [`shared/setup.md`](../../shared/setup.md) with `specify init omnia@v1`. Create one docs source whose claims contradict themselves (e.g. one paragraph says the session expires after 30 minutes, another says 24 hours). Plan a one-slice change named `session-expiry`.

## Invocation

1. **Plan** — `/spec:plan session-expiry source brief=docs/session-expiry.md`; stamp Gate 1.
2. **Refine** — `/spec:refine` (or `/spec:execute` to the refine stage); inspect `.specify/slices/session-expiry/spec.md`.
3. **Hand-edit + build** — reconcile the `[conflict]` requirement by hand, then `/spec:build`.

## Assertions

- `plan-exists`: `plan.yaml` exists after `/spec:plan`.
- `conflict-tag-written`: the contradictory requirement carries an inline `[conflict]` tag in `spec.md` with both values preserved.
- `lifecycle-reaches-refined`: the slice transitions to `refined` despite the conflict.
- `hand-edit-then-build`: after the operator edits the requirement, `/spec:build` proceeds without a parking-state ceremony.

## Negative expectations

Manual by design — see [`docs/contributing/acceptance.md`](../../../../docs/contributing/acceptance.md). No automated runner, fake forge, recorded transcript, CI target, or golden comparison.

## Recording

Capture with [`shared/run-summary-template.md`](../../shared/run-summary-template.md) under [`acceptance/runs/`](../../../runs/README.md).
