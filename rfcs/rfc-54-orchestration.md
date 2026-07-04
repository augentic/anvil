# RFC-54: Vertical Adapter Operation Proof

> Status: Draft · Order 5 of 10 · Stage S3 · Depends: [RFC-51](rfc-51-adapter-wit.md), [RFC-53](rfc-53-wasi-model.md), [RFC-59](rfc-59-model-tool-loop.md) · Enables: [RFC-56](rfc-56-runtime-move.md), [RFC-57](rfc-57-specify-guests.md) · Owns: the first end-to-end adapter operation on typed effects

## Abstract

This RFC proves the architecture with one real adapter operation before the whole workflow moves. The proof has two legs: one deterministic `tool` operation invoked through generated bindings, and one judgment operation invoked through `wasi-model.eval` with lazy reference loading. It validates the contract, runtime core, model boundary, and adapter reference shelf against real Specify behavior while the blast radius is still small.

## The proof

- **Deterministic leg.** A selected `execution: tool` operation is reached through its world export using [RFC-51](rfc-51-adapter-wit.md) bindings. This retires `wasi:cli/run` for that operation: typed request in, typed `result<_, error>` out, no argv packing or stdout-JSON parsing.
- **Judgment leg.** A selected judgment operation calls `wasi-model.eval` ([RFC-53](rfc-53-wasi-model.md)). The model tool loop ([RFC-59](rfc-59-model-tool-loop.md)) resolves the adapter's `references` shelf, reads only through bounded tools, and returns a validated typed answer.
- **Replay leg.** The judgment operation records and replays through the [RFC-53](rfc-53-wasi-model.md) replay boundary, so CI can exercise the operation without a live model.

The chosen operation may be an Omnia target operation if that is the best proving workload, but the Omnia target adapter is not a prerequisite to the generic runtime core.

## Brief-typing

The operation signature is named at the call site, so a brief is a prompt body, not a contract-bearing artifact. The authoring-time checks survive as `specify lint framework` rules: every agent operation has exactly one binding brief; a brief's placeholders reference real request fields; embedded examples validate against the WIT-derived report schema; the reference-discovery graph resolves without loading the referenced prose.

## Scope

- Route one deterministic `tool` adapter operation through generated bindings.
- Route one judgment adapter operation through `wasi-model.eval`.
- Exercise the adapter `references` shelf through lazy resolution.
- Record and replay the judgment operation.
- Keep the proof adapter-local; do not move the whole workflow yet.

## Out of scope

- Broad workflow sequencing; see [RFC-57](rfc-57-specify-guests.md).
- The complete model backend catalogue; see [RFC-58](rfc-58-model-backends.md).
- Broad target `build` / `merge` migration before verify profiles are locked down; see [RFC-60](rfc-60-verify-profiles.md).

## Acceptance criteria

1. One deterministic operation invokes a WASM component export through generated bindings.
2. One judgment operation runs through `eval` against an adapter that exports its `references` shelf.
3. The judgment operation replays deterministically through the [RFC-53](rfc-53-wasi-model.md) replay boundary.
4. Lazy reference loading holds: the call carries a brief id plus handles, not an inlined corpus.
5. `make lint` and `cargo make ci` stay green at each increment.

## Risks and invariants

- **Prove, then widen.** This RFC validates the execution model with one adapter operation; it is not the workflow migration.
- **Prose holism.** The orchestrator hands the model whole briefs. It sequences and types; it does not fragment the prompt.
- **Adapter-local.** Adapter logic lives in exports and the shelf. Workflow-level fan-out belongs in [RFC-57](rfc-57-specify-guests.md).
- **Law 2 preserved.** The model id stays in the `wasi-model` backend, never in the contract or runtime core.
