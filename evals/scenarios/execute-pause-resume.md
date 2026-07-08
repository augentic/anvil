---
id: execute-pause-resume
owner: scenarios
kind: suite
entrypoint: /spec:plan
stages: [plan, refine, build, merge]
isolation: fresh-project
assertions:
  - plan-exists
  - breakout-state-consistent
  - execute-resumes-without-flags
  - execute-loop-all-done
negative-expectations:
  - automated-runner-added
  - fake-forge-added
  - transcript-replay-added
  - ci-target-added
  - golden-output-required
expected-artifacts:
  - plan.yaml
---

# Step-through breakout mid-execute

Scenario ID: `execute-pause-resume`

## Intent

Prove the stop/resume contract: an operator starts `specify plan execute`, cancels on the second slice, runs `/spec:build` directly to investigate, then re-invokes `specify plan execute` — which resumes without flags because the step-through verbs left on-disk state consistent.

## Setup

Follow the **single-project setup** in [`shared/setup.md`](../shared/setup.md) with `specify init omnia@1.0.0`. Author a multi-slice plan named `dashboard` (at least two slices) and stamp Gate 1.

## Invocation

1. **Execute** — `specify plan execute`; cancel during the second slice.
2. **Breakout** — `/spec:build` directly on the active slice to investigate.
3. **Resume** — re-invoke `specify plan execute`; confirm it resumes from the right entry without extra flags and reaches drained.

## Assertions

- `plan-exists`: `plan.yaml` exists after `/spec:plan`.
- `breakout-state-consistent`: after cancel + breakout `/spec:build`, on-disk slice/plan state is consistent.
- `execute-resumes-without-flags`: re-invoking `specify plan execute` resumes from the in-progress entry with no flags.
- `execute-loop-all-done`: the resumed loop reaches drained.

## Negative expectations

Manual by design — see [`docs/contributing/evals.md`](../../docs/contributing/evals.md). No automated runner, fake forge, recorded transcript, CI target, or golden comparison.

## Recording

Capture with [`shared/run-template.md`](../shared/run-template.md) as [`evals/runs/<id>.<result>.md`](../runs/README.md).
