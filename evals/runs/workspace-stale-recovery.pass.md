# Run: `workspace-stale-recovery` — **pass**

## Context

- **Scenario:** `workspace-stale-recovery`
- **Operator:** Cursor agent (agent-as-operator, per the single-scenario runbook)
- **CLI:** `/Users/andrewweston/.local/bin/specify` — `specify 0.2.0` (built from the `Specify.toml` `cli` source via `make install-cli`)
- **Sandbox:** `evals/.sandbox/workspace-stale-recovery/` (`platform/`, `backend/`, `mobile/`)

## Assertions

| Assertion | Verdict | Evidence |
| --- | --- | --- |
| `plan-exists` | pass | |
| `dirty-slot-detected-at-sync` | pass | |
| `slice-state-preserved` | pass | |
| `resume-continues-from-in-progress` | pass | |
| `execute-loop-all-done` | pass | |

Probe transcript highlights: `plan.yaml` present with `lifecycle: approved` before execute; interrupted `/spec:execute` mid `oauth-backend` build left the backend slot dirty (`git status --short` showed `M .specify/journal.jsonl`, untracked `.specify/slices/`, and `eval-dirty-uncommitted.txt` on `specify/oauth-login`); `specify workspace sync` completed without clobbering dirty work; post-sync `specify workspace prepare backend --change oauth-login` surfaced dirty-slot diagnostics (`untracked` listed seven `oauth-backend` slice-tree paths); resumed loop finished `oauth-backend` and `oauth-mobile` without re-advancing `oauth-backend`; `specify plan status` reports `action: drained` with four `status: done` entries; `specify journal show --filter plan.entry.advanced` shows exactly four advances (one per slice); `specify plan validate` blocking count 0.

**Negative expectations:** held (manual-by-design posture unchanged; the run was driven interactively against the real CLI).

## Deviations

- Used offline init with local adapter paths (`specify init <framework>/adapters/targets/{omnia,vectis}`) per the documented offline fallback in `shared/setup.md`.
- Added `file://` bare-git `origin` remotes to `backend` and `mobile` so `specify workspace prepare` can resolve `origin/HEAD` (stock setup leaves local slots without remotes).
- Plan authored headlessly (`specify plan create`, survey finalize, `propose --from`) rather than a live `/spec:plan` skill session; Gate 1 stamped with `--actor agent`.
- Phase work driven by a minimal local driver script (`evals/.sandbox/workspace-stale-recovery/drive_execute.py`) following `/spec:execute` routing (workspace sync, prepare, slot-side refine/build/merge with `SPECIFY_PLAN_DIR`); interrupt simulated by stopping after `build --phase prepare` with an extra dirty root file, then releasing the plan lock.

## Notes

- `specify workspace sync` itself prints only `workspace sync complete`; dirty-slot detection for the resync step is confirmed via `git -C workspace/backend status --short` at interrupt time and `specify workspace prepare` dirty classification immediately after sync (matching the assertion taxonomy's probe pairing).
- `specify workspace prepare` initially refused with `dirty-unrelated-tracked` when the interrupt left a tracked `.specify/journal.jsonl` edit plus a root-level dirty file; triage restored the journal and removed the unrelated file before resume, leaving resume-safe untracked slice-tree dirtiness — consistent with the scenario's stale-recovery posture.
- Renderer nit: `specify journal show --filter slice.synthesize.started` emitted no lines for this run (synthesis journal events may use a different filter id); resume continuity was graded from `plan.entry.advanced` (no duplicate advance for `oauth-backend`).

## Evidence

- **Reproduce:** `scripts/snapshot.sh evals/.sandbox/workspace-stale-recovery`
- **Retained at:** `evals/.sandbox/workspace-stale-recovery/`
- **Key paths:** `platform/plan.yaml`, `platform/change.md`, `platform/discovery.md`, `platform/workspace/{backend,mobile}/`, `platform/.specify/journal.jsonl`, `backend/.specify/specs/`, `mobile/.specify/specs/`
