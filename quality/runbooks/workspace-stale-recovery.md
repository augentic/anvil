---
id: workspace-stale-recovery
owner: scenarios
kind: suite
entrypoint: /spec:plan
stages: [plan, refine, build, merge]
isolation: fresh-project
assertions:
  - plan-exists
  - dirty-slot-preserved
  - slice-state-preserved
  - resume-continues-from-in-progress
  - execute-loop-all-done
negative-expectations:
  - live-model-ci-required
  - semantic-byte-golden-required
expected-artifacts:
  - plan.yaml
  - workspace
---

# Stale-workspace recovery

Scenario ID: `workspace-stale-recovery`

## Intent

Prove re-entry after an interrupted execute: the operator interrupts `specify plan execute` mid-run leaving a workspace slot dirty with uncommitted work, inspects and preserves that state through operator-owned repository handling, then resumes from the in-progress entry rather than restarting or dropping work.

## Setup

Follow the **cross-repo workspace setup** in [`shared/setup.md`](../shared/setup.md) and the **OAuth login brief**. Author and approve a multi-slice plan.

## Invocation

1. **Execute + interrupt** — `specify plan execute`; interrupt mid-run, leaving a slot dirty with uncommitted work. (Workspace routing has no in-guest counterpart yet: the verb currently exits with the typed `plan-execute-workspace-unsupported` refusal, so a run files as blocked until the workspace leg lands.)
2. **Inspect** — use `git status` in the affected slot; confirm the interrupted work remains present and make the slot safe for resume without discarding slice state.
3. **Resume** — `specify plan execute`; confirm it continues from the in-progress entry and reaches drained.

## Assertions

- `plan-exists`: `plan.yaml` exists and is approved before execute.
- `dirty-slot-preserved`: operator inspection confirms the dirty/uncommitted slot state survives the interruption.
- `slice-state-preserved`: slice state survives the interruption (no lost or duplicated work).
- `resume-continues-from-in-progress`: resume continues from the in-progress entry, not a restart.
- `execute-loop-all-done`: the resumed loop reaches drained.

## Negative expectations

Live profiles remain outside per-commit CI, and semantic output is rubric-graded rather than byte-golden tested.

## Recording

Capture with [`shared/run-template.md`](../shared/run-template.md) as [`evals/runs/<id>.<result>.md`](../runs/README.md).
