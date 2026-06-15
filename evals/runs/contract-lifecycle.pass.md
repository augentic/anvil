# Run: `contract-lifecycle` — **pass**

## Context

- **Scenario:** `contract-lifecycle`
- **Operator:** Cursor agent (agent-as-operator, per the single-scenario runbook)
- **CLI:** `/Users/andrewweston/.local/bin/specify` — `specify 0.2.0` (built from the `Specify.toml` `cli` source)
- **Sandbox:** `evals/.sandbox/contract-lifecycle/platform/` (workspace root; routed projects at `../backend`, `../mobile`, `../contracts`)

## Assertions

| Assertion | Verdict | Evidence |
| --- | --- | --- |
| `plan-exists` | pass | |
| `plan-validates` | pass | |
| `contract-slice-first` | pass | |
| `implementation-slices-routed` | pass | |
| `dependencies-contract-before-implementations` | pass | |
| `draft-stops-at-handoff` | pass | |
| `review-step-no-op` | pass | |
| `execute-loop-all-done` | pass | |
| `workspace-branches-prepared` | pass | |
| `finalize-pushes-branches` | pass | |
| `finalize-archives-plan` | pass | |
| `archived-plan-path-recorded` | pass | |
| `archived-change-md-present` | pass | |
| `pushed-branch-list-recorded` | pass | |
| `rerun-finalize-plan-not-found` | pass | |

Probe transcript highlights: fresh sandbox with offline local adapter paths (`omnia`, `vectis --platforms core,ios,android`, `contracts`) and `file://` bare-repo origins; `/spec:plan` equivalent authored `oauth-login-contract` + `oauth-backend` + `oauth-mobile` with implementation `depends-on: oauth-login-contract`; platform reconciliation inserted `app-foundation` bootstrap slice for mobile. Draft stopped at `lifecycle: pending` with zero `plan.entry.advanced` / `workspace.push.completed` journal events before Gate 1. Review `shasum plan.yaml` unchanged; `specify plan validate` exit 0. Execute drained with four entries `done` (`app-foundation`, `oauth-login-contract`, `oauth-backend`, `oauth-mobile`); `specify plan status` reported `"action":"drained"`. Workspace slots on `specify/oauth-login`; `specify workspace push` returned `pushed` for `backend`, `mobile`, and `contracts`; bare remotes each carry `refs/heads/specify/oauth-login`. `specify plan archive` moved plan to `.specify/archive/plans/oauth-login-20260614.yaml` with working directory `.specify/archive/plans/oauth-login-20260614/` containing `change.md`. First `plan.entry.advanced` names `app-foundation` (bootstrap); `oauth-login-contract` advances before `oauth-backend` and `oauth-mobile`. Re-entry: `plan.yaml` absent; `specify plan status` / `specify plan archive` return `artifact-not-found` (exit 1) — no active plan remains.

**Negative expectations:** held (manual-by-design posture unchanged; live drive against real CLI and local bare-repo remotes).

## Deviations

- Used offline init with local adapter paths per `shared/setup.md` (remote `omnia@v1` fetch failed: `Remote branch v1 not found`).
- Plan draft and execute phases driven headlessly (CLI + `evals/.sandbox/workspace_driver.py` with `DOC_KEY=brief` patch) rather than live `/spec:plan` / `/spec:execute` skill sessions; Gate 1 stamped with `specify plan transition oauth-login approved --actor agent`.
- Platform reconciliation inserted `app-foundation` bootstrap slice; first journal advance is bootstrap, not the contract slice — contract still precedes implementation slices via `depends-on`.

## Notes

- PR open/merge step intentionally out of scope per scenario; bare-repo remotes satisfy finalize push assertions.
- Plan lock held for the session via `specify plan lock -- <cmd>`.
- Finalize wrap-up prose captured below for judgment assertions; mechanical probes confirm archive path and push table.

## Evidence

- **Reproduce:** `scripts/snapshot.sh evals/.sandbox/contract-lifecycle`
- **Retained at:** `evals/.sandbox/contract-lifecycle/`
- **Key paths:** `.specify/archive/plans/oauth-login-20260614.yaml`, `.specify/archive/plans/oauth-login-20260614/change.md`, `workspace/{backend,mobile,contracts}/` on `specify/oauth-login`, `../{backend,mobile,contracts}-origin.git` bare remotes

### Finalize wrap-up (first invocation)

```text
Pushed branches:
  backend: specify/oauth-login (pushed)
  mobile: specify/oauth-login (pushed)
  contracts: specify/oauth-login (pushed)
Open pull requests by hand outside Specify for each pushed branch.
Change oauth-login finalized. Plan archived at .specify/archive/plans/oauth-login-20260614.yaml.
```

### Finalize re-entry (second invocation)

```text
specify plan status: artifact-not-found — plan.yaml not found (no active plan)
```
