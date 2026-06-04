---
id: divergence-authority
owner: lifecycle
kind: suite
backend: manual
entrypoint: /spec:plan
stages: [plan, refine]
isolation: fresh-project
assertions:
  - plan-exists
  - divergence-tag-written
  - documentation-authority-wins
  - behaviour-preserved-as-commentary
  - lifecycle-reaches-refined
negative-expectations:
  - automated-runner-added
  - fake-forge-added
  - transcript-replay-added
  - ci-target-added
  - golden-output-required
expected-artifacts:
  - plan.yaml
  - .specify/slices/token-expiry/spec.md
---

# Divergence from authority resolution

Scenario ID: `divergence-authority`

## Intent

Prove authority-resolved disagreement: when documentation and observed legacy code disagree at different authority classes (e.g. docs say "30 minutes" expiry while code observed 24 hours), synthesis writes `[divergence]`, the higher-authority `documentation` value wins as the operative requirement, and the behaviour value is preserved as inline commentary. The slice still reaches `refined`.

## Setup

Follow the **single-project setup** in [`shared/setup.md`](../../shared/setup.md) with `specify init omnia@v1`. Bind a docs source (`authority: documentation`) and a legacy repo (`authority: behaviour`) that disagree on one value. Plan a one-slice change named `token-expiry`.

## Invocation

1. **Plan** — `/spec:plan token-expiry` binding both sources; stamp Gate 1.
2. **Refine** — `/spec:refine` (or `/spec:execute` to refine); inspect `.specify/slices/token-expiry/spec.md`.

## Assertions

- `plan-exists`: `plan.yaml` exists after `/spec:plan`.
- `divergence-tag-written`: the disagreeing requirement carries an inline `[divergence]` tag.
- `documentation-authority-wins`: the operative requirement value is the documentation value.
- `behaviour-preserved-as-commentary`: the observed-behaviour value survives as inline commentary.
- `lifecycle-reaches-refined`: the slice transitions to `refined`; the operator may hand-edit before build.

## Negative expectations

Manual by design — see [`docs/contributing/acceptance.md`](../../../../docs/contributing/acceptance.md). No automated runner, fake forge, recorded transcript, CI target, or golden comparison.

## Recording

Capture with [`shared/run-summary-template.md`](../../shared/run-summary-template.md) under [`acceptance/runs/`](../../../runs/README.md).
