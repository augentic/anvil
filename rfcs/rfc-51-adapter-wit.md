# RFC-51: Adapter WIT — the typed contract package

> Status: Draft · Order 1 of 8 · Stage S1 · Enables: [RFC-52](rfc-52-effect.md), [RFC-54](rfc-54-orchestration.md), [RFC-56](rfc-56-runtime-move.md) · Owns: the `augentic:specify` package

## Abstract

One versioned WebAssembly Component Model package, `augentic:specify@<semver>`, is the single typed contract for both adapter axes. It defines the cross-cutting data types, the per-axis `source` / `target` operation signatures, the `references` shelf every adapter exports, and the worlds. Generated, type-checked records replace argv conventions and stdout-JSON envelopes.

## The package

Authored in [`../wit/specify.wit`](../wit/specify.wit):

- `interface types` — the cross-cutting records: `error`, `adapter-id`, `artifact`, `revision`, `edit`, `changeset`.
- `interface source` — `survey` / `extract`, each taking an `adapter-id`, with `lead`, `weight`, `backing`, `claim`, `evidence`.
- `interface target` — `guidance` / `build` / `merge`, each taking an `adapter-id`, with `input`, `working-tree`, `finding`, `severity`, `outcome`, `report`.
- `interface references` — the reference shelf: `resolve(adapter-id, reference) -> bytes`, a stateless adapter export the model backend ([RFC-53](rfc-53-wasi-model.md)) calls to follow a brief's internal references. Served from prose embedded in the module at build time, so `resolve` is an in-module lookup.
- worlds — `source-adapter` exports `source` + `references`; `target-adapter` exports `target` + `references`; `workflow` imports `source` + `target`. The per-axis interfaces double as the adapter's export and the workflow's host-satisfied import: naming a plan-bound `adapter-id` as each call's first argument is what the host routes on, so there is no separate dispatch interface — host-mediated dynamic linking ([RFC-56](rfc-56-runtime-move.md)).

Judgment is Omnia's `wasi-model` host (`eval`), imported by guests as an upstream host interface ([RFC-52](rfc-52-effect.md) / [RFC-53](rfc-53-wasi-model.md)); it is not part of this package. `resolve` is the only adapter export the model backend calls back into.

## Data model

- A `claim` carries a required `synopsis` and an optional `backing` (`payload(string)` | `path(string)`); an absent `backing` is the third state. `weight` (`directive` | `specification` | `observation`) drives conflict resolution. Adapters extract facts; cross-source reconciliation, categorization, and joining are the synthesis engine's job, not the adapter's.
- Build I/O is content-addressed and node-independent: inputs cross as the `input` variant; a build's result is a `changeset` against a base `revision`, extracted from the working tree by the caller; the mutable tree is a `working-tree` capability, not a path ([RFC-52](rfc-52-effect.md) / [RFC-55](rfc-55-working-tree.md)). Neither `build` nor `merge` returns the delta — the `report` carries only judgment.

## Versioning

`specify` publishes `augentic:specify@<semver>` via `wkg publish`; `specify-adapters` consumes it as a pinned dependency. The host advertises the world versions it supports.

## Scope

- Author the package and wire `wasmtime::component::bindgen!` host bindings against it.
- Land the claim / `weight` data model in `schemas/evidence.schema.json`, the engine DTOs, and the dependent `slice/model`, `slice/provenance`, and `plan` schemas; align the `specify-adapters` `extract` briefs.
- Publish and pin.

## Acceptance criteria

1. The `augentic:specify` package defines every operation's records and per-axis signatures; the generated bindings match the authored records.
2. The claim / `weight` data model is landed across schemas, engine, and `extract` briefs.
3. The package is published via `wkg publish` and resolvable by `specify-adapters`.
4. The package carries no adapter name, taxonomy, or model id.

## Risks and invariants

- The contract stays adapter- and model-agnostic; the records are the single source of truth, and JSON-schema constants are generated from or retired against them downstream ([RFC-52](rfc-52-effect.md)).
- `resolve` is the sole guest-export reentry point for judgment; everything else a guest needs is an import.
