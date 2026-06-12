# Run: `workspace-two-projects` — **pass**

## Context

- **Scenario:** `workspace-two-projects`
- **Operator:** Cursor agent (agent-as-operator, per the single-scenario runbook)
- **CLI:** `/Users/andrewweston/.local/bin/specify` — `specify 0.2.0` (built from sibling `specify-cli` via `make install-cli`)
- **Sandbox:** `evals/.sandbox/workspace-two-projects/` (workspace root `platform/`, peers `backend/`, `mobile/`)

## Assertions

| Assertion | Verdict | Evidence |
| --- | --- | --- |
| `plan-exists` | pass | |
| `per-slice-project-routing` | pass | |
| `slots-materialised` | pass | |
| `plan-lock-at-workspace` | pass | |
| `execute-loop-all-done` | pass | |

Probe transcript highlights: `plan.yaml` at the workspace root with `lifecycle: approved` before execute (Gate 1 stamped `--actor agent`). Four slices, each `status: done` (4/4), `specify plan status` reporting `drained` with `resume: /spec:finalize oauth-login`, and exactly four `plan.entry.advanced` events in the workspace journal — none in any slot. Both slots materialised at `.specify/workspace/{backend,mobile}`; `workspace.sync.completed` payloads list both projects. Per-slice routing: `bootstrap-core` + `mobile-oauth-signin` merge/residue commits land in the mobile slot, `oauth-contract` + `backend-oauth-exchange` in the backend slot. No slot grew a `plan.yaml`; after the driver released the lock, `specify plan next` refused with `plan-lock-not-held` (exit 2).

**Negative expectations:** held (manual-by-design posture unchanged; live interactive drive against the real CLI and real git workspaces).

## Deviations

- `omnia@v1` / `vectis@v1` GitHub shorthand failed (`Remote branch v1 not found`); setup used local adapter paths from the framework checkout instead.
- `mobile` initialised with `--platforms core` only (host lacks iOS/Android toolchains); platform reconciliation inserted `bootstrap-core` instead of shell scaffolds.
- `backend-oauth-exchange` merge archived the slice but left the plan entry `in-progress`; recovered with `specify plan transition backend-oauth-exchange done` after merge had already succeeded.
- `wasm32-wasip2` release build hit `omnia-sdk`/toolchain conflicts; native `cargo test` passed for backend crates.
- `mobile-oauth-signin` core-only build skipped `composition.yaml`; finalize emitted non-blocking `composition-empty-for-ui-slice` warning.
- Plan lock held via Python `fcntl` fallback (stock macOS lacks `flock(1)`).

## Notes

- Scenario exercises workspace `/spec:execute` only — no `/spec:finalize` invocation in this run.
- RFC-45 slot adapter provisioning exercised via `workspace sync` mirror into slot manifest caches.

## Evidence

- **Reproduce:** `scripts/snapshot.sh evals/.sandbox/workspace-two-projects`
- **Retained at:** `evals/.sandbox/workspace-two-projects/`
- **Key paths:** `platform/{plan.yaml,change.md,discovery.md,registry.yaml,.specify/journal.jsonl}`, `platform/.specify/workspace/` (symlink slots), per-project `.specify/archive/` (archived slices incl. `build/report.yaml`)
