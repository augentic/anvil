---
id: execute-fail-resume
owner: scenarios
kind: suite
entrypoint: /spec:plan
stages: [plan, refine, build, merge]
isolation: fresh-project
assertions:
  - plan-exists
  - build-failure-stop-hint
  - build-resumes-from-failed-task
  - loop-continues-to-merge
negative-expectations:
  - live-model-ci-required
  - semantic-byte-golden-required
expected-artifacts:
  - plan.yaml
---

# Plan execute parks on a build failure, operator fixes, resumes

Scenario ID: `execute-fail-resume`

## Intent

Prove the build-failure recovery path: a slice's `cargo test` fails during `specify plan execute`, the loop parks with a stop hint, the operator patches the crate, and re-running `specify plan execute` resumes from the failed task and continues to merge.

## Setup

Follow the **single-project setup** in [`reference/setup.md`](../reference/setup.md) with `specify init omnia@1.0.0`. Author a plan named `rate-limit` whose build will fail on first attempt (e.g. a task that needs an operator-supplied fix). Stamp Gate 1.

## Invocation

1. **Execute** — `specify plan execute`; confirm it parks on the build failure with a clear stop hint.
2. **Fix** — patch the crate so the failing `cargo test` passes.
3. **Resume** — `specify plan execute`; confirm it resumes from the failed task and reaches drained through merge.

## Assertions

- `plan-exists`: `plan.yaml` exists after `/spec:plan`.
- `build-failure-stop-hint`: the loop parks with a structured stop hint naming the failed task/slice.
- `build-resumes-from-failed-task`: after the fix, the build resumes from the failed task rather than restarting the slice.
- `loop-continues-to-merge`: the resumed loop continues through merge to drained.

## Negative expectations

Live profiles remain outside per-commit CI, and semantic output is rubric-graded rather than byte-golden tested.

## Recording

Capture with [`reference/run-template.md`](../reference/run-template.md) as [`quality/runs/archive/<id>.<result>.md`](../runs/README.md).
