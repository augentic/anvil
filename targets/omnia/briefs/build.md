---
id: build
description: Drive Omnia crate / test / guest generation for the active slice, then run code review. Consumed by `/spec:build` once the slice is `refined`; carries the bodies of the retired `omnia-{crate,test,guest}-writer` and `omnia-code-reviewer` skills, sequenced as a single linear orchestration.
---

# Omnia target — build brief

> `/spec:build` loads this brief when it walks an `in-progress` plan entry whose slice has `target: omnia`. The brief carries the agent-side body of the retired `omnia-crate-writer`, `omnia-test-writer`, `omnia-guest-writer`, and `omnia-code-reviewer` skills. Read it linearly; do not invoke the retired skills directly. Synthesis idioms (provider DI, WASM guardrails, error variants, validation placement) live in [`shape.md`](shape.md) and must already be reflected in the slice's `spec.md` + `design.md` before this brief runs.

## Inputs and bindings

```text
$SLICE_NAME   = active in-progress plan entry's slice name (from `specify plan next`)
$SLICE_DIR    = .specify/slices/$SLICE_NAME
$SPEC_PATH    = $SLICE_DIR/spec.md
$DESIGN_PATH  = $SLICE_DIR/design.md
$TASKS_PATH   = $SLICE_DIR/tasks.md
$CRATE_NAME   = $SLICE_NAME with kebab → snake (or the slice's plan-level `crate:` override)
$CRATE_PATH   = crates/$CRATE_NAME
$GUEST_PATH   = workspace root (single `src/lib.rs` exports HTTP / Messaging / WebSocket guests)
$REVIEW_OUTPUT = $CRATE_PATH/REVIEW.md
```

`/spec:build` resolves `$SLICE_NAME` from `specify plan next`. The brief uses that name throughout.

## Mode detection

Check whether `$CRATE_PATH/Cargo.toml` exists:

- **Missing** → **create mode**: generate the crate, tests, and (if `src/lib.rs` is absent at the guest root) guest scaffolding.
- **Present** → **update mode**: incremental change against the existing crate; guest wiring updates are folded into the crate-writer step (do not re-run the guest scaffolding pass).

## Critical path

1. Read [`shape.md`](shape.md) refresher and the slice's `spec.md` + `design.md` + `tasks.md`.
2. Generate or update the crate (§ Crate writer).
3. Generate or update tests (§ Test writer).
4. Generate the guest project on first build only (§ Guest writer); update path mirrors crate-writer's four-category cadence.
5. Run the verify-repair loop (§ Verify-repair loop) — `cargo fmt`, `cargo check`, `cargo clippy -- -D warnings`, `cargo test`.
6. Run code review (§ Code reviewer) and process findings.
7. Mark `tasks.md` checkboxes complete as each task lands; the slice transitions to `built` by `/spec:build` itself.

---

## § Crate writer

> Body lifted from the retired `omnia-crate-writer` skill. Reads `spec.md` + `design.md` and writes `$CRATE_PATH`.

### Hard rules and authority hierarchy

1. **Specify artifacts are ground truth.** `spec.md` and `design.md` outrank inferred behaviour. If artifacts conflict with source, trust the artifacts.
2. **Apply update categories in fixed order**: structural → subtractive → modifying → additive. Type renames propagate first, dead code is removed before any new code is added, additive code depends on the already-updated type system.
3. **Idempotency is non-negotiable.** If a section of an existing crate already matches the artifacts, do nothing.
4. **No `unwrap()` / `expect()` in production code.** Tests may unwrap.
5. **Provider trait selection follows [`shape.md`](shape.md) and [`plugins/omnia/references/capabilities.md`](../../../plugins/omnia/references/capabilities.md).** Every external I/O point in `design.md` resolves to a provider trait.
6. **WASM guardrails are absolute** — see [`plugins/omnia/references/guardrails.md`](../../../plugins/omnia/references/guardrails.md). Forbidden crates and forbidden std APIs never appear in generated code.
7. **Never write tests in this step.** Tests belong to the test-writer pass below.
8. **Re-scan inventory after every structural change** before proceeding to subtractive / modifying / additive categories.

### Create mode

