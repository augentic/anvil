# Run: `single-project-plan` — **pass**

## Context

- **Scenario:** `single-project-plan`
- **Operator:** Cursor agent (agent-as-operator, per the single-scenario runbook)
- **CLI:** `/Users/andrewweston/.local/bin/specify` — `specify 0.2.0` (built from the `Specify.toml` `cli` source via `make install-cli`)
- **Sandbox:** `evals/.sandbox/single-project-plan/`

## Assertions

| Assertion | Verdict | Evidence |
| --- | --- | --- |
| `plan-exists` | pass | |
| `plan-validates` | pass | |
| `slices-match-expected-shape` | pass | One slice `inventory-adjustments` maps to the brief's sole feature: synopsis and rationale cover all four Goals (record adjustment for SKU/warehouse, require reason and operator id, reject negative available stock, emit audit event on success); `change.md` Out of scope mirrors the brief's Scope exclusions (no bulk imports, approval workflows, warehouse transfers). No goal lacks a slice home; no slice lacks brief grounding. |
| `no-project-routing-required` | pass | |

Probe transcript highlights: `plan.yaml` present with `lifecycle: pending`; `specify plan validate --format json` exits 0 with zero findings; `registry.yaml` absent; sole `project: project` line is the auto-bound project from `project.yaml` (not registry routing); `plan.reconcile.completed` payload reads `"slice-count":1`; no `plan.transition.approved` or `plan.entry.advanced` events (scenario stops at Gate 1).

**Negative expectations:** held (manual-by-design posture unchanged; the run was driven interactively against the real CLI).

## Deviations

- Used local adapter path `specify init <framework>/adapters/targets/omnia` instead of `omnia@v1` (consistent with other eval runs; offline fallback documented in shared setup).
- Symlinked the `documentation` source adapter into the sandbox (`adapters/sources/documentation`) per the setup prerequisite — `specify init` caches only the target adapter.
- Phase work driven by the agent following the `/spec:plan` skill body directly (plan create → survey → propose → validate); Gate 1 not stamped and `/spec:execute` not invoked per the scenario's Invocation.

## Notes

- Single-lead documentation survey: monolithic `docs/inventory-adjustments.md` surfaced one lead; one slice is the sensible decomposition for the tightly scoped brief (multi-slice would over-split a single transactional flow).
- `plan.yaml` `description: null` on the slice row is expected — rationale lives in `change.md` and the propose response envelope, not persisted on the slice entry.

## Evidence

- **Reproduce:** `scripts/snapshot.sh evals/.sandbox/single-project-plan`
- **Retained at:** `evals/.sandbox/single-project-plan/`
- **Key paths:** `plan.yaml`, `change.md`, `discovery.md`, `docs/inventory-adjustments.md`, `.specify/scratch/plan/propose-response.json`, `.specify/journal.jsonl`
