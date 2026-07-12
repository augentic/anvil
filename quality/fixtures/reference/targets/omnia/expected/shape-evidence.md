# Expected shape-injection evidence

Synthesis (`/spec:refine`) MUST fold the Omnia `shape` brief's idiom guidance into the slice's `spec.md` and `design.md` regardless of source. A pure-intent fixture and a documentation-sourced fixture for the same slice MUST both surface every checkbox below; this is the eval contract for scenario #5h.

## `spec.md` checklist

- [x] Per-requirement provenance block (`ID:`, `Sources:`, `Status:`) per workflow §Requirement block contract.
- [x] One requirement block per operation-observable behaviour (HTTP / message / WebSocket / scheduled).
- [x] Acceptance scenarios that name **inputs**, **provider state preconditions**, and **observable outcomes**.
- [x] An "Error conditions" table mapping triggers to `omnia_guest::Error` variants (one of `BadRequest`, `NotFound`, `ServerError`, `BadGateway`) with stable `code` strings.

## `design.md` checklist (sections must appear in this order)

- [x] **Domain model** — newtypes for IDs; no raw primitives for domain concepts; types listed before behaviour.
- [x] **Provider trait dependencies** — table listing each operation's consumed traits drawn from the closed Omnia set (`Config`, `HttpRequest`, `Publish`, `StateStore`, `Identity`, `TableStore`, `Broadcast`, `Blobstore`, `DocumentStore`).
- [x] **Operation delegation** — stateless operation type → `Operation<P>` impl with a typed request `Input` → standalone domain function; `Operation::call` receives `CallContext<'_, P>` and returns a typed serialisable output; runtime-only values such as `Utc::now()` are never computed during transport decoding.
- [x] **External surfaces** — explicit HTTP / message / WebSocket / scheduled list with Axum 0.8 `{param}` brace syntax for routes.
- [x] **Configuration** — every `Config::get` key enumerated with default + missing-key behaviour.
- [x] **Error mapping** — `thiserror`-derived domain error enum with `From<DomainError> for omnia_guest::Error` per variant.
- [x] **Validation placement** — table partitioning each check into transport decoding/conversion, `Operation::call` (typed structural validation), or delegated domain logic (temporal/contextual validation) with explicit reasoning.
- [x] **Observability** — operation-level `tracing::info!(monotonic_counter.* / gauge.*)` metric names enumerated.

## Cross-fixture invariant

A pure-intent variant (`sources: [intent]`, lead ≡ slice name) and a documentation-sourced variant of the same slice MUST produce `spec.md` / `design.md` where every checkbox above is satisfied. The `Sources:` line on each requirement block reflects the actual source(s); the structural shape does not vary.
