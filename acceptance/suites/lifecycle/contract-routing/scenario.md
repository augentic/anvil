---
id: contract-routing
owner: lifecycle
kind: suite
backend: manual
entrypoint: /spec:plan
stages: [plan]
isolation: fresh-project
assertions:
  - plan-exists
  - plan-validates
  - contract-slice-present
  - implementation-slices-routed
  - dependencies-correct
  - routing-deterministic
expected-artifacts:
  - plan.yaml
  - registry.yaml
  - .specify/plans/oauth-login-plan/discovery.md
  - .specify/plans/oauth-login-plan/proposal.md
  - .specify/plans/oauth-login-plan/workspace.md
negative-expectations:
  - automated-runner-added
  - fake-forge-added
  - transcript-replay-added
  - ci-target-added
  - golden-output-required
---

# Contract routing plan generation

Scenario ID: `contract-routing`

## Intent

Prove the plan-generation half of the cross-repo contract-first path: a short feature brief becomes one contract slice and routed implementation slices, with deterministic project routing — without executing, pushing, or finalizing. This is the plan-only stop variant of [`cross-repo-contract-flow`](../cross-repo-contract-flow/scenario.md); the two share setup and may be consolidated in a future pass.

## Setup

Follow the **cross-repo workspace setup** in [`shared/setup.md`](../../shared/setup.md) and the **OAuth login brief** at `docs/oauth-login.md`.

## Invocation

1. **Plan** — `/spec:plan oauth-login-plan from docs/oauth-login.md`, asking for one contract slice plus backend and mobile implementation slices that both depend on the contract slice.
2. **Validate + inspect** — `specify plan validate`; `specify registry validate`; inspect `plan.yaml`. Do not run `/spec:execute`, `specify workspace push`, or `specify plan archive`.

## Assertions

- `plan-exists`: `plan.yaml` exists after `/spec:plan`.
- `plan-validates`: `specify plan validate` exits cleanly.
- `contract-slice-present`: the plan includes a contract slice before implementation work begins.
- `implementation-slices-routed`: implementation slices route to `shop-backend` and `shop-mobile`.
- `dependencies-correct`: each implementation slice depends on the contract slice.
- `routing-deterministic`: project assignments match the registry descriptions and do not depend on generated prose wording.

## Negative expectations

Manual by design — see [`docs/contributing/acceptance.md`](../../../../docs/contributing/acceptance.md). No automated runner, fake forge, recorded transcript, CI target, or golden comparison.

## Recording

Capture with [`shared/run-summary-template.md`](../../shared/run-summary-template.md) under [`acceptance/runs/`](../../../runs/README.md).
