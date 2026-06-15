# Run: `lead-reconciliation` — **pass**

## Context

- **Scenario:** `lead-reconciliation`
- **Operator:** Cursor agent (agent-as-operator, per the single-scenario runbook)
- **CLI:** `/Users/andrewweston/.local/bin/specify` — `specify 0.2.0`
- **Sandbox:** `evals/.sandbox/lead-reconciliation/`

## Assertions

| Assertion | Verdict | Evidence |
| --- | --- | --- |
| `plan-exists` | pass | |
| `plan-validates` | pass | |
| `merged-slice-combines-sources` | pass | |
| `tentative-merge-surfaced` | pass | |
| `amend-overrides-merge` | pass | |
| `extract-runs-per-contributing-source` | pass | |

Probe transcript highlights: `plan.yaml` present with `lifecycle: pending` at Gate 1 (later stamped `approved` for refine); `specify plan validate --format json` exits 0 before and after amend; `plan.reconcile.completed` payload reads `"slice-count":1` with slice `account-lockout` carrying `sources: [product-notes/password-reset, legacy-monolith/account-pwd-reset]`; `change.md` lists the cross-name pairing under `## Tentative merges`; two `slice.extract.completed` journal events name `(account-lockout, product-notes)` and `(account-lockout, legacy-monolith)`; evidence dir holds `product-notes.yaml` and `legacy-monolith.yaml`; `slice.transition.refined` fired for `account-lockout`.

**Judgment (`tentative-merge-surfaced`):** `change.md` documents the `password-reset` ↔ `account-pwd-reset` pairing under `## Tentative merges` because the leads share intent but differ in slug and synopsis (30-minute docs expiry vs 24-hour legacy TTL constant).

**Negative expectations:** held (manual-by-design posture unchanged; the run was driven interactively against the real CLI).

## Deviations

- Offline init via `specify init $FRAMEWORK/adapters/targets/omnia` (local adapter path) instead of `omnia@1.0.0` network fetch.
- Symlinked `adapters/sources/documentation` and `adapters/sources/typescript` from the framework checkout per setup prerequisites.
- Docs binding uses only `password-reset.md` under `./docs` (copied from `evals/fixtures/sources/documentation/input/password-reset.md`); source key `product-notes`.
- TypeScript binding copies `evals/fixtures/sources/typescript/source` to `./legacy-monolith` and adds `POST /auth/reset-password` handler at `src/auth/reset-password.ts` describing the same lockout/reset behaviour; survey lead `account-pwd-reset`.
- Phase work driven by following the `/spec:plan` and `/spec:refine` skill bodies via CLI verbs (`source survey`, `plan propose --from`, `source extract`, `slice synthesize --from`) rather than invoking Cursor slash commands directly.
- Gate 1 stamped `approved` with `--actor agent`; plan lock held for the session via `specify plan lock -- <cmd>`.

## Notes

- Amend commands (split then rebind): `specify plan amend account-lockout --sources product-notes=password-reset`, then `specify plan amend account-lockout --sources product-notes=password-reset legacy-monolith=account-pwd-reset`.
- Synthesis surfaced `[divergence]` on REQ-002 (30-minute documentation expiry vs 24-hour legacy TTL); slice validated cleanly (review-only findings) and transitioned to `refined`.
- Scenario stops after refine; no build/merge/execute loop.

## Evidence

- **Reproduce:** `scripts/snapshot.sh evals/.sandbox/lead-reconciliation`
- **Retained at:** `evals/.sandbox/lead-reconciliation/`
- **Key paths:** `plan.yaml`, `change.md`, `discovery.md`, `docs/password-reset.md`, `legacy-monolith/src/auth/reset-password.ts`, `.specify/slices/account-lockout/evidence/`, `.specify/journal.jsonl`

## Plan structure

| Slice | Project | Sources | Status |
| --- | --- | --- | --- |
| account-lockout | project | product-notes / password-reset, legacy-monolith / account-pwd-reset | in-progress (refined) |
