---
name: omnia-crate-writer
description: "Write Rust WASM crates from Specify artifacts -- greenfield creation or incremental updates -- following Omnia SDK patterns with provider-based dependency injection. Use when an Omnia slice has pending crate-implementation tasks, or when an existing crate must be regenerated after artifact updates; not for guest wiring (`guest-writer`) or test scaffolding (`test-writer`)."
argument-hint: "[crate-name]"
---

# Crate Writer

> **Write Rust WASM crates from Specify artifacts (specs + design.md), following Omnia SDK patterns for stateless, provider-based WASM components.** Handles both greenfield creation and incremental updates; tests come from `test-writer` in a later step.

## Arguments

```text
$CRATE_NAME    = $ARGUMENTS[0]
$SLICE_DIR     = .specify/slices/$CRATE_NAME
$SPECS_DIR     = $SLICE_DIR/specs
$DESIGN_PATH   = $SLICE_DIR/design.md
$CRATE_PATH    = crates/$CRATE_NAME
```

The orchestrator passes `$CRATE_NAME` explicitly. The runbook covers what to do when invoked without one.

## Critical Path

1. **Detect mode**: `$CRATE_PATH/Cargo.toml` exists -> update; missing -> create.
2. **Read** [rules.md](./rules.md) — the Hard Rules and Authority Hierarchy bind every step below.
3. **Read artifacts** (`spec.md`, `design.md`) and required references; pick the matching example under [`examples/`](./examples/).
4. **Derive Omnia capabilities** from design.md (Source Capabilities Summary, External Services, `[runtime]` constraints) via [capability-mapping.md](references/capability-mapping.md) and [wasm-constraints.md](references/wasm-constraints.md); apply artifact corrections (Hard Rule 9) before writing code.
5. **Build the three matrices** (Side-Effect, Outbound Message, Transaction Boundary) for every changed handler; every cell must land in code.
6. **Generate / update code** following the per-mode process below; in update mode apply categories in fixed order: structural → subtractive → modifying → additive.
7. **Smoke check** with `cargo check`, run traceability verification, then inject or update guest wiring (when `src/lib.rs` exists). Tests come from test-writer in a later step.

## Orientation

The skill accepts Specify artifacts from any producer: code-analysis artifacts (from `/spec:extract`) and feature specs (from change artifacts). Mode dispatch is mechanical — the presence of `$CRATE_PATH/Cargo.toml` selects create vs update; arguments and path derivation (`$CRATE_NAME`, `$SLICE_DIR`, `$SPECS_DIR`, `$DESIGN_PATH`, `$CRATE_PATH`) are listed in the runbook.

Every handler follows the delegation pattern: a request struct implementing `Handler<P>` delegates to a standalone `async fn handle()`. Domain errors use `thiserror` and convert to `omnia_sdk::Error` via `From<DomainError>`. Never use `type Input = MyRequest` (bypasses deserialization) and never call `Utc::now()` in `from_input()` (`shift_time` cannot fix parse-time validation).

Update mode walks four change categories — **structural → subtractive → modifying → additive** — in fixed order so that type renames propagate first, dead code is removed before any new code, and additive code depends on the already-updated type system. After structural changes, re-scan the inventory and re-run `cargo check` before proceeding ([rules.md](./rules.md) Hard Rule 16). Idempotency is non-negotiable: if a section already matches the artifacts, do nothing.

Guest wiring fires only when `src/lib.rs` exists. In create mode it is append-only (routes, topic arms, WebSocket delegations, instrumented handler functions, provider impls, crate dependency); in update mode it follows the same four-category split as the crate edits.

See [`references/runbook.md`](references/runbook.md) for the operational detail (mode dispatch, full reference list, examples, artifact mapping, crate structure, error handling, guest-wiring rules, the per-mode generation process, and the outputs & quality table).

## Reference Documentation

| Reference | Purpose |
|---|---|
| [`references/runbook.md`](references/runbook.md) | Mode dispatch, arguments, required references list, examples, artifact mapping, crate structure, handler / error / guest-wiring detail, full Create / Update process, outputs & quality table |
| [`rules.md`](./rules.md) | Hard Rules and Authority Hierarchy that bind every generation pass |
| [`references/sdk-api.md`](references/sdk-api.md) | `Handler<P>`, `Context`, `Reply`, `IntoBody`, `Client`, `Error`; Input Type Decision Tree; Response Types |
| [`references/capabilities.md`](references/capabilities.md) | All 9 provider traits with exact signatures and artifact triggers |
| [`references/capability-mapping.md`](references/capability-mapping.md) | Mapping from Specify artifact capabilities to Omnia provider traits |
| [`references/wasm-constraints.md`](references/wasm-constraints.md) | Translating `[runtime]` constraints to Omnia/WASM patterns |
| [`references/providers.md`](references/providers.md) | Provider struct setup, trait composition rules, MockProvider patterns |
| [`references/error-handling.md`](references/error-handling.md) | Error macros, domain error enums, context patterns, troubleshooting |
| [`references/guardrails.md`](references/guardrails.md) | WASM constraints and forbidden patterns |
| [`references/cargo-toml.md`](references/cargo-toml.md) | `Cargo.toml` template and dependency rules |
| [`references/guest-wiring.md`](references/guest-wiring.md) | How crates wire into the WASM guest |
| [`references/checklists.md`](references/checklists.md) | Pre-generation and verification checklists |
| [`references/todo-markers.md`](references/todo-markers.md) | TODO marker rules, capability overrides, cache-aside patterns |
| [`references/output-documents.md`](references/output-documents.md) | `Migration.md`, `Architecture.md`, `CHANGELOG.md`, `.env.example` shapes |
| [`references/cross-cutting-matrices.md`](references/cross-cutting-matrices.md) | Side-Effect / Outbound-Message / Transaction-Boundary matrices and traceability rules |
| [`references/update-patterns.md`](references/update-patterns.md) | Update strategy patterns by category |
| [`references/change-classification.md`](references/change-classification.md) | How to classify artifact-vs-code differences |
| [`references/mock-provider.md`](references/mock-provider.md) | MockProvider patterns referenced by guest/handler examples |
| [`references/repair-patterns.md`](references/repair-patterns.md) | Common verify-loop repair patterns |
| [`references/guest-patterns.md`](references/guest-patterns.md) | Guest wiring patterns by capability |
| [`references/providers/`](references/providers/) | Per-provider deep-dive notes |

## Guardrails

- **NEVER write tests in this skill.** Tests are `test-writer`'s responsibility; a unified verify-repair loop runs after both writers complete.
- **NEVER reorder update categories.** Apply them in fixed order — structural → subtractive → modifying → additive — and re-run the inventory after structural changes (Hard Rule 16 in [rules.md](./rules.md)).
- **NEVER touch sections of an existing crate that already match the artifacts.** Idempotency: if the artifact section matches the code, do nothing.
- **ALWAYS delegate authority through [rules.md](./rules.md).** Hard Rules and the Authority Hierarchy bind every decision in this skill; conflicts are resolved by that document, not local prose.
