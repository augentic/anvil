# Scenario run summary

## Run header

- **Scenario id:** `dual-driving-refused`
- **Scenario file:** `acceptance/lifecycle/12-dual-driving-refused.md`
- **Backend:** `manual`
- **Operator / agent:** Cursor agent (Claude Opus 4.8)
- **Run id:** `dual-driving-refused-2026-06-06`
- **Started at / finished at:** `2026-06-05T18:59Z` / `2026-06-05T19:01Z`
- **`specify` build:** `/Users/andrewweston/.local/bin/specify` → `specify 0.3.0`
- **Workspace / project roots:** `/private/tmp/specify-acceptance-dual.6kYDWl/{shop-platform,shop-backend,shop-mobile}` (disposable temp)

## Inputs created

- `shop-platform/.specify/...` + `registry.yaml` — created (`specify init --workspace`, `specify registry add` ×2)
- `shop-backend/.specify/...` — created (`git init`, `specify init omnia@v1`)
- `shop-mobile/.specify/...` — created (`git init`, `specify init vectis@v1 --platforms core,ios,android`)
- `shop-platform/plan.yaml` — created + approved (workspace plan `oauth-login` routing `oauth-login` → `shop-backend`)

## Invocation

### Plan (workspace, setup)

```text
cd shop-platform
specify plan create oauth-login --source intent=intent:value:add OAuth token exchange to the backend
specify source survey intent (prepare/finalize)        # 1 lead oauth-login
specify plan propose --from proposal-response.json --reconcile-platforms   # slices: app-foundation (shop-mobile), oauth-login (shop-backend)
specify plan transition oauth-login approved           # pending -> approved
specify workspace sync                                  # slots materialised; topology.lock written
```

### Attempt project-root plan (the assertion under test)

```text
cd shop-backend
specify plan create local-change --source intent=intent:value:unrelated local change
  → EXIT 0; wrote shop-backend/plan.yaml (lifecycle: pending). NOT refused.
```

## Plan structure (workspace plan)

| Role | Slice | Project | Depends on | Sources | Status |
| --- | --- | --- | --- | --- | --- |
| implementation | `app-foundation` | `shop-mobile` | none | (bootstrap) | pending |
| implementation | `oauth-login` | `shop-backend` | none | `intent` | approved-plan / pending entry |

## Expected artifacts and state

| Item | Status | Notes |
| --- | --- | --- |
| `registry.yaml` | `present` | 2 projects; `registry validate` ok |
| `shop-platform/plan.yaml` | `present (approved)` | workspace plan active, routes to shop-backend |
| `shop-backend/plan.yaml` | `present (UNEXPECTED)` | created by the project-root `plan create` that should have been refused |

## Assertions

| Assertion id | Verdict | Evidence pointer |
| --- | --- | --- |
| `workspace-plan-active` | `pass` | `shop-platform/plan.yaml` `lifecycle: approved`; slice `oauth-login` → `project: shop-backend` |
| `plan-from-project-refused` | `fail` | `specify plan create local-change` in `shop-backend/` exited 0 and wrote `shop-backend/plan.yaml`; no structured error |
| `one-driving-mode-per-project` | `fail` | no refusal raised; no error cited the one-driving-mode-per-project invariant |

## Negative expectations

| Negative expectation | Verdict | Notes |
| --- | --- | --- |
| `automated-runner-added` | `held` | real CLI only |
| `fake-forge-added` | `held` | no forge used |
| `transcript-replay-added` | `held` | |
| `ci-target-added` | `held` | |
| `golden-output-required` | `held` | structural grading only |

## Command output

- `specify plan create local-change` (in shop-backend): exit 0, wrote `name: local-change / lifecycle: pending`.
- `specify plan create --help`: no `workspace` / `driving` / `refuse` guard documented.

## Cleanup

- **Workspaces / projects:** retained (disposable temp dir) for triage.
- **Run evidence:** this file; preserved temp root `/private/tmp/specify-acceptance-dual.6kYDWl`.

## Verdict

- **Result:** `fail`
- **Fault domain on failure:** `plan` (CLI `specify plan create` — missing dual-driving guard).
- **Notes:** The one-driving-mode-per-project invariant (plan SKILL §Guardrails "Single-driving-mode per project"; `AGENTS.md`; `DECISIONS.md`) is **not enforced** by `specify 0.3.0`. `specify plan create` from a registered project's own root silently created a competing local plan while a workspace plan was active and routing a slice to that project. Architectural observation: the registered project (`shop-backend/.specify/`) carries **no back-reference** to the owning workspace or its active plan (project.yaml has no workspace marker), so `plan create` running in the project has no signal to detect dual-driving — strongly suggesting the guard is unimplemented (or depends on a sync-written marker that is not being written). Wave-2 failure: recorded and triaged; does not halt the sweep. Recommend a follow-up issue in `augentic/specify-cli`.
