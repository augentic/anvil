# Expected shape-injection evidence

Synthesis (W3.1, `/spec:refine`) MUST fold the Omnia `shape` brief's idiom guidance into the slice's `spec.md` and `design.md` regardless of source. A pure-intent fixture and a documentation-sourced fixture for the same slice MUST both surface every checkbox below; this is the acceptance contract for scenario #5h.

## `spec.md` checklist

- [x] Per-requirement provenance block (`ID:`, `Sources:`, `Status:`) per RFC-25 §Requirement block contract.
- [x] One requirement block per handler-observable behaviour (HTTP / message / WebSocket / scheduled).
- [x] Acceptance scenarios that name **inputs**, **provider state preconditions**, and **observable outcomes**.
- [x] An "Error conditions" table mapping triggers to `omnia_sdk::Error` variants (one of `BadRequest`, `NotFound`, `ServerError`, `BadGateway`) with stable `code` strings.

## `design.md` checklist (sections must appear in this order)

- [x] **Domain model** — newtypes for IDs; no raw primitives for domain concepts; types listed before behaviour.
- [x] **Provider trait dependencies** — table listing each handler's consumed traits drawn from the closed Omnia set (`Config`, `HttpRequest`, `Publish`, `StateStore`, `Identity`, `TableStore`, `Broadcast`, `Blobstore`, `DocumentStore`).
- [x] **Handler delegation** — request struct → `Handler<P>` impl → standalone `async fn handle()` per handler; `type Input` is one of `Vec<u8>` / `String` / `(String, String)` / `Option<String>` / `()`; never `type Input = MyRequest`; `Utc::now()` never called in `from_input`.
- [x] **External surfaces** — explicit HTTP / message / WebSocket / scheduled list with Axum 0.8 `{param}` brace syntax for routes.
- [x] **Configuration** — every `Config::get` key enumerated with default + missing-key behaviour.
- [x] **Error mapping** — `thiserror`-derived domain error enum with `From<DomainError> for omnia_sdk::Error` per variant.
- [x] **Validation placement** — table partitioning each check into `from_input()` (structural) or `handle()` (temporal / contextual) with explicit reasoning.
- [x] **Observability** — handler-level `tracing::info!(monotonic_counter.* / gauge.*)` metric names enumerated.

## Cross-fixture invariant

A pure-intent variant (`sources: [intent]`, candidate ≡ slice name) and a documentation-sourced variant of the same slice MUST produce `spec.md` / `design.md` where every checkbox above is satisfied. The `Sources:` line on each requirement block reflects the actual source(s); the structural shape does not vary.