1. Author the workspace `Cargo.toml` and the crate `Cargo.toml`. Workspace dependencies pin `omnia-sdk` plus the `omnia-wasi-*` adapters for the provider traits the design declares. No private registries — every `omnia-*` crate lives on crates.io.
2. Generate `src/lib.rs` (or `src/main.rs` for non-library crates) with one module per handler. Module layout follows the convention: `handlers/<surface>.rs`, `types.rs`, `error.rs`, `provider.rs`.
3. For each handler, emit:
   - A request struct with the `Handler<P>` impl per [`shape.md`](shape.md) §Idiom: provider-based DI. `type Input` is one of `Vec<u8>` (POST / message body), `String` (single path param), `(String, String)` (tuple path params), `Option<String>` (query string), or `()` (scheduled / cron). Never `type Input = MyRequest`.
   - A standalone `async fn handle(owner: &str, request: …, provider: &P) -> Result<Reply<…>>` that the `Handler::handle` impl delegates to.
   - Response types implementing `IntoBody` for HTTP handlers (`fn into_body(self) -> anyhow::Result<Vec<u8>>`). Messaging handlers use `type Output = ()` and do not need `IntoBody`.
4. Emit a domain error enum via `thiserror`, plus `impl From<DomainError> for omnia_sdk::Error` mapping each variant to the right `BadRequest` / `NotFound` / `ServerError` / `BadGateway` constructor with stable `code` strings.
5. Author the provider trait bundle: an `AppProvider` trait that composes the per-handler trait bounds, plus a `Provider` struct in the guest wrapper that implements it via the `WasiConfig` / `WasiHttp` / … defaults.
6. Apply [`plugins/omnia/references/guardrails.md`](../../../plugins/omnia/references/guardrails.md) serde, timestamp, and DST rules verbatim: `#[serde(rename_all = "camelCase")]` on output types, `#[serde(skip_serializing_if = "Option::is_none")]` on optional fields, `#[serde(default)]` + `#[serde(rename(deserialize = …))]` on input-only types, `.earliest()` (not `.single()`) for DST-safe local-time conversion, `received_at = Utc::now()` semantics.

### Update mode

Walk the four categories in fixed order; re-scan after structural before proceeding.

1. **Structural** — type renames, file moves, module reshuffles. Apply via small, semantics-preserving rewrites. Re-run `cargo check` before moving on.
2. **Subtractive** — delete handlers / fields / types the new artifacts no longer name. Removing a topic subscription deletes the matching arm in the guest's messaging dispatcher (see §Guest writer for the four-category guest cadence).
3. **Modifying** — change a handler's behaviour, response shape, validation rules, or provider dependencies in place. Update the matching `Cargo.toml` adapter dependency if a new provider trait is consumed.
4. **Additive** — add new handlers, new types, new variants. Additive code MUST compile against the already-updated structural layer.

### Outputs and quality checklist

- Every handler in `design.md` has a matching module / function in `$CRATE_PATH`.
- Every external surface (HTTP route, topic publish/subscribe, WebSocket export, scheduled job) is wired in `src/lib.rs` if the crate exports guest types.
- Every provider trait bound on a handler appears in the `AppProvider` composition.
- Every `Config::get` key in `design.md` has a matching read in the handler (or in `Provider::new`).
- Every `omnia_sdk::Error` mapping in `design.md` has a matching arm in `impl From<DomainError>`.
- No forbidden crate or forbidden std API per [`plugins/omnia/references/guardrails.md`](../../../plugins/omnia/references/guardrails.md).
- `cargo fmt`, `cargo check`, `cargo clippy -- -D warnings` all pass before this section reports complete.

---

## § Test writer

> Body lifted from the retired `omnia-test-writer` skill. Reads `spec.md` + `design.md` + the existing crate inventory and writes `$CRATE_PATH/tests/`. Tests are spec-driven, not code-driven — generate the side-effect assertion implied by `design.md` even when the current handler does not yet satisfy it; a failing test is the right signal back to the crate-writer step.

### Authority hierarchy

`spec.md` scenarios drive happy-path, error-path, and validation coverage. `design.md` "Business logic" and "Provider trait dependencies" drive side-effect assertions (publishes, state writes, cache updates, transactions, rollback).

