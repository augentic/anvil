# Run: `workspace-stale-recovery` — **pass**

## Context

- **Scenario:** `workspace-stale-recovery`
- **Operator:** Cursor agent (agent-as-operator, per the single-scenario runbook)
- **CLI:** `/Users/andrewweston/.local/bin/specify` — `specify 0.2.0` (built from the `Specify.toml` `cli` source via `make install-cli`)
- **Sandbox:** `evals/.sandbox/workspace-stale-recovery/` (`platform/`, `backend/`, `mobile/`, `contracts/`)

## Assertions

| Assertion | Verdict | Evidence |
| --- | --- | --- |
| `plan-exists` | pass | |
| `dirty-slot-detected-at-sync` | pass | |
| `slice-state-preserved` | pass | |
| `resume-continues-from-in-progress` | pass | |
| `execute-loop-all-done` | pass | |

Probe transcript highlights: `plan.yaml` present with `lifecycle: approved` before execute; interrupted `/spec:execute` mid `oauth-backend` build left the backend slot dirty (`git status --short` showed `M .specify/journal.jsonl`, untracked `.specify/slices/oauth-backend/build/`, and `eval-dirty-uncommitted.txt` on `specify/oauth-login`); `specify workspace sync` completed without clobbering dirty work; post-sync triage removed the unrelated root dirty marker and committed resume-safe slice-tree dirtiness; resumed loop finished `oauth-backend` and `oauth-mobile` without re-advancing `oauth-backend`; `specify plan status` reports `action: drained` with four `status: done` entries; `specify journal show --filter plan.entry.advanced` shows exactly four advances (one per slice); `specify plan validate` blocking count 0.

**Negative expectations:** held (manual-by-design posture unchanged; the run was driven interactively against the real CLI).

## Deviations

- Used offline init with local adapter paths (`specify init <framework>/adapters/targets/{omnia,vectis,contracts}`) per the documented offline fallback in `shared/setup.md`.
- Added `file://` bare-git `origin` remotes to `backend`, `mobile`, and `contracts` so `specify workspace prepare` can resolve `origin/HEAD`.
- Plan authored headlessly (`specify plan create`, survey finalize, `propose --from`) rather than a live `/spec:plan` skill session; Gate 1 stamped with `--actor agent`.
- Phase work driven by `evals/.sandbox/workspace_driver.py` following `/spec:execute` routing; interrupt simulated by stopping after `build --phase prepare` with an extra dirty root file, then releasing the plan lock.

## Notes

- `specify workspace sync` itself prints only `workspace sync complete`; dirty-slot detection for the resync step is confirmed via `git -C workspace/backend status --short` at interrupt time and post-sync triage before resume (matching the assertion taxonomy's probe pairing).
- Resume continuity graded from `plan.entry.advanced` (no duplicate advance for `oauth-backend` after interrupt).
- `specify plan validate` emits a non-blocking `topology-cache-stale` suggestion for the mobile slot after drain.

## Evidence

- **Reproduce:** `scripts/snapshot.sh evals/.sandbox/workspace-stale-recovery`
- **Retained at:** `evals/.sandbox/workspace-stale-recovery/`
- **Key paths:** `platform/plan.yaml`, `platform/change.md`, `platform/discovery.md`, `platform/workspace/{backend,mobile,contracts}/`, `platform/.specify/journal.jsonl`, `backend/.specify/specs/`, `mobile/.specify/specs/`
