---
id: stale-workspace-recovery
owner: scenarios
kind: suite
backend: manual
entrypoint: /spec:plan
stages: [plan, refine, build, merge]
isolation: fresh-project
assertions:
  - plan-exists
  - dirty-slot-detected-at-sync
  - slice-state-preserved
  - resume-continues-from-in-progress
  - execute-loop-all-done
negative-expectations:
  - automated-runner-added
  - fake-forge-added
  - transcript-replay-added
  - ci-target-added
  - golden-output-required
expected-artifacts:
  - plan.yaml
  - .specify/workspace
---

# Stale-workspace recovery

Scenario ID: `stale-workspace-recovery`

## Intent

Prove re-entry after an interrupted execute: the operator interrupts `/spec:execute` mid-run leaving a workspace slot dirty with uncommitted work; a fresh `specify workspace sync` plus resume reconciles cleanly without losing slice state, continuing from the in-progress entry rather than restarting or dropping work.

## Setup

Follow the **cross-repo workspace setup** in [`shared/setup.md`](../shared/setup.md) and the **OAuth login brief**. Author and approve a multi-slice plan.

## Invocation

1. **Execute + interrupt** — `/spec:execute loop`; interrupt mid-run, leaving a slot dirty with uncommitted work.
2. **Resync** — `specify workspace sync`; confirm it detects the dirty slot.
3. **Resume** — `/spec:execute loop`; confirm it continues from the in-progress entry and reaches `all-done`.

## Assertions

- `plan-exists`: `plan.yaml` exists and is approved before execute.
- `dirty-slot-detected-at-sync`: `specify workspace sync` detects the dirty/uncommitted slot.
- `slice-state-preserved`: slice state survives the interruption (no lost or duplicated work).
- `resume-continues-from-in-progress`: resume continues from the in-progress entry, not a restart.
- `execute-loop-all-done`: the resumed loop reaches `all-done`.

## Negative expectations

Manual by design — see [`docs/contributing/acceptance.md`](../../docs/contributing/acceptance.md). No automated runner, fake forge, recorded transcript, CI target, or golden comparison.

## Recording

Capture with [`shared/run-summary-template.md`](../shared/run-summary-template.md) under [`acceptance/runs/`](../runs/README.md).