Manual tests in the existing suite are **flagged as drift, never silently deleted**. The drift report lists missing tests (in spec, not in suite), extra tests (in suite, not in spec), and assertion-drift cases (test present but assertions stale).

### Test generation process

1. **Load artifacts and references** — `spec.md`, `design.md`, [`plugins/omnia/references/providers/`](../../../plugins/omnia/references/providers/) for the trait-specific MockProvider patterns, and the slice's `tests/data/` fixtures if any.
2. **Inventory crate and tests** — enumerate handlers, provider trait bounds, request / response types, existing `tests/*.rs`, existing fixtures.
3. **Map specs to tests** — one deterministic test function per scenario, named `test_<crate>_<scenario_snake_case>`. Trace each test to the stable `REQ-XXX` ID in `spec.md` via a doc comment.
4. **Assert side effects** — enumerate every provider interaction in `design.md` and emit assertions: `assert_eq!(provider.publish_calls(), &[…])`, `assert_eq!(provider.state_writes(…), …)`, cache-aside hit/miss order, transaction commit vs rollback.
5. **Generate `MockProvider`** — implement only the provider traits the handler under test consumes. Static / replay variants per [`plugins/omnia/references/providers/`](../../../plugins/omnia/references/providers/).
6. **Load JSON fixtures** — `include_str!("data/<fixture>.json")` from `tests/data/`. Preserve any existing fixture style.
7. **Report drift** — emit drift notes inline (a leading `// DRIFT: ...` comment on tests that needed regeneration) but never delete operator-authored tests.

### Output and quality checklist

- Every requirement block in `spec.md` has at least one matching test function.
- Every provider interaction in `design.md` has at least one assertion.
- The MockProvider implements exactly the trait set the handlers consume (no extras).
- Fixture JSON files referenced by `include_str!` exist under `tests/data/`.
- No `cargo test` invocation here — execution belongs to the verify-repair loop below.

---

## § Guest writer

> Body lifted from the retired `omnia-guest-writer` skill. Runs on **first build only** (when no `src/lib.rs` exists at the workspace root). Subsequent builds skip this step; route / topic / WebSocket wiring updates are folded into the crate-writer's four-category cadence.

### Hard rules

- **Never put business logic in the guest.** All domain logic lives in the project's domain crates; the guest is wiring only.
- **Gate the guest with `#![cfg(target_arch = "wasm32")]`** — wasm32 is the only supported target.
- **Forbid `std::env`, `std::fs`, `std::net`, `std::thread::spawn`** in guest code. All I/O routes through provider traits; configuration via `omnia_sdk::Config`. Async only — no blocking operations.
- **Dispatch messaging handlers explicitly.** Match topics directly and return `Err` for any unhandled topic.
- **Export WebSocket handlers via `omnia_wasi_websocket::export!`** and implement `omnia_wasi_websocket::incoming_handler::Guest`.
- **Always pass an owner.** Every handler invocation must include `.owner("...")` in the builder chain.
- **Use the builder API** — `.provider(&p).owner("o").await`, never the legacy `.process(&p)` form.
- **Axum 0.8 route params use `{param}` brace syntax**, never `:param`.

### Process

1. Lay down the root workspace: `Cargo.toml` (workspace), `.cargo/config.toml`, `rust-toolchain.toml`, `rustfmt.toml`, `clippy.toml`, `Makefile.toml`, `.vscode/settings.json`.
2. Generate `src/lib.rs` with Axum HTTP routing, message-topic dispatcher (`match topic { … }`), and WebSocket export hooks. See [`plugins/omnia/references/guest-patterns.md`](../../../plugins/omnia/references/guest-patterns.md) for the canonical export patterns.
3. Implement the `Provider` struct that satisfies the consumed `omnia-wasi-*` adapter traits. Validate every required `Config::get` key in `Provider::new()` and document each in `examples/.env.example`.
4. Author `examples/<guest-name>.rs` with the `omnia::runtime!({ main: true, hosts: { … } });` block enumerating every WASI host the guest consumes. See [`plugins/omnia/references/runtime.md`](../../../plugins/omnia/references/runtime.md).
5. Author the supply-chain files: `deny.toml`, `cargo-vet` config (`exemptions.lock`, `imports.lock`, `audits.toml`). After the workspace builds for the first time and produces `Cargo.lock`, run `cargo vet regenerate {imports,exemptions,unpublished}`.
6. Author the five GitHub workflows: `audit`, `ci`, `patch`, `publish`, `release`.
7. Verify with `cargo check` — fix any missing route / provider impl / wasm32-incompatible usage and re-check until clean.

