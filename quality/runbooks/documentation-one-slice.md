---
id: documentation-one-slice
owner: scenarios
kind: suite
entrypoint: /spec:plan
stages: [plan, refine, build, merge]
isolation: fresh-project
assertions:
  - plan-exists
  - plan-validates
  - single-slice-from-doc
  - sources-documentation-only
  - execute-loop-all-done
expected-artifacts:
  - plan.yaml
  - discovery.md
negative-expectations:
  - live-model-ci-required
  - semantic-byte-golden-required
---

# Documentation, one slice

Scenario ID: `documentation-one-slice`

## Intent

Prove `documentation` survey correctness at the new entry point: a single bound docs path produces one coherent slice with `Sources: [<doc-key>]` provenance, then executes end to end.

## Setup

Follow the **single-project setup** in [`shared/setup.md`](../shared/setup.md) with `specify init omnia@1.0.0`. Create a short single-feature brief at `docs/feature-doc.md` describing one self-contained behavior (e.g. a single validated endpoint).

## Invocation

1. **Plan** — `/spec:plan feature-doc source feature=docs/feature-doc.md`; confirm it produces one slice and stops at `pending`.
2. **Stamp Gate 1** — `specify plan transition feature-doc approved`.
3. **Execute** — `specify plan execute`; confirm drained.

## Assertions

- `plan-exists`: `plan.yaml` exists after `/spec:plan`.
- `plan-validates`: `specify plan validate` exits cleanly.
- `single-slice-from-doc`: the single docs path maps to exactly one slice.
- `sources-documentation-only`: the slice's provenance is `Sources: [<doc-key>]` and nothing else.
- `execute-loop-all-done`: `specify plan execute` reaches drained.

## Negative expectations

Live profiles remain outside per-commit CI, and semantic output is rubric-graded rather than byte-golden tested.

## Recording

Capture with [`shared/run-template.md`](../shared/run-template.md) as [`evals/runs/<id>.<result>.md`](../runs/README.md).
