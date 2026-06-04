---
id: multi-repo-workspace
owner: lifecycle
kind: suite
backend: manual
entrypoint: /spec:plan
stages: [plan]
isolation: fresh-project
assertions:
  - plan-exists
  - plan-validates
  - workspace-discriminator-set
  - per-candidate-project-routing
  - workspace-sync-before-propose
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

# Multi-repo assignment from a workspace

Scenario ID: `multi-repo-workspace`

## Intent

Prove multi-repo plan authoring from a registry-only workspace: the `workspace:` discriminator is set, the propose step routes each candidate to a project via `--project`, and `workspace sync` runs at the right time so routing sees materialised peers. The scenario stops at Gate 1.

## Setup

Follow the **cross-repo workspace setup** in [`shared/setup.md`](../../shared/setup.md) (workspace plus registered `shop-backend` / `shop-mobile`) and the **OAuth login brief**.

## Invocation

1. **Plan** — `/spec:plan oauth-login source brief=docs/oauth-login.md` from the workspace; confirm the propose step assigns each candidate to a project.
2. **Review** — inspect `plan.yaml`; confirm per-candidate project routing matches the registry descriptions. Stop at Gate 1.

## Assertions

- `plan-exists`: `plan.yaml` exists after `/spec:plan`.
- `plan-validates`: `specify plan validate` exits cleanly.
- `workspace-discriminator-set`: the plan carries the `workspace:` discriminator.
- `per-candidate-project-routing`: each routed slice carries an explicit `--project` assignment.
- `workspace-sync-before-propose`: routing reflects synced peer context (no orphan/unrouted candidate).

## Negative expectations

Manual by design — see [`docs/contributing/acceptance.md`](../../../../docs/contributing/acceptance.md). No automated runner, fake forge, recorded transcript, CI target, or golden comparison.

## Recording

Capture with [`shared/run-summary-template.md`](../../shared/run-summary-template.md) under [`acceptance/runs/`](../../../runs/README.md).
