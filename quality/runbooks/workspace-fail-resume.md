---
id: workspace-fail-resume
owner: scenarios
kind: suite
entrypoint: /spec:plan
stages: [plan, refine, build, merge]
isolation: fresh-project
assertions:
  - plan-exists
  - breakout-routes-to-slot
  - active-slice-resolved-across-boundary
  - chdir-without-operator-intervention
  - execute-loop-all-done
negative-expectations:
  - live-model-ci-required
  - semantic-byte-golden-required
expected-artifacts:
  - plan.yaml
  - workspace
---

# Workspace breakout after build failure in a slot

Scenario ID: `workspace-fail-resume`

## Intent

Prove breakout-verb routing across the workspace/slot boundary: `specify plan execute` parks on a slice in one project, the operator stays at the workspace and runs `/spec:build`, and the breakout verb resolves the active slice and `chdir`s into the correct slot without operator intervention.

## Setup

Follow the **cross-repo workspace setup** in [`reference/setup.md`](../reference/setup.md) and the **OAuth login brief**. Author and approve a plan that parks on a slice (e.g. `auth-rotate`) in `backend`.

## Invocation

1. **Execute** — `specify plan execute` from the workspace; let it park on the backend slice. (Workspace routing has no in-guest counterpart yet: the verb currently exits with the typed `plan-execute-workspace-unsupported` refusal, so a run files as blocked until the workspace leg lands.)
2. **Breakout** — from the workspace, run `/spec:build`; confirm it routes into the backend slot automatically.
3. **Resume** — `specify plan execute`; confirm drained.

## Assertions

- `plan-exists`: `plan.yaml` exists and is approved before execute.
- `breakout-routes-to-slot`: `/spec:build` from the workspace routes into the parked slice's project slot.
- `active-slice-resolved-across-boundary`: the breakout verb resolves the active slice across the workspace/slot boundary.
- `chdir-without-operator-intervention`: the correct `chdir` happens without the operator changing directories.
- `execute-loop-all-done`: the resumed loop reaches drained.

## Negative expectations

Live profiles remain outside per-commit CI, and semantic output is rubric-graded rather than byte-golden tested.

## Recording

Capture with [`reference/run-template.md`](../reference/run-template.md) as [`quality/runs/archive/<id>.<result>.md`](../runs/README.md).