### When `WasiIdentity` is consumed

Identity needs OAuth2 credentials wired through `Config`. Add `IDENTITY_CLIENT_ID`, `IDENTITY_CLIENT_SECRET`, `IDENTITY_TOKEN_URL` to `.env.example` and assert their presence in `Provider::new()`.

---

## § Verify-repair loop (max 3 iterations)

Run after both crate-writer and test-writer have completed. Each iteration runs the three checks below; if any fail, classify the failure, apply the targeted fix, and start a new iteration.

### Iteration steps

```bash
cd $CRATE_PATH && cargo fmt --check
cd $CRATE_PATH && cargo check
cd $CRATE_PATH && cargo clippy -- -D warnings
cd $CRATE_PATH && cargo test
```

If `cargo fmt --check` fails, run `cargo fmt` once. Formatting is mechanical; one pass suffices.

If `cargo check` or `cargo clippy` fails, re-enter the crate-writer step with the error output as context. Apply minimum-change repair discipline (see below).

If `cargo test` fails, classify each failure:

| Failure signal | Classification | Fix action |
|---|---|---|
| Error in `tests/` paths, `MockProvider`, or `provider.rs` | Test issue | Re-enter test-writer with the error output |
| Error in `src/` paths, missing trait impls in production | Code issue | Re-enter crate-writer with the error output |
| Assertion mismatch where *actual* matches spec | Test issue | Test expectation is stale |
| Assertion mismatch where *expected* matches spec | Code issue | Handler returns the wrong result |
| MockProvider missing a trait impl the handler now requires | Test issue | Update MockProvider |
| Unresolved import or missing crate in `Cargo.toml` | Workspace issue | Fix `Cargo.toml` paths or workspace member list directly |

### Repair discipline

- **Minimum change only** — fix the reported error and nothing else. Do not refactor adjacent code. A repair that touches more than the failing path is likely to introduce new failures elsewhere, causing the loop to oscillate.
- **Scope the diff** — before committing a repair, verify the change is limited to files and functions named in the error output.
- **One failure class per re-entry** — group failures by classification and re-enter each step once with all same-class errors. Do not interleave code and test fixes.

### Update-mode regression check

When in update mode, before Iteration 1 record the baseline `cargo test` output:

```bash
cd $CRATE_PATH && cargo test 2>&1 | tee /tmp/${SLICE_NAME}-${CRATE_NAME}-baseline.txt
```

After each iteration, for each test that passed before and now fails:

- If `spec.md` explicitly changes the asserted behaviour → expected behavioural change, not a regression. Re-enter test-writer to align expectations.
- If `spec.md` does not change the asserted behaviour → true regression. Route the fix through the classification table.

### Loop control

Repeat until all four checks pass or 3 iterations exhausted. If still failing after 3 iterations: **STOP**. Do not mark the slice complete. Report the remaining failures with full error output to the operator and signal the build phase outcome accordingly.

---

## § Code reviewer

> Body lifted from the retired `omnia-code-reviewer` skill. Runs after the verify-repair loop succeeds. Drives an agent team of three specialists (Security, Correctness, Quality) plus an antagonist; the lead synthesises the final review into `$REVIEW_OUTPUT = $CRATE_PATH/REVIEW.md`.

### Review pipeline

1. **Verify prerequisites** — `cargo check` passes (the verify-repair loop already guarantees this) and `$CRATE_PATH` exists. Resolve the optional `fix` flag.
2. **Spawn specialists concurrently**:
   - Security Reviewer — SEC-prefixed findings: SQL injection / XSS / secret leakage in fixtures, WASM constraint violations.
   - Correctness Reviewer — COR-prefixed findings: `unwrap` / `expect` in production paths, validation placement (parse-time vs handle-time), provider trait misuse, missing error mapping.
   - Quality Reviewer — QUA-prefixed findings: N+1 provider calls, naming, function length (>50 lines), dead code, missing doc comments on public items.
