# Run: `workspace-fail-resume` — **pass**

## Context

- **Scenario:** `workspace-fail-resume`
- **Operator:** Cursor agent (agent-as-operator, per the single-scenario runbook)
- **CLI:** `/Users/andrewweston/.local/bin/specify` — `specify 0.2.0` (built from the `Specify.toml` `cli` source via `make install-cli`)
- **Sandbox:** `evals/.sandbox/workspace-fail-resume/` (`platform/`, `backend/`, `mobile/`)

## Assertions

| Assertion | Verdict | Evidence |
| --- | --- | --- |
| `plan-exists` | pass | |
| `breakout-routes-to-slot` | pass | |
| `active-slice-resolved-across-boundary` | pass | |
| `chdir-without-operator-intervention` | pass | |
| `execute-loop-all-done` | pass | |

Probe transcript highlights: `plan.yaml` present with `lifecycle: approved` before execute; `/spec:execute` parked on `auth-rotate` with `specify plan status` reporting `action: stop`, `stop.reason: build-failed`, one `status: in-progress` entry; breakout emitted `Routing: auth-rotate → backend (.specify/workspace/backend/)` while CWD stayed at the workspace root; backend slot journal records `slice.build.failed` then breakout `slice.build.succeeded` for `auth-rotate`; resumed loop merged `auth-rotate`, completed `oauth-mobile`, and `specify plan status` reports `action: drained` with two `status: done` entries; `specify plan next` after lock release returns `reason: drained` (exit 0 via `plan status`).

**Negative expectations:** held (manual-by-design posture unchanged; the run was driven interactively against the real CLI).

## Deviations

- `specify init omnia@v1` / `vectis@v1` failed (`adapter-git-failed: Remote branch v1 not found`); used local adapter paths per the documented offline fallback in `shared/setup.md`.
- Pre-scaffolded minimal `shared/src/app.rs`, `iOS/App.swift`, and `Android/.../App.kt` in `mobile` before propose to avoid the `--reconcile-platforms` `app-foundation` bootstrap slice (two-slice plan: `auth-rotate`, `oauth-mobile`).
- Initialized bare `origin` remotes in `backend` and `mobile` so `specify workspace prepare` could create `specify/oauth-login` branches.
- OAuth brief trimmed to backend/mobile implementation sections only (two surveyed leads) to satisfy `plan-reconcile-partition` without a separate contract slice.
- Plan lock acquired via Python `fcntl` fallback on stock macOS (same posture as `intent-only.pass.md`).
- Gate 1 stamped with `--actor agent`; build failure injected via `session_cookie_secure_flag_set` test in `crates/auth_rotate` (fixed before workspace `/spec:build` breakout, per fixture #11).

## Notes

- Park state captured at `specify plan status`: `next-action: stop build-failed`, `resume: /spec:build auth-rotate`, entry `auth-rotate` `in-progress` / slice lifecycle `refined`.
- Breakout `/spec:build` from `platform/` resolved the active slice without `--project` or manual `chdir`; merge remained the sole writer of per-entry `done`.
- `specify slice validate` on both slices returned non-blocking `kind: review` suggestions only.

## Evidence

- **Reproduce:** `scripts/snapshot.sh evals/.sandbox/workspace-fail-resume/platform`
- **Retained at:** `evals/.sandbox/workspace-fail-resume/`
- **Key paths:** `platform/plan.yaml`, `platform/registry.yaml`, `platform/.specify/workspace/{backend,mobile}/`, `backend/.specify/journal.jsonl` (`slice.build.failed` / `slice.build.succeeded` for `auth-rotate`), `platform/.specify/journal.jsonl` (`plan.entry.advanced`, `plan.transition.approved`)
