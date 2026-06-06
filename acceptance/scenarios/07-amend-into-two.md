---
id: amend-into-two
owner: lifecycle
kind: suite
backend: fixture
entrypoint: /spec:plan
stages: [plan]
isolation: fresh-project
assertions:
  - plan-exists
  - plan-validates
  - amend-splits-into-two
  - dependencies-coherent-after-amend
  - gate-1-reentry
expected-artifacts:
  - plan.yaml
---

# Operator amends a one-slice plan into two at Gate 1

Scenario ID: `amend-into-two`

> **Automated (`backend: fixture`).** This scenario's assertions are deterministic CLI behavior and proven by fixture-driven tests in the deterministic surface — no manual sweep run is required. See [Automated coverage](#automated-coverage).

## Intent

Prove the Gate-1 amendment flow: an operator splits a one-slice plan into two slices via `specify plan amend`, the resulting dependencies stay coherent, and the plan re-enters Gate 1 at `pending` after the amend.

## Automated coverage

Proven by the plan-mutation and transition tests in [`augentic/specify-cli` `tests/plan_orchestrate/`](https://github.com/augentic/specify-cli/tree/main/tests/plan_orchestrate), run under `cargo make test`. The slice decomposition itself is an operator judgment; the CLI mechanics that make it safe are deterministic:

- `plan-exists` / `plan-validates`: `validate.rs::plan_validate_clean_json` and `create.rs::plan_create_then_validate_passes_clean`.
- `amend-splits-into-two`: `mutate.rs::plan_add_appends_pending_entry_json` (add the second slice) — `plan.yaml` round-trips with both entries.
- `dependencies-coherent-after-amend`: `mutate.rs::plan_amend_replaces_depends_on` proves the dependency edge is rewritten coherently; `mutate.rs::plan_remove_refuses_when_depended_on` proves cycles/dangling edges are refused.
- `gate-1-reentry`: a plan stays at `pending` after `plan add`/`plan amend` (lifecycle is only written by `plan transition`); `transition.rs::transition_rejects_per_entry_in_progress` and `plan_transition_happy_path_text` pin the Gate-1 transition surface.

## Reproducing by hand (optional)

The fixture tests are the source of truth; the steps below only reproduce it for inspection. Follow the **single-project setup** in [`shared/setup.md`](../shared/setup.md) with `specify init omnia@v1`, author a one-slice plan named `profile-page`, then `specify plan amend` to split the slice into two with a coherent dependency edge, re-validate, and confirm the plan remains at `pending` printing the transition command.