3. **Universal checks (lead)** — apply UNI-001 … UNI-021 from the default codex with Omnia / WASM heuristics; prefix `UNI-`. Skip universal checks already covered by SEC / COR / QUA.
4. **Adversarial challenge** — forward all findings to the antagonist. The antagonist confirms, upgrades, downgrades, disputes, and may add `NEW-` findings.
5. **Synthesis** — author `REVIEW.md` with sections: Summary, Findings (grouped by severity), Adversarial Review (confirmed / downgraded / upgraded / disputed / new tallies), Auto-Fix Summary (when `fix` is set), Quality Metrics.
6. **Auto-fix (only when `fix`)** — apply safe fixes for confirmed / upgraded auto-fixable findings only. Re-run `cargo check`; revert on failure. Respect antagonist regression flags.

### Finding-ID conventions

- Report-local occurrence IDs: `SEC-1`, `COR-1`, `QUA-1`, `UNI-1`, `NEW-1`.
- Stable codex citations: `rule_id: OMNIA-002` (for example) appears alongside each mapped finding. The codex rules live under [`plugins/omnia/references/codex/`](../../../plugins/omnia/references/codex/): `OMNIA-001` Provider-Only Host Access, `OMNIA-002` WASM Guest Runtime Constraints, `RUST-001` Classified SDK Errors / No Panic Paths, `SEC-001` Host-Managed Secrets and Identity.
- Severity reflects antagonist adjustments — upgrades and downgrades rewrite the displayed severity but preserve the original prefix and occurrence ID.
- Every finding carries a `file:line` reference and a verbatim code snippet.

### Auto-fix scope

Auto-fix applies only to findings the antagonist confirmed or upgraded, and only to the auto-fixable categories listed in the per-category success-rate table that lives in the codex notes. Auto-fix runs after synthesis, before the report is finalised. If `cargo check` fails after a fix is applied, the fix reverts and the finding is left for manual handling. Critical / high findings not auto-fixed are left for the operator; medium findings without an auto-fix are documented as accepted technical debt with rationale; low findings are reported and require no action.

### Remediation cycle

After auto-fix completes:

1. Parse `$REVIEW_OUTPUT`. Process by severity.
2. **CRITICAL / HIGH** — auto-fixable + not disputed: apply the fix directly. Non-auto-fixable: classify as test issue vs code issue and re-enter the matching § (crate writer or test writer). After all critical / high fixes, run the verify-repair loop with max 2 iterations (tighter than the standard 3, since these are targeted repairs).
3. **MEDIUM** — auto-fix when available; otherwise document as accepted technical debt in `REVIEW.md`.
4. **LOW** — document only.
5. Re-run the code reviewer (without `fix`) to verify fix quality. If new CRITICAL / HIGH findings appear, repeat the remediation cycle once.

---

## References

- [`shape.md`](shape.md) — Idiom guidance core synthesis already folded into the slice's `spec.md` + `design.md`.
- [`plugins/omnia/references/guardrails.md`](../../../plugins/omnia/references/guardrails.md) — Forbidden crates / APIs, statelessness rules, serde / timestamp / DST idioms.
- [`plugins/omnia/references/capabilities.md`](../../../plugins/omnia/references/capabilities.md) — All nine provider traits with signatures and adapter triggers.
- [`plugins/omnia/references/guest-patterns.md`](../../../plugins/omnia/references/guest-patterns.md) — HTTP / Messaging / WebSocket guest export patterns.
- [`plugins/omnia/references/guest-wiring.md`](../../../plugins/omnia/references/guest-wiring.md) — Crate → guest injection contract.
- [`plugins/omnia/references/runtime.md`](../../../plugins/omnia/references/runtime.md) — `omnia::runtime!` macro, WASI host options, `.env.example` shape.
- [`plugins/omnia/references/providers/`](../../../plugins/omnia/references/providers/) — Per-provider deep dives.
- [`plugins/omnia/references/agent-teams.md`](../../../plugins/omnia/references/agent-teams.md) — Multi-agent review pattern (specialists + antagonist + lead synthesis).
- [`plugins/omnia/references/codex/`](../../../plugins/omnia/references/codex/) — Stable codex rules cited by the code reviewer.
