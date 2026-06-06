---
id: contract-routing
owner: scenarios
kind: suite
backend: fixture
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
  - change.md
  - discovery.md
---

# Contract routing plan generation

Scenario ID: `contract-routing`

> **Automated (`backend: fixture`).** This scenario's routing/dependency assertions are a deterministic kernel projection and proven by fixture-driven tests in the deterministic surface — no manual sweep run is required. See [Automated coverage](#automated-coverage).

## Intent

Prove the plan-generation half of the cross-repo contract-first path: a short feature brief becomes one contract slice and routed implementation slices, with deterministic project routing — without executing, pushing, or finalizing. This is the plan-only stop variant of [`cross-repo-contract-flow`](cross-repo-contract-flow.md), which exercises the live-forge finalize tail (and stays manual).

## Automated coverage

Proven by the propose kernel tests in [`augentic/specify-cli` `tests/plan_orchestrate/propose.rs`](https://github.com/augentic/specify-cli/blob/main/tests/plan_orchestrate/propose.rs) and the depends-on ordering in `tests/fan_in_fan_out.rs`, run under `cargo make test`. The agent's grouping response is fixture-provided; the kernel's routing, binding, and `depends-on` derivation over it are deterministic:

- `plan-exists` / `plan-validates`: `tests/plan_orchestrate/validate.rs::plan_validate_clean_json`.
- `contract-slice-present` / `implementation-slices-routed`: `propose_from_fan_out_golden` writes a contract-bound slice plus single-target implementation slices each bound to their project.
- `dependencies-correct`: the same golden + `fan_in_fan_out.rs` assert the implementation slices carry `depends-on` the contract slice, and the driver never advances to a dependent before its upstream merges.
- `routing-deterministic`: `propose_from_fan_out_golden` is a byte-stable golden, and `propose_dry_run_workspace_request_golden` pins the request envelope — routing does not depend on prose wording.

## Reproducing by hand (optional)

The fixture tests are the source of truth; the steps below only reproduce it for inspection. Follow the **cross-repo workspace setup** in [`shared/setup.md`](../shared/setup.md) and the **OAuth login brief**, run `/spec:plan oauth-login-plan from docs/oauth-login.md` asking for one contract slice plus routed backend/mobile implementation slices, then `specify plan validate` and inspect `plan.yaml`. Do not execute, push, or finalize.
