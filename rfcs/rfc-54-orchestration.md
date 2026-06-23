# RFC-54: Orchestration — deterministic dispatch and judgment via `eval`

> Status: Draft · Order 4 of 8 · Stage S3 · Depends: [RFC-52](rfc-52-effect.md), [RFC-53](rfc-53-wasi-model.md) · Enables: [RFC-56](rfc-56-runtime-move.md), [RFC-57](rfc-57-specify-guests.md) · Owns: how an adapter operation is sequenced

## Abstract

An adapter operation splits along its grain. The **deterministic** part — the `tool` operations (e.g. the `contract` and `vectis` validators) — is a typed guest export, called through the [RFC-51](rfc-51-adapter-wit.md) bindings. The **judgment** part — `build` / `extract` / `merge` synthesis — runs through `wasi-model.eval` ([RFC-53](rfc-53-wasi-model.md)): the orchestrator hands the model the operation's brief, and the backend's tool loop resolves the adapter's `references` shelf, reads and writes the working tree, and verifies. The N typed steps, lazy reference loading, conditional sub-flows, and verify-repair are properties of that loop.

## The model

- **Typed tool dispatch.** Deterministic `tool` operations are reached through their world export (`instance.call_build(&mut store, &req)`), retiring `wasi:cli/run` on that path — a typed `result<_, error>` in place of exit codes and stdout JSON.
- **Judgment through `eval`.** A judgment operation calls `eval` with the brief `path`; the model pulls the brief's references through `resolve`, scans code through `read` / `list`, mutates through `write`, and checks through `verify`. What stays in the guest is the deterministic exports and the `references` shelf; judgment runs in the model backend.
- **Reentrancy.** Every adapter export the orchestrator (or the `eval` loop) invokes lands in a fresh instance on a new store — component instances are not reentrant, so a guest export invoked mid-operation never shares a store with an open session.

## Brief-typing (authoring-time lint)

The operation signature is named at the call site, so a brief is a prompt body, not a contract-bearing artifact. The authoring-time checks survive as `specify lint framework` rules: every agent operation has exactly one binding brief; a brief's placeholders reference real request fields; embedded examples validate against the WIT-derived report schema; the reference-discovery graph resolves without loading it.

## Scope

- Route `execution: tool` adapters through the RFC-51 generated bindings; retire `wasi:cli/run` on that path.
- Express judgment operations as `eval` calls over the adapter's brief and `references` shelf.
- The authoring-time brief lint rules above.

## Acceptance criteria

1. The deterministic `tool` adapters are invoked through the generated bindings — no argv packing or stdout-JSON parsing; `wasi:cli/run` is retired for `execution: tool`.
2. At least one judgment operation runs through `eval` against an adapter that exports its `references` shelf.
3. The operation replays deterministically via the replay model backend ([RFC-53](rfc-53-wasi-model.md)).
4. Lazy reference loading holds — the call carries a brief `path` + handles; no corpus crosses the boundary.
5. `make lint` and `cargo make ci` stay green at each increment.

## Risks and invariants

- **Prose holism.** The orchestrator hands the model whole briefs; it sequences and types, it does not fragment the prompt.
- **Adapter-local.** This RFC is one adapter operation; the workflow layer is [RFC-57](rfc-57-specify-guests.md).
- **Law 2 preserved.** Adapter logic lives in exports and the shelf; the model id stays in the `wasi-model` backend.
