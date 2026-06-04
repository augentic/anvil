---
id: dual-driving-refused
owner: lifecycle
kind: suite
backend: manual
entrypoint: /spec:plan
stages: [plan]
isolation: fresh-project
assertions:
  - workspace-plan-active
  - plan-from-project-refused
  - one-driving-mode-per-project
negative-expectations:
  - automated-runner-added
  - fake-forge-added
  - transcript-replay-added
  - ci-target-added
  - golden-output-required
expected-artifacts:
  - plan.yaml
  - registry.yaml
---

# Dual-driving refused

Scenario ID: `dual-driving-refused`

## Intent

Prove the one-driving-mode-per-project invariant: with a workspace-driven plan active for a registered project, running `/spec:plan` from that project's own root is refused.

## Setup

Follow the **cross-repo workspace setup** in [`shared/setup.md`](../shared/setup.md). Author and approve a workspace plan that routes a slice to `shop-backend` so that project is being workspace-driven.

## Invocation

1. **Workspace plan active** — confirm a workspace-driven plan is active for `shop-backend`.
2. **Attempt project-root plan** — from `shop-backend/`, run `/spec:plan local-change "..."`; confirm it is refused with a structured error citing the active workspace driving mode.

## Assertions

- `workspace-plan-active`: a workspace-driven plan is active for the project.
- `plan-from-project-refused`: `/spec:plan` from the project root is refused, not silently allowed.
- `one-driving-mode-per-project`: the refusal cites the one-driving-mode-per-project invariant.

## Negative expectations

Manual by design — see [`docs/contributing/acceptance.md`](../../docs/contributing/acceptance.md). No automated runner, fake forge, recorded transcript, CI target, or golden comparison.

## Recording

Capture with [`shared/run-summary-template.md`](../shared/run-summary-template.md) under [`acceptance/runs/`](../runs/README.md).
