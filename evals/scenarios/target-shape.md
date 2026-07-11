---
id: target-shape
owner: scenarios
kind: suite
entrypoint: /spec:plan
stages: [plan, refine]
isolation: fresh-project
assertions:
  - plan-exists
  - spec-reflects-shape-idioms
  - design-reflects-shape-idioms
  - intent-and-doc-fixtures-agree
negative-expectations:
  - live-model-ci-required
  - semantic-byte-golden-required
expected-artifacts:
  - plan.yaml
  - .specify/slices/greeting/spec.md
  - .specify/slices/greeting/design.md
---

# Target shape injection

Scenario ID: `target-shape`

## Intent

Prove that core synthesis folds a non-empty `target.shape` brief into a slice's `spec.md` and `design.md` regardless of source. Two fixtures — one pure-intent, one documentation-sourced — should both pick up the same target-idiom guidance. See the worked fixture in [`evals/fixtures/targets/omnia/`](../fixtures/targets/omnia/README.md).

## Setup

Follow the **single-project setup** in [`shared/setup.md`](../shared/setup.md) with `specify init omnia@1.0.0` (a target whose `shape` brief is non-empty). Prepare two one-slice inputs for slice `greeting`: one driven by pure intent, one by a short docs path describing the same behavior.

## Invocation

1. **Plan + refine (intent fixture)** — `/spec:plan greeting "..."`, stamp Gate 1, `/spec:refine`; inspect `.specify/slices/greeting/spec.md` and `design.md`.
2. **Plan + refine (documentation fixture)** — repeat in a fresh project from a docs path describing the same behavior.
3. Compare the shape-derived sections across both fixtures.

## Assertions

- `plan-exists`: `plan.yaml` exists after `/spec:plan`.
- `spec-reflects-shape-idioms`: `spec.md` reflects the target `shape` idiom guidance.
- `design-reflects-shape-idioms`: `design.md` reflects the target `shape` idiom guidance (provider DI, error conventions, validation placement).
- `intent-and-doc-fixtures-agree`: the intent and documentation fixtures honour the same `shape`-derived sections.

## Negative expectations

Live profiles remain outside per-commit CI, and semantic output is rubric-graded rather than byte-golden tested.

## Recording

Capture with [`shared/run-template.md`](../shared/run-template.md) as [`evals/runs/<id>.<result>.md`](../runs/README.md).
