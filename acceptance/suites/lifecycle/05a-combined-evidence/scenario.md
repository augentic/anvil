---
id: combined-evidence
owner: lifecycle
kind: suite
backend: manual
entrypoint: /spec:plan
stages: [plan, refine]
isolation: fresh-project
assertions:
  - plan-exists
  - serial-extract-per-source
  - two-entry-evidence
  - sources-line-carries-both
  - deterministic-reconciliation
  - lifecycle-reaches-refined
negative-expectations:
  - automated-runner-added
  - fake-forge-added
  - transcript-replay-added
  - ci-target-added
  - golden-output-required
expected-artifacts:
  - plan.yaml
  - .specify/slices/inventory-sync/spec.md
---

# Combined evidence (code + documentation), one slice

Scenario ID: `combined-evidence`

## Intent

Prove synthesis end to end when two agreeing sources are bound on one slice: serial `extract` per source, a two-entry `Evidence[]`, a `Sources:` line carrying both keys, deterministic reconciliation by `id` correlation, and a clean transition to `refined`.

## Setup

Follow the **single-project setup** in [`shared/setup.md`](../../shared/setup.md) with `specify init omnia@v1`. Bind a legacy repo path and a design-notes docs path that describe the same behavior without disagreement. Plan a one-slice change named `inventory-sync` with both sources bound.

## Invocation

1. **Plan** — `/spec:plan inventory-sync` binding both the legacy and design-notes sources; stamp Gate 1.
2. **Refine** — `/spec:refine` (or `/spec:execute` to refine); inspect `.specify/slices/inventory-sync/spec.md` and the evidence directory.

## Assertions

- `plan-exists`: `plan.yaml` exists after `/spec:plan`.
- `serial-extract-per-source`: `extract` runs once per bound source.
- `two-entry-evidence`: the slice carries a two-entry `Evidence[]`.
- `sources-line-carries-both`: the requirement `Sources:` line lists both source keys.
- `deterministic-reconciliation`: `id` correlation produces stable reconciliation across re-runs.
- `lifecycle-reaches-refined`: the slice transitions to `refined` cleanly when the sources agree.

## Negative expectations

Manual by design — see [`docs/contributing/acceptance.md`](../../../../docs/contributing/acceptance.md). No automated runner, fake forge, recorded transcript, CI target, or golden comparison.

## Recording

Capture with [`shared/run-summary-template.md`](../../shared/run-summary-template.md) under [`acceptance/runs/`](../../../runs/README.md).
