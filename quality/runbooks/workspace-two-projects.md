---
id: workspace-two-projects
owner: scenarios
kind: suite
entrypoint: /spec:plan
stages: [plan, refine, build, merge]
isolation: fresh-project
assertions:
  - plan-exists
  - per-slice-project-routing
  - slots-materialised
  - plan-lock-at-workspace
  - execute-loop-all-done
negative-expectations:
  - live-model-ci-required
  - semantic-byte-golden-required
expected-artifacts:
  - plan.yaml
  - registry.yaml
  - workspace
---

# Workspace plan execute across two projects

Scenario ID: `workspace-two-projects`

## Intent

Prove workspace-driven execution across projects: a plan with slices targeting two registered projects executes from the workspace, with per-slice project routing into operator-materialized slots and the plan lock held at the workspace while phase work runs in the slots.

## Setup

Follow the **cross-repo workspace setup** in [`shared/setup.md`](../shared/setup.md) and the **OAuth login brief**. Author and approve a plan whose slices route to `backend` and `mobile`.

## Invocation

1. **Execute** — `specify plan execute` from the workspace.
2. **Inspect** — `inspect plan.yaml`; `inspect workspace/<project>` with `git status`. Confirm each slice ran in its routed slot.

## Assertions

- `plan-exists`: `plan.yaml` exists and is approved before execute.
- `per-slice-project-routing`: each slice runs against its routed project slot.
- `slots-materialised`: `workspace/backend/` and `workspace/mobile/` are materialised.
- `plan-lock-at-workspace`: the workspace root owns the plan lock while phase work runs in slots; unlocked mutation is refused.
- `execute-loop-all-done`: the loop reaches drained.

## Negative expectations

Live profiles remain outside per-commit CI, and semantic output is rubric-graded rather than byte-golden tested.

## Recording

Capture with [`shared/run-template.md`](../shared/run-template.md) as [`evals/runs/<id>.<result>.md`](../runs/README.md).
