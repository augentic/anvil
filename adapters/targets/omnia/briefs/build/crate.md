# Omnia build — crate writer

Loaded by [../build.md](../build.md) phase 2. Reads `specs/<domain>/spec.md` + `design.md`, writes `$CRATE_PATH`. Sequenced after [shape.md](../shape.md) (idiom guidance already folded into spec + design by core synthesis).

## Authority hierarchy

The full Hard Rules + Authority Hierarchy live in [`../../references/hard-rules.md`](../../references/hard-rules.md). The summary below is a load-bearing extract; the full reference governs ties.

1. **Specify artifacts are ground truth.** `specs/<domain>/spec.md` and `design.md` outrank inferred behaviour. If artifacts conflict with source, trust the artifacts.
2. **Apply update categories in fixed order**: structural → subtractive → modifying → additive. Type renames propagate first, dead code is removed before any new code is added, additive code depends on the already-updated type system.
3. **Idempotency is non-negotiable.** If a section of an existing crate already matches the artifacts, do nothing.
4. **No `unwrap()` / `expect()` in production code.** Tests may unwrap.
5. **Provider trait selection follows [`shape.md`](../shape.md) and [`capabilities.md`](../../references/capabilities.md).** Every external I/O point in `design.md` resolves to a provider trait; see [`capability-mapping.md`](../../references/capability-mapping.md) for the artifact-to-trait mapping rules.
6. **WASM guardrails are absolute** — see [`guardrails.md`](../../references/guardrails.md) and [`wasm-constraints.md`](../../references/wasm-constraints.md). Forbidden crates and forbidden std APIs never appear in generated code.
7. **Never write tests in this step.** Tests belong to the [test writer](test.md) pass.
8. **Re-scan inventory after every structural change** before proceeding to subtractive / modifying / additive categories. See [`change-classification.md`](../../references/change-classification.md) for how to classify artifact-vs-code differences.

## Critical path

1. Read [shape.md](../shape.md) refresher and the slice's `specs/<domain>/spec.md` + `design.md` + `tasks.md`.
2. **Build the three cross-cutting matrices** per [`cross-cutting-matrices.md`](../../references/cross-cutting-matrices.md): Side-Effect, Outbound-Message, Transaction-Boundary. Every cell must land in code.
3. **Mode dispatch.** Inherited from the parent brief: create mode (no `Cargo.toml`) vs update mode.
4. Apply the per-mode process below; in update mode walk the four categories in fixed order.
5. Run the inventory re-scan after every structural change.
6. Return control to the parent brief, which runs the verify-repair loop.

## Create mode

1. Author the workspace `Cargo.toml` and the crate `Cargo.toml` per [`cargo-toml.md`](../../references/cargo-toml.md). Workspace dependencies pin `omnia-sdk` plus the `omnia-wasi-*` adapters for the provider traits the design declares. No private registries — every `omnia-*` crate lives on crates.io.
2. Generate `src/lib.rs` (or `src/main.rs` for non-library crates) with one module per handler. Module layout follows the convention: `handlers/<surface>.rs`, `types.rs`, `error.rs`, `provider.rs`.
3. For each handler, emit:
   - A request struct with the `Handler<P>` impl per [`shape.md`](../shape.md) §Idiom: provider-based DI and [`sdk-api.md`](../../references/sdk-api.md) (`Handler<P>`, `Context`, `Reply`, `IntoBody`, `Client`, `Error`; Input Type Decision Tree; Response Types). `type Input` is one of `Vec<u8>` (POST / message body), `String` (single path param), `(String, String)` (tuple path params), `Option<String>` (query string), or `()` (scheduled / cron). Never `type Input = MyRequest`.
   - A standalone `async fn handle(owner: &str, request: …, provider: &P) -> Result<Reply<…>>` that the `Handler::handle` impl delegates to.
   - Response types implementing `IntoBody` for HTTP handlers (`fn into_body(self) -> anyhow::Result<Vec<u8>>`). Messaging handlers use `type Output = ()` and do not need `IntoBody`.
