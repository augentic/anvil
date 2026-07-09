# Run: `workspace-two-projects` — **pass**

## Context

- **Scenario:** `workspace-two-projects`
- **Operator:** Cursor agent (agent-as-operator, per the single-scenario runbook)
- **CLI:** `/Users/andrewweston/.local/bin/specify` — `specify 0.2.0` (built from the `Specify.toml` `cli` source)
- **Sandbox:** `evals/.sandbox/workspace-two-projects/` (`platform/`, `backend/`, `mobile/`, `contracts/`)

## Assertions

| Assertion | Verdict | Evidence |
| --- | --- | --- |
| `plan-exists` | pass | |
| `per-slice-project-routing` | pass | |
| `slots-materialised` | pass | |
| `plan-lock-at-workspace` | pass | |
| `execute-loop-all-done` | pass | |

Probe transcript highlights: `plan.yaml` present with `lifecycle: approved` before execute; `specify plan status --format json` reports `"action":"drained"` with `"counts":{"done":4}`; `grep -c 'status: done' plan.yaml` returns 4; `grep 'project:' plan.yaml` names `contracts` for `oauth-contract`, `backend` for `oauth-backend`, and `mobile` for `app-foundation`/`oauth-mobile`; `test -d workspace/{backend,mobile}` succeeds; `plan.yaml` lives at workspace root with no slot `plan.yaml`; unlocked `specify plan next` returns `"error":"plan-lock-not-held"` (exit 2); `specify journal show --filter plan.entry.advanced` shows four advances (one per slice).

**Negative expectations:** held (manual-by-design posture unchanged; the run was driven interactively against the real CLI).

## Deviations

- Used offline init with local adapter paths (`specify init <framework>/adapters/targets/{omnia,vectis,contracts}`) per documented offline fallback in `shared/setup.md`.
- Symlinked the `documentation` source adapter into the workspace (`adapters/sources/documentation`) per setup prerequisite.
- Gate 1 stamped with `specify plan transition oauth-login approved --actor agent`.
- Plan authored headlessly (`specify plan create`, survey finalize, `propose --from`) rather than a live `/spec:plan` skill session; default-on platform bootstrap inserted `app-foundation` bootstrap slice for mobile.
- Execute driven by `evals/drivers/workspace.sh workspace-two-projects` following `/spec:execute` routing (workspace sync/prepare, `SPECIFY_PLAN_DIR`, refine → build → merge per slice) with minimal omnia/vectis/contracts build stubs; finalize not run (execute-only per scenario stages).
- Inter-slice residue commits after refine/build in each slot to satisfy `workspace prepare` dirty-boundary classification between phases.

## Notes

- `specify plan next --format json` after drain (lock released) returns `plan-lock-not-held` (exit 2) rather than a drained payload — `specify plan status` is the authoritative drained signal post-execute; consistent with `plan-lock-at-workspace`.

## Evidence

- **Retained at:** `evals/.sandbox/workspace-two-projects/`
- **Key paths:** `platform/plan.yaml`, `platform/change.md`, `platform/discovery.md`, `platform/registry.yaml`, `platform/workspace/{backend,mobile,contracts}`, `platform/.specify/journal.jsonl`, `backend/.specify/specs/`, `mobile/.specify/specs/`
