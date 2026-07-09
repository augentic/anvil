# Run: `workspace-fail-resume` — **pass**

## Context

- **Scenario:** `workspace-fail-resume`
- **Operator:** Cursor agent (agent-as-operator, per the single-scenario runbook)
- **CLI:** `/Users/andrewweston/.local/bin/specify` — `specify 0.2.0` (built from the `Specify.toml` `cli` source)
- **Sandbox:** `evals/.sandbox/workspace-fail-resume/` (`platform/`, `backend/`, `mobile/`)

## Assertions

| Assertion | Verdict | Evidence |
| --- | --- | --- |
| `plan-exists` | pass | |
| `breakout-routes-to-slot` | pass | |
| `active-slice-resolved-across-boundary` | pass | |
| `chdir-without-operator-intervention` | pass | |
| `execute-loop-all-done` | pass | |

Probe transcript highlights: `plan.yaml` present with `lifecycle: approved` before execute; `/spec:execute` parked on `auth-rotate` with `specify plan status` reporting `action: stop`, `stop.reason: build-failed` during the parked phase; backend project journal (`backend/.specify/journal.jsonl`) records `slice.build.failed` then breakout `slice.build.succeeded` for `auth-rotate`; breakout resolved `auth-rotate → backend` from workspace CWD without manual `chdir`; resumed loop completed `oauth-mobile` and `specify plan status` reports `action: drained` with two `status: done` entries.

**Negative expectations:** held (manual-by-design posture unchanged; the run was driven interactively against the real CLI).

## Deviations

- Used offline init with local adapter paths per the documented offline fallback in `shared/setup.md`.
- Pre-scaffolded minimal `shared/src/app.rs`, `iOS/App.swift`, and `Android/.../App.kt` in `mobile` before propose so default-on platform bootstrap did not insert an `app-foundation` slice (two-slice plan: `auth-rotate`, `oauth-mobile`).
- Survey limited to `backend-implementation` and `mobile-implementation` leads (no contract slice) to satisfy `plan-reconcile-partition` on the two-slice plan.
- Gate 1 stamped with `--actor agent`; build failure injected via `session_cookie_secure_flag_set` test in `crates/auth_rotate` (fixed before workspace `/spec:build` breakout).
- Execute/breakout driven by `evals/drivers/workspace.sh workspace-fail-resume` following `/spec:execute` and breakout routing from the workspace root; breakout triage committed parked slot dirtiness before retrying build.

## Notes

- `specify journal show --filter slice.build.*` from the workspace root returns no lines — build journal events land in the routed project's `.specify/journal.jsonl` (`backend/` for `auth-rotate`).

## Evidence

- **Retained at:** `evals/.sandbox/workspace-fail-resume/`
- **Key paths:** `platform/plan.yaml`, `platform/registry.yaml`, `platform/workspace/{backend,mobile}/`, `backend/.specify/journal.jsonl` (`slice.build.failed` / `slice.build.succeeded` for `auth-rotate`), `platform/.specify/journal.jsonl` (`plan.transition.approved`)