4. Emit a domain error enum via `thiserror`, plus `impl From<DomainError> for omnia_sdk::Error` mapping each variant to the right `BadRequest` / `NotFound` / `ServerError` / `BadGateway` constructor with stable `code` strings. See [`error-handling.md`](../../references/error-handling.md) for the macros, domain enums, context patterns, and troubleshooting.
5. Author the provider trait bundle: an `AppProvider` trait that composes the per-handler trait bounds, plus a `Provider` struct in the guest wrapper that implements it via the `WasiConfig` / `WasiHttp` / … defaults.
6. Apply [`guardrails.md`](../../references/guardrails.md) serde, timestamp, and DST rules verbatim: `#[serde(rename_all = "camelCase")]` on output types, `#[serde(skip_serializing_if = "Option::is_none")]` on optional fields, `#[serde(default)]` + `#[serde(rename(deserialize = …))]` on input-only types, `.earliest()` (not `.single()`) for DST-safe local-time conversion, `received_at = Utc::now()` semantics.
7. Honour TODO markers, adapter overrides, and cache-aside patterns per [`todo-markers.md`](../../references/todo-markers.md).
8. Emit accompanying output documents (`Migration.md`, `Architecture.md`, `CHANGELOG.md`, `.env.example`) per [`output-documents.md`](../../references/output-documents.md).

Worked examples: [`examples/crates/single-handler.md`](../../references/examples/crates/single-handler.md), [`examples/crates/multi-handler.md`](../../references/examples/crates/multi-handler.md), per-capability walkthroughs under [`examples/crates/capabilities/`](../../references/examples/crates/capabilities/), and [`examples/crates/anti-patterns.md`](../../references/examples/crates/anti-patterns.md) for shapes to avoid.

## Update mode

Walk the four categories in fixed order; re-scan after structural before proceeding. The strategy library per category lives at [`update-patterns.md`](../../references/update-patterns.md); diff classification rules live at [`change-classification.md`](../../references/change-classification.md).

1. **Structural** — type renames, file moves, module reshuffles. Apply via small, semantics-preserving rewrites. Re-run `cargo check` before moving on. Worked example: [`examples/crates/updates/structural.md`](../../references/examples/crates/updates/structural.md).
2. **Subtractive** — delete handlers / fields / types the new artifacts no longer name. Removing a topic subscription deletes the matching arm in the guest's messaging dispatcher (see [guest writer](guest.md) for the four-category guest cadence). Worked example: [`examples/crates/updates/subtractive.md`](../../references/examples/crates/updates/subtractive.md).
3. **Modifying** — change a handler's behaviour, response shape, validation rules, or provider dependencies in place. Update the matching `Cargo.toml` adapter dependency if a new provider trait is consumed. Worked example: [`examples/crates/updates/modifying.md`](../../references/examples/crates/updates/modifying.md).
4. **Additive** — add new handlers, new types, new variants. Additive code MUST compile against the already-updated structural layer. Worked example: [`examples/crates/updates/additive.md`](../../references/examples/crates/updates/additive.md).

When a `cargo` failure surfaces during this pass, apply minimum-change repair via [`repair-patterns.md`](../../references/repair-patterns.md) before re-entering the parent verify-repair loop.

## Outputs and quality checklist

The full checklist lives at [`checklists.md`](../../references/checklists.md). Highlights:

- Every handler in `design.md` has a matching module / function in `$CRATE_PATH`.
- Every external surface (HTTP route, topic publish/subscribe, WebSocket export, scheduled job) is wired in `src/lib.rs` if the crate exports guest types.
- Every provider trait bound on a handler appears in the `AppProvider` composition.
- Every `Config::get` key in `design.md` has a matching read in the handler (or in `Provider::new`).
- Every `omnia_sdk::Error` mapping in `design.md` has a matching arm in `impl From<DomainError>`.
- No forbidden crate or forbidden std API per [`guardrails.md`](../../references/guardrails.md).
- `cargo fmt`, `cargo check`, `cargo clippy -- -D warnings` all pass before returning control to the parent brief's verify-repair loop.
