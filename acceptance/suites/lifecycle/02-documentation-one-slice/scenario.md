---
id: documentation-one-slice
owner: lifecycle
kind: suite
backend: manual
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
  - .specify/plans/feature-doc/discovery.md
negative-expectations:
  - automated-runner-added
  - fake-forge-added
  - transcript-replay-added
  - ci-target-added
  - golden-output-required
---

# Documentation, one slice

Scenario ID: `documentation-one-slice`

## Intent

Prove `documentation` survey correctness at the new entry point: a single bound docs path produces one coherent slice with `Sources: [<doc-key>]` provenance, then executes end to end.

## Setup

Follow the **single-project setup** in [`shared/setup.md`](../../shared/setup.md) with `specify init omnia@v1`. Create a short single-feature brief at `docs/feature-doc.md` describing one self-contained behavior (e.g. a single validated endpoint).

## Invocation

1. **Plan** — `/spec:plan feature-doc source brief=docs/feature-doc.md`; confirm it produces one slice and stops at `pending`.
2. **Stamp Gate 1** — `specify plan transition feature-doc approved`.
3. **Execute** — `/spec:execute`; confirm `all-done`.

## Assertions

- `plan-exists`: `plan.yaml` exists after `/spec:plan`.
- `plan-validates`: `specify plan validate` exits cleanly.
- `single-slice-from-doc`: the single docs path maps to exactly one slice.
- `sources-documentation-only`: the slice's provenance is `Sources: [<doc-key>]` and nothing else.
- `execute-loop-all-done`: `/spec:execute` reaches `all-done`.

## Negative expectations

Manual by design — see [`docs/contributing/acceptance.md`](../../../../docs/contributing/acceptance.md). No automated runner, fake forge, recorded transcript, CI target, or golden comparison.

## Recording

Capture with [`shared/run-summary-template.md`](../../shared/run-summary-template.md) under [`acceptance/runs/`](../../../runs/README.md).
