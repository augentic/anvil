---
id: typescript-multi-slice
owner: scenarios
kind: suite
entrypoint: /spec:plan
stages: [plan]
isolation: fresh-project
assertions:
  - plan-exists
  - plan-validates
  - multiple-slices-from-code
  - sources-legacy-only
  - no-under-slicing
expected-artifacts:
  - plan.yaml
  - discovery.md
negative-expectations:
  - live-model-ci-required
  - semantic-byte-golden-required
---

# Code, multi-slice

Scenario ID: `typescript-multi-slice`

## Intent

Prove `typescript` survey and the enumerate / repair loop under `/spec:plan`: a bound legacy repo maps to multiple slices with `Sources: [<legacy-key>]` provenance, and the under-slicing failure mode (collapsing distinct behaviors into one slice) is caught. The scenario stops at Gate 1.

## Setup

Follow the **single-project setup** in [`shared/setup.md`](../shared/setup.md) with `specify init omnia@1.0.0`. Bind a small legacy TypeScript service as a `typescript` source (a repo with several distinct handlers/services). Record the exact `specify` binding command in the run summary.

## Invocation

1. **Plan** — `/spec:plan legacy-port source legacy=<path-to-ts-repo>`; confirm the survey enumerates the legacy surface and proposes multiple slices.
2. **Review** — inspect `plan.yaml`; confirm distinct behaviors are not collapsed into a single slice (run the enumerate/repair loop if they are).
3. Stop at Gate 1.

## Assertions

- `plan-exists`: `plan.yaml` exists after `/spec:plan`.
- `plan-validates`: `specify plan validate` exits cleanly.
- `multiple-slices-from-code`: the legacy repo maps to more than one slice.
- `sources-legacy-only`: each slice's provenance is `Sources: [<legacy-key>]`.
- `no-under-slicing`: distinct legacy behaviors are not collapsed into one slice.

## Negative expectations

Live profiles remain outside per-commit CI, and semantic output is rubric-graded rather than byte-golden tested.

## Recording

Capture with [`shared/run-template.md`](../shared/run-template.md) as [`evals/runs/<id>.<result>.md`](../runs/README.md).
