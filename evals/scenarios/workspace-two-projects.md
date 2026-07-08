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
  - guest-lock-at-workspace
  - execute-loop-all-done
negative-expectations:
  - automated-runner-added
  - fake-forge-added
  - transcript-replay-added
  - ci-target-added
  - golden-output-required
expected-artifacts:
  - plan.yaml
  - registry.yaml
  - workspace
---

# Workspace plan execute across two projects

Scenario ID: `workspace-two-projects`

## Intent

Prove workspace-driven execution across projects: a plan with slices targeting two registered projects executes from the workspace, with per-slice project routing, slot materialisation, `workspace prepare`, `chdir` + residue commit, and the guest execute marker held at the workspace while phase work runs in the slots.

## Setup

Follow the **cross-repo workspace setup** in [`shared/setup.md`](../shared/setup.md) and the **OAuth login brief**. Author and approve a plan whose slices route to `backend` and `mobile`.

## Invocation

1. **Execute** — `specify plan execute` from the workspace. (Workspace routing has no in-guest counterpart yet: the verb currently exits with the typed `plan-execute-workspace-unsupported` refusal, so a run files as blocked until the workspace leg lands.)
2. **Inspect** — `inspect plan.yaml`; `inspect workspace/<project>` with `git status`. Confirm each slice ran in its routed slot.

## Assertions

- `plan-exists`: `plan.yaml` exists and is approved before execute.
- `per-slice-project-routing`: each slice runs against its routed project slot.
- `slots-materialised`: `workspace/backend/` and `workspace/mobile/` are materialised.
- `guest-lock-at-workspace`: the guest execute marker is held at the workspace while phase work runs in slots.
- `execute-loop-all-done`: the loop reaches drained.

## Negative expectations

Manual by design — see [`docs/contributing/evals.md`](../../docs/contributing/evals.md). No automated runner, fake forge, recorded transcript, CI target, or golden comparison.

## Recording

Capture with [`shared/run-template.md`](../shared/run-template.md) as [`evals/runs/<id>.<result>.md`](../runs/README.md).
