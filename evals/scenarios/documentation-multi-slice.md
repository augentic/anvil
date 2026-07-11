---
id: documentation-multi-slice
owner: scenarios
kind: suite
entrypoint: /spec:plan
stages: [plan]
isolation: fresh-project
assertions:
  - plan-exists
  - plan-validates
  - multiple-slices-proposed
  - cross-cutting-lead-multi-homed
  - propose-edit-reject-loop
  - gate-1-amendment
expected-artifacts:
  - plan.yaml
  - discovery.md
negative-expectations:
  - live-model-ci-required
  - semantic-byte-golden-required
---

# Documentation, multi-slice

Scenario ID: `documentation-multi-slice`

## Intent

Prove the propose / edit / reject loop and the Gate-1 amendment flow: a docs path that maps to several candidates yields multiple proposed slices the operator can amend before approval. The scenario stops at Gate 1.

## Setup

Follow the **single-project setup** in [`shared/setup.md`](../shared/setup.md) with `specify init omnia@1.0.0`. Create a multi-feature brief at `docs/catalog-revamp.md` that clearly describes N (3+) separable behaviors. Add a cross-cutting conventions doc at `docs/conventions.md` (e.g. validation and error-shape rules that apply to every behavior in the brief) bound as a second source, so its survey synopsis marks it cross-cutting and propose can multi-home it.

## Invocation

1. **Plan** — `/spec:plan catalog-revamp source storefront=docs/catalog-revamp.md source conventions=docs/conventions.md`; confirm the propose step yields multiple slices and stops at `pending`.
2. **Review + amend** — inspect `plan.yaml`; use `specify plan amend` to edit or reject at least one proposed slice; re-validate.
3. Stop at Gate 1 (do not stamp `approved`).

## Assertions

- `plan-exists`: `plan.yaml` exists after `/spec:plan`.
- `plan-validates`: `specify plan validate` exits cleanly before and after amendment.
- `multiple-slices-proposed`: the docs path maps to more than one proposed slice.
- `cross-cutting-lead-multi-homed`: the conventions lead appears in the `sources:` of more than one proposed slice (no `depends-on` edge between them implied), and `change.md` lists it under `## Cross-cutting leads`.
- `propose-edit-reject-loop`: an operator edit/reject via `specify plan amend` is reflected in `plan.yaml`.
- `gate-1-amendment`: the plan remains at `pending` after amendment and prints the transition command.

## Negative expectations

Live profiles remain outside per-commit CI, and semantic output is rubric-graded rather than byte-golden tested.

## Recording

Capture with [`shared/run-template.md`](../shared/run-template.md) as [`evals/runs/<id>.<result>.md`](../runs/README.md).
