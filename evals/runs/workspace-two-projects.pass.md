# Run: `workspace-two-projects` — **pass**

## Context

- **Scenario:** `workspace-two-projects`
- **Operator:** Cursor agent (agent-as-operator, per the single-scenario runbook)
- **CLI:** `/Users/andrewweston/.local/bin/specify` — `specify 0.2.0` (built from the `Specify.toml` `cli` source via `make install-cli`)
- **Sandbox:** `evals/.sandbox/workspace-two-projects/` (`platform/`, `backend/`, `mobile/`, `contracts/`)

## Assertions

| Assertion | Verdict | Evidence |
| --- | --- | --- |
| `plan-exists` | pass | |
| `per-slice-project-routing` | pass | |
| `slots-materialised` | pass | |
| `plan-lock-at-workspace` | pass | |
| `execute-loop-all-done` | pass | |

Probe transcript highlights: `plan.yaml` present with `lifecycle: approved` before execute; `specify plan status --format json` reports `"action":"drained"` with `"counts":{"done":4}`; `grep -c 'status: done' plan.yaml` returns 4; `grep 'project:' plan.yaml` names `contracts` for `oauth-contract`, `backend` for `oauth-backend`, and `mobile` for `app-foundation`/`oauth-mobile`; `git -C workspace/backend log --oneline specify/oauth-login` shows residue commits for `oauth-backend`; `git -C workspace/mobile log --oneline specify/oauth-login` shows residue commits for `app-foundation` and `oauth-mobile`; `git -C workspace/contracts log --oneline specify/oauth-login` shows residue commits for `oauth-contract`; `test -d workspace/{backend,mobile}` succeeds; first `workspace.sync.completed` payload lists `"projects":["backend","mobile","contracts"]`; `plan.yaml` lives at workspace root with no slot `plan.yaml`; unlocked `specify plan next` returns `"error":"plan-lock-not-held"` (exit 2).

**Negative expectations:** held (manual-by-design posture unchanged; the run was driven interactively against the real CLI).

## Deviations

- Used offline init with local adapter paths (`specify init <framework>/adapters/targets/{omnia,vectis,contracts}`) per documented offline fallback in `shared/setup.md`.
- Symlinked the `documentation` source adapter into the workspace (`adapters/sources/documentation`) per setup prerequisite.
- Gate 1 stamped with `specify plan transition oauth-login approved --actor agent`.
- Plan authored headlessly (`specify plan create`, survey finalize, `propose --from`) rather than a live `/spec:plan` skill session; default-on platform bootstrap inserted `app-foundation` bootstrap slice for mobile.
- Execute driven by a local `evals/.sandbox/workspace_driver.py` helper following `/spec:execute` routing (workspace sync/prepare, `SPECIFY_PLAN_DIR`, refine → build → merge per slice) with minimal omnia/vectis/contracts build stubs; finalize not run (execute-only per scenario stages).
- Inter-slice residue commits after refine/build in each slot to satisfy `workspace prepare` dirty-boundary classification between phases.

## Notes

- `specify plan next --format json` after drain (lock released) returns `plan-lock-not-held` (exit 2) rather than a drained payload — `specify plan status` is the authoritative drained signal post-execute; consistent with `plan-lock-at-workspace`.
- `plan.entry.advanced` journal shows four events (`app-foundation`, `oauth-contract`, `oauth-backend`, `oauth-mobile`); one advance per slice.

## Evidence

- **Reproduce:** `scripts/snapshot.sh evals/.sandbox/workspace-two-projects`
- **Retained at:** `evals/.sandbox/workspace-two-projects/`
- **Key paths:** `platform/plan.yaml`, `platform/change.md`, `platform/discovery.md`, `platform/registry.yaml`, `platform/workspace/{backend,mobile,contracts}`, `platform/.specify/journal.jsonl`, `backend/.specify/specs/`, `mobile/.specify/specs/`
