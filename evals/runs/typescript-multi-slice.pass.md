# Run: `typescript-multi-slice` — **pass**

## Context

- **Scenario:** `typescript-multi-slice`
- **Operator:** Cursor agent (agent-as-operator, per the single-scenario runbook)
- **CLI:** `/Users/andrewweston/.local/bin/specify` — `specify 0.2.0`
- **Sandbox:** `evals/.sandbox/typescript-multi-slice/`

## Assertions

| Assertion | Verdict | Evidence |
| --- | --- | --- |
| `plan-exists` | pass | |
| `plan-validates` | pass | |
| `multiple-slices-from-code` | pass | |
| `sources-legacy-only` | pass | |
| `no-under-slicing` | pass | |

Probe transcript highlights: `plan.yaml` present with `lifecycle: pending`; `specify plan validate --format json` exits 0 with zero findings; `plan.reconcile.completed` payload reads `"slice-count":5`; `grep 'source: ' plan.yaml | sort -u` yields only `legacy`; zero `plan.transition.approved` events (Gate 1 not stamped).

**Judgment (`no-under-slicing`):** Five surveyed Express surfaces in `discovery.md` — `POST /users`, `GET /users/:id`, `POST /auth/login`, `POST /auth/refresh`, `POST /auth/reset-password` — each landed in its own slice (`user-registration`, `user-lookup`, `auth-login`, `auth-refresh`, `password-reset`) rather than a single catch-all migration slice.

**Negative expectations:** held (manual-by-design posture unchanged; the run was driven interactively against the real CLI).

## Deviations

- Offline init via `specify init $FRAMEWORK/adapters/targets/omnia` (local adapter path) instead of `omnia@v1` network fetch.
- Symlinked `adapters/sources/typescript` from the framework checkout per setup prerequisites.
- Copied `evals/fixtures/sources/typescript/source` to `./legacy-monolith`, then extended it with four additional Express routes and a shared `policy-engine` module so union production LOC exceeds the survey brief's 1000-line threshold and per-surface leads are emitted (the bare fixture is a single-route service under the threshold).
- Binding command: `specify plan create legacy-port --source legacy=typescript:./legacy-monolith` (source key `legacy`, path `./legacy-monolith`).
- Phase work driven by following the `/spec:plan` skill body via CLI verbs (`source survey`, `plan propose --from`) rather than invoking the Cursor slash command directly.
- Stopped at Gate 1 without stamping `approved`, per scenario invocation.

## Notes

- `auth-refresh` carries `depends-on: [auth-login]` because refresh semantics require login-issued sessions; the other four slices remain independent.
- The fixture README still documents the degenerate single-lead shape for extract fixtures; the eval run intentionally extends the copied tree for multi-surface survey behavior.

## Evidence

- **Reproduce:** `scripts/snapshot.sh evals/.sandbox/typescript-multi-slice`
- **Retained at:** `evals/.sandbox/typescript-multi-slice/`
- **Key paths:** `plan.yaml`, `change.md`, `discovery.md`, `legacy-monolith/src/server.ts`, `.specify/journal.jsonl`

## Plan structure

| Slice | Project | Sources | Status |
| --- | --- | --- | --- |
| user-registration | project | legacy / user-registration | pending |
| user-lookup | project | legacy / user-lookup | pending |
| auth-login | project | legacy / auth-login | pending |
| auth-refresh | project | legacy / auth-refresh | pending |
| password-reset | project | legacy / password-reset | pending |
