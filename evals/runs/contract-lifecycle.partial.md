# Run: `contract-lifecycle` — **partial**

## Context

- **Scenario:** `contract-lifecycle`
- **Operator:** Cursor agent (agent-as-operator, per the single-scenario runbook)
- **CLI:** `/Users/andrewweston/.local/bin/specify` — `specify 0.2.0` (built from `Specify.local.toml` → `../specify-cli` via `make install-cli`)
- **Sandbox:** `evals/.sandbox/contract-lifecycle/` (`platform/`, `backend/`, `mobile/`, `contracts/`)

## Assertions

| Assertion | Verdict | Evidence |
| --- | --- | --- |
| `plan-exists` | pass | `platform/plan.yaml` present after plan create + propose |
| `plan-validates` | pass | `specify plan validate` summary all zeros after propose |
| `contract-slice-first` | skipped | Execute not started; note `app-foundation` is prepended by `--reconcile-platforms` and would advance before `oauth-login-contract` in list order |
| `implementation-slices-routed` | pass | `grep -c 'project: backend' plan.yaml` → 1; `grep -c 'project: mobile' plan.yaml` → 2 (includes `app-foundation` bootstrap) |
| `dependencies-contract-before-implementations` | pass | `oauth-backend` / `oauth-mobile` `depends-on` include `oauth-login-contract` |
| `draft-stops-at-handoff` | pass | Plan left at `lifecycle: pending` until explicit `specify plan transition oauth-login approved` |
| `review-step-no-op` | pass | `specify plan validate` clean after draft; no `plan amend` |
| `execute-loop-all-done` | skipped | Execute not driven |
| `workspace-branches-prepared` | skipped | Execute/finalize not driven |
| `finalize-halts-on-unmerged-prs` | skipped | Finalize not driven |
| `finalize-archives-plan` | skipped | Finalize not driven |
| `archived-plan-path-recorded` | skipped | Finalize not driven |
| `archived-change-md-present` | skipped | Finalize not driven |
| `merged-pr-list-recorded` | skipped | Finalize not driven |
| `rerun-finalize-plan-not-found` | skipped | Finalize not driven |

**Negative expectations:** held (no automated runner, fake forge, transcript replay, CI target, or golden comparison added).

## Deviations

- Used offline init with local adapter paths (`specify init <framework>/adapters/targets/{omnia,vectis,contracts}`) because `omnia@v1` / `vectis@v1` GitHub refs failed (`Remote branch v1 not found`).
- Added a fourth sandbox root `contracts/` and registered `specify registry add contracts --url ../contracts --adapter contracts` — required so `oauth-login-contract` can bind a `project` (propose rejects omitting `project` when the registry declares more than one routed implementation project).
- Symlinked `adapters/sources/documentation` into the workspace per setup prerequisite.
- Initialized `file://` bare remotes for `backend/` and `mobile/` (`backend-origin.git`, `mobile-origin.git`) so `specify workspace prepare` can resolve `origin/HEAD`; not sufficient for `/spec:finalize` `gh pr view` (needs real GitHub remotes).
- Gate 1 stamped with `specify plan transition oauth-login approved --actor agent`.
- Execute and finalize not completed in this session.

## Notes

- Plan has four slices after platform reconciliation: `app-foundation`, `oauth-login-contract`, `oauth-backend`, `oauth-mobile`.
- `specify plan status` reports next action `refine app-foundation` (bootstrap prepended ahead of contract slice in plan list order).
- Resume execute from `platform/` with plan lock acquired per `plugins/spec/references/plan-lock.md`, then `/spec:execute` or per-slice breakouts.
- Finalize requires GitHub `origin` remotes on `backend` and `mobile`, then operator-owned PR merges between the first and second `/spec:finalize oauth-login` invocations.

## Evidence

- **Reproduce:** `scripts/snapshot.sh evals/.sandbox/contract-lifecycle`
- **Retained at:** `evals/.sandbox/contract-lifecycle/`
- **Key paths:** `platform/plan.yaml`, `platform/change.md`, `platform/discovery.md`, `platform/registry.yaml`, `platform/docs/oauth-login.md`
