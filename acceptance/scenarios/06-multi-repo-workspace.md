---
id: multi-repo-workspace
owner: lifecycle
kind: suite
backend: fixture
entrypoint: /spec:plan
stages: [plan]
isolation: fresh-project
assertions:
  - plan-exists
  - plan-validates
  - workspace-discriminator-set
  - per-candidate-project-routing
  - workspace-sync-before-propose
expected-artifacts:
  - plan.yaml
  - registry.yaml
---

# Multi-repo assignment from a workspace

Scenario ID: `multi-repo-workspace`

> **Automated (`backend: fixture`).** This scenario's routing assertions are a deterministic kernel projection and proven by fixture-driven tests in the deterministic surface — no manual sweep run is required. See [Automated coverage](#automated-coverage).

## Intent

Prove multi-repo plan authoring from a registry-only workspace: the `workspace:` discriminator is set, the propose step routes each candidate to a project via `--project`, and `workspace sync` runs at the right time so routing sees materialised peers. The scenario stops at Gate 1.

## Automated coverage

Proven by the propose kernel and workspace-sync tests in [`augentic/specify-cli`](https://github.com/augentic/specify-cli), run under `cargo make test`. The agent groups leads into a response; the kernel's routing over that response is deterministic (the response is fixture-provided here):

- `plan-exists` / `plan-validates`: `tests/plan_orchestrate/validate.rs::plan_validate_clean_json`.
- `workspace-discriminator-set`: workspace mode is established by `specify init --workspace`, exercised by `tests/workspace.rs` (sync over a registry-only workspace).
- `per-candidate-project-routing`: `tests/plan_orchestrate/propose.rs::propose_from_fan_out_golden` writes single-target slices each bound to their `project`; `reconcile_project_binding_required` / `propose_reconcile_project_orphan` pin the routing guards.
- `workspace-sync-before-propose`: `tests/workspace.rs::planning_sync_two_symlink_peers` materialises the peer context that routing reads (no orphan/unrouted candidate).

## Reproducing by hand (optional)

The fixture tests are the source of truth; the steps below only reproduce it for inspection. Follow the **cross-repo workspace setup** in [`shared/setup.md`](../shared/setup.md) (workspace plus registered `shop-backend` / `shop-mobile`) and the **OAuth login brief**, run `/spec:plan oauth-login source brief=docs/oauth-login.md` from the workspace, and inspect per-candidate routing in `plan.yaml`. Stop at Gate 1.
