---
id: execute-build-failure
owner: lifecycle
kind: suite
backend: manual
entrypoint: /spec:plan
stages: [plan, refine, build, merge]
isolation: fresh-project
assertions:
  - plan-exists
  - build-failure-stop-hint
  - build-resumes-from-failed-task
  - loop-continues-to-merge
negative-expectations:
  - automated-runner-added
  - fake-forge-added
  - transcript-replay-added
  - ci-target-added
  - golden-output-required
expected-artifacts:
  - plan.yaml
---

# /spec:execute parks on a build failure, operator fixes, resumes

Scenario ID: `execute-build-failure`

## Intent

Prove the build-failure recovery path: a slice's `cargo test` fails during `/spec:execute`, the loop parks with a stop hint, the operator patches the crate, and re-running `/spec:execute` resumes from the failed task and continues to merge.

## Setup

Follow the **single-project setup** in [`shared/setup.md`](../../shared/setup.md) with `specify init omnia@v1`. Author a plan named `rate-limit` whose build will fail on first attempt (e.g. a task that needs an operator-supplied fix). Stamp Gate 1.

## Invocation

1. **Execute** — `/spec:execute loop`; confirm it parks on the build failure with a clear stop hint.
2. **Fix** — patch the crate so the failing `cargo test` passes.
3. **Resume** — `/spec:execute loop`; confirm it resumes from the failed task and reaches `all-done` through merge.

## Assertions

- `plan-exists`: `plan.yaml` exists after `/spec:plan`.
- `build-failure-stop-hint`: the loop parks with a structured stop hint naming the failed task/slice.
- `build-resumes-from-failed-task`: after the fix, the build resumes from the failed task rather than restarting the slice.
- `loop-continues-to-merge`: the resumed loop continues through merge to `all-done`.

## Negative expectations

Manual by design — see [`docs/contributing/acceptance.md`](../../../../docs/contributing/acceptance.md). No automated runner, fake forge, recorded transcript, CI target, or golden comparison.

## Recording

Capture with [`shared/run-summary-template.md`](../../shared/run-summary-template.md) under [`acceptance/runs/`](../../../runs/README.md).
