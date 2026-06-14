# Run: `workspace-two-projects` — **pass**

## Context

- **Scenario:** `workspace-two-projects`
- **Operator:** Cursor agent (agent-as-operator, per the single-scenario runbook)
- **CLI:** `/Users/andrewweston/.local/bin/specify` — `specify 0.2.0` (built from the `Specify.toml` `cli` source via `make install-cli`)
- **Sandbox:** `evals/.sandbox/workspace-two-projects/` (`platform/`, `backend/`, `mobile/`)

## Assertions

| Assertion | Verdict | Evidence |
| --- | --- | --- |
| `plan-exists` | pass | |
| `per-slice-project-routing` | pass | |
| `slots-materialised` | pass | |
| `plan-lock-at-workspace` | pass | |
| `execute-loop-all-done` | pass | |

Probe transcript highlights: `plan.yaml` present with `lifecycle: approved` before execute; `specify plan status --format json` reports `"action":"drained"` with `"counts":{"done":4}`; `grep -c 'status: done' plan.yaml` returns 4; `grep 'project:' plan.yaml` names `backend` for `oauth-contract`/`oauth-backend` and `mobile` for `app-foundation`/`oauth-mobile`; `git -C workspace/backend log --oneline specify/oauth-login` shows residue commits for `oauth-contract` and `oauth-backend`; `git -C workspace/mobile log --oneline specify/oauth-login` shows residue commits for `app-foundation` and `oauth-mobile`; `test -d workspace/{backend,mobile}` succeeds; first `workspace.sync.completed` payload lists `"projects":["backend","mobile"]`; `plan.yaml` lives at workspace root with no slot `plan.yaml`; unlocked `specify plan next` returns `"error":"plan-lock-not-held"` (exit 2).

**Negative expectations:** held (manual-by-design posture unchanged; the run was driven interactively against the real CLI).

## Deviations

- Used offline init with local adapter paths (`specify init <framework>/adapters/targets/{omnia,vectis}`) per documented offline fallback; `omnia@v1` / `vectis@v1` shorthand was not used.
- Symlinked the `documentation` source adapter into the workspace (`adapters/sources/documentation`) per setup prerequisite.
- Initialized git repos in `backend/` and `mobile/` with `file://` bare remotes (`backend-origin.git`, `mobile-origin.git`) so `specify workspace prepare` could resolve `origin/HEAD` — required after sandbox init, not spelled out in `shared/setup.md`.
- Gate 1 stamped with `specify plan transition oauth-login approved --actor agent`.
- Execute driven by agent following `/spec:execute` routing (workspace sync/prepare, `SPECIFY_PLAN_DIR`, refine → build → merge per slice) with a local `_drive.zsh` helper for omnia/vectis build stubs; finalize not run (execute-only per scenario stages).
- Stale `platform/.specify/plan.lock` from an interrupted driver removed before the completing pass (`rm -f .specify/plan.lock`).
- Backend workspace `Cargo.toml` lists both `crates/oauth_contract` and `crates/oauth_backend` as members; omnia eval test harness uses `HeaderMap<String>::with_capacity(0)` to satisfy `omnia-sdk 0.33` `Context` typing.
- First `oauth-contract` residue commit accidentally tracked `target/`; follow-up commit removed it from git and added `/target/` to `.gitignore`.

## Notes

- `specify plan next --format json` after drain (lock released) returns `plan-lock-not-held` (exit 2) rather than a drained payload — `specify plan status` is the authoritative drained signal post-execute; consistent with `plan-lock-at-workspace`.
- Journal `plan.entry.advanced` shows two events (`app-foundation`, `oauth-mobile`) because `oauth-contract` and `oauth-backend` were already `in-progress` from an earlier partial drive before the completing pass; all four entries reached `done` and the scheduler reports drained.
- `cargo test` in `backend/` passes for `oauth_backend` integration test (`handler_works`).

## Evidence

- **Reproduce:** `scripts/snapshot.sh evals/.sandbox/workspace-two-projects`
- **Retained at:** `evals/.sandbox/workspace-two-projects/`
- **Key paths:** `platform/plan.yaml`, `platform/change.md`, `platform/discovery.md`, `platform/registry.yaml`, `platform/workspace/{backend,mobile}`, `platform/.specify/journal.jsonl`, `backend/.specify/specs/`, `mobile/.specify/specs/composition.yaml`
