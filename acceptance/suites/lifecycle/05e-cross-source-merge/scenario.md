---
id: cross-source-merge
owner: lifecycle
kind: suite
backend: manual
entrypoint: /spec:plan
stages: [plan, refine]
isolation: fresh-project
assertions:
  - plan-exists
  - plan-validates
  - merged-slice-combines-sources
  - tentative-merge-surfaced
  - amend-overrides-merge
  - extract-runs-per-contributing-source
negative-expectations:
  - automated-runner-added
  - fake-forge-added
  - transcript-replay-added
  - ci-target-added
  - golden-output-required
expected-artifacts:
  - plan.yaml
  - change.md
---

# Cross-source propose-time merge

Scenario ID: `cross-source-merge`

## Intent

Prove the `/spec:plan` propose step merges leads automatically when two adapters surface the same candidate: `specify plan propose --from` writes a slice with combined `sources:` without operator ceremony, uncertain merges surface under `## Tentative merges` in `change.md`, the operator can override a wrong merge via `specify plan amend` at Gate 1, and downstream `extract` runs against every contributing source.

## Setup

Follow the **single-project setup** in [`shared/setup.md`](../../shared/setup.md) with `specify init omnia@v1`. Bind two sources (e.g. a docs path and a legacy repo) that describe the same candidate behavior. Plan a change named `account-lockout`.

## Invocation

1. **Plan** — `/spec:plan account-lockout` binding both sources; confirm the propose step writes a slice with combined `sources:` and surfaces any uncertain merge under `## Tentative merges` in `change.md`.
2. **Review** — inspect `plan.yaml` and `change.md`; if the merge is wrong, override with `specify plan amend`.
3. **Refine** — `/spec:refine` (or `/spec:execute` to refine); confirm `extract` runs against every contributing source.

## Assertions

- `plan-exists`: `plan.yaml` exists after `/spec:plan`.
- `plan-validates`: `specify plan validate` exits cleanly.
- `merged-slice-combines-sources`: the merged slice's `sources:` lists both contributing keys.
- `tentative-merge-surfaced`: any uncertain merge appears under `## Tentative merges` in `change.md`.
- `amend-overrides-merge`: `specify plan amend` can split or rebind a wrong merge at Gate 1.
- `extract-runs-per-contributing-source`: `extract` runs once per contributing source downstream.

## Negative expectations

Manual by design — see [`docs/contributing/acceptance.md`](../../../../docs/contributing/acceptance.md). No automated runner, fake forge, recorded transcript, CI target, or golden comparison.

## Recording

Capture with [`shared/run-summary-template.md`](../../shared/run-summary-template.md) under [`acceptance/runs/`](../../../runs/README.md).
