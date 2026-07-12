---
id: lead-reconciliation
owner: scenarios
kind: suite
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
  - live-model-ci-required
  - semantic-byte-golden-required
expected-artifacts:
  - plan.yaml
  - change.md
---

# Cross-source propose-time merge

Scenario ID: `lead-reconciliation`

## Intent

Prove the `/spec:plan` reconciliation step merges leads automatically when two adapters surface the same candidate: `specify plan author` writes a slice with combined `sources:` without operator ceremony, uncertain merges surface under `## Tentative merges` in `change.md`, the operator can override a wrong merge via `specify plan amend` at Gate 1, and downstream `extract` runs against every contributing source.

## Setup

Follow the **single-project setup** in [`reference/setup.md`](../reference/setup.md) with `specify init omnia@1.0.0`. Bind two sources (e.g. a docs path and a legacy repo) that describe the same candidate behavior. Plan a change named `account-lockout`.

## Invocation

1. **Plan** — `/spec:plan account-lockout` binding both sources; confirm the reconciliation step writes a slice with combined `sources:` and surfaces any uncertain merge under `## Tentative merges` in `change.md`.
2. **Review** — inspect `plan.yaml` and `change.md`; if the merge is wrong, override with `specify plan amend`.
3. **Refine** — `/spec:refine` (or `specify plan execute` to refine); confirm `extract` runs against every contributing source.

## Assertions

- `plan-exists`: `plan.yaml` exists after `/spec:plan`.
- `plan-validates`: `specify plan validate` exits cleanly.
- `merged-slice-combines-sources`: the merged slice's `sources:` lists both contributing keys.
- `tentative-merge-surfaced`: any uncertain merge appears under `## Tentative merges` in `change.md`.
- `amend-overrides-merge`: `specify plan amend` can split or rebind a wrong merge at Gate 1.
- `extract-runs-per-contributing-source`: `extract` runs once per contributing source downstream.

## Negative expectations

Live profiles remain outside per-commit CI, and semantic output is rubric-graded rather than byte-golden tested.

## Recording

Capture with [`reference/run-template.md`](../reference/run-template.md) as [`quality/runs/archive/<id>.<result>.md`](../runs/README.md).
