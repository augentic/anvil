# Run: `workspace-execute-two-projects` — **pass**

## Context

- **Scenario:** `workspace-execute-two-projects`
- **Operator:** Cursor agent (agent-as-operator, per the single-scenario runbook; operator seams driven at the operator's standing direction)
- **CLI:** `/Users/andrewweston/.local/bin/specify` — `specify 0.2.0` (built from the `Specify.local.toml` `cli = { path = "../specify-cli" }` source via `make install-cli`, including the RFC-45 slot-adapter-provisioning mirror and the rebuilt vectis dist)
- **Sandbox:** `evals/.sandbox/workspace-execute-two-projects/` (workspace root `shop-platform/`, peers `shop-backend/`, `shop-mobile/`, local bare remotes under `remotes/`)

## Assertions

| Assertion | Verdict | Evidence |
| --- | --- | --- |
| `plan-exists` | pass | |
| `per-slice-project-routing` | pass | |
| `slots-materialised` | pass | |
| `plan-lock-at-workspace` | pass | |
| `execute-loop-all-done` | pass | |

Probe transcript highlights: `plan.yaml` at the workspace root with `lifecycle: approved` before execute (Gate 1 stamped `--actor agent`). Four slices, each `status: done` (4/4), `specify plan status` reporting `drained` with `resume: /spec:finalize oauth-login`, and exactly four `plan.entry.advanced` events in the workspace journal — none in any slot. Both slots materialised at `.specify/workspace/{shop-backend,shop-mobile}` (local symlink slots per the local-peer posture); `workspace.sync.completed` payloads list both projects. Per-slice routing: `bootstrap-core` + `mobile-oauth-signin` merge/residue commits land in the shop-mobile slot, `oauth-contract` + `backend-oauth-exchange` in the shop-backend slot. No slot grew a `plan.yaml`; after the driver released the lock, `specify plan next` refused with `plan-lock-not-held` (exit 2), confirming the runtime lock enforcement at the workspace.

**RFC-45 R4 live evidence (no manual cache-stage):** `workspace sync` provisioned each slot's manifest cache with the workspace's adapter set — `shop-backend/.specify/cache/manifests/sources/documentation` + `targets/{omnia,vectis}` and `shop-mobile/.specify/cache/manifests/sources/documentation` + `targets/vectis` — and every slot-side `specify source extract` / `specify target resolve` resolved against those mirrored roots.

**Negative expectations:** held (manual-by-design posture unchanged; live interactive drive against the real CLI, real git remotes, and real cargo workspaces).

## Deviations

- `shop-mobile` initialised with `--platforms core` only: the host lacks the iOS/Android toolchains (xcodegen, gradle, kotlinc, cargo-swift, Java), so the vectis build ran core-only. Platform reconciliation inserted `bootstrap-core` instead of `app-foundation` shell scaffolds; the core-only composition skip in `vectis validate` was exercised live.
- The vectis `tools.yaml` sidecar was copied into the mirrored manifest cache by hand: the locally built dist's sidecar generation runs at `make use-local-dev`, which the slot mirror does not re-run. Mirror provenance itself needed no staging (R1 evidence above).
- `flock(1)` absent on this macOS host; the plan lock was held for the driver session with `plan-lock.md`'s zsh `zsystem flock` fallback (a long-lived background holder), which `require_held` observed correctly across every plan-state write.
- The shop-backend slot worked directly on `main` (its scaffold default); shop-mobile worked on `specify/oauth-login`. Both pushed to their bare remotes; branch naming is outside the scenario's assertions.
- Build-phase code review run by a single agent walking the review categories sequentially (REVIEW.md per the output template), not as concurrently spawned specialist subagents.

## Notes

- The backend OAuth slices target omnia-sdk 0.33.0; the `wasm32-wasip2` release build of the full shop-backend workspace is the definitive gate and passed for both backend slices. The 401 contract rows required a guest-level error wrapper (the four-variant `omnia_sdk::Error` has no Unauthorized variant) — accepted in REVIEW.md as the documented seam.
- The mobile core models the backend exchange as a shell-driven IO seam (`Exchanging` state + `ExchangeSucceeded` / `ExchangeFailed` events) because the platform set is core-only; a future shell slice binds the real HTTP capability.

## Evidence

- **Reproduce:** `scripts/snapshot.sh evals/.sandbox/workspace-execute-two-projects`
- **Retained at:** `evals/.sandbox/workspace-execute-two-projects/`
- **Key paths:** `shop-platform/{plan.yaml,change.md,discovery.md,registry.yaml,.specify/journal.jsonl}`, `shop-platform/.specify/workspace/` (symlink slots), `shop-backend/.specify/cache/manifests/` + `shop-mobile/.specify/cache/manifests/` (RFC-45 mirrored adapter roots), `shop-backend/crates/{oauth_contract,backend_oauth_exchange}/`, `shop-backend/src/lib.rs` (guest), `shop-mobile/shared/src/app.rs`, per-project `.specify/archive/2026-06-11-*/` (archived slices incl. `build/report.yaml`)
