# RFC-52: Effect Interfaces

> Status: Reviewed · Implements: Effect-oriented architecture · Depends: RFC-51 · Sequences into: RFC-53

## Abstract

This RFC defines explicit, typed WIT interfaces for the fixed vocabulary of effects the runtime currently performs implicitly: `eval` (to run a brief or prompt) and the `wasi:filesystem` accessors (which also serve the `references` resolve fallback). Note that `kv` (host-held state) and lifecycle hooks (`journal` / `transition`) are provided by the Omnia runtime as `KeyValue` and `JsonDb` WIT worlds, respectively. It changes no runtime behavior—each named effect is initially backed by existing machinery. The goal is to make the implicit boundaries explicit, typed, and mockable, unlocking deterministic record/replay.

## Motivation

Currently, LLM inference is an out-of-band convention (the CLI prints an envelope, the agent reads a brief) with no typed contract. Defining it as an effect interface:

- Creates a single, typed surface for context-injection, recording, and rate-limiting.
- Enables a **replay stub** to satisfy `eval`, making runs deterministic in CI.
- Provides a stable import for future orchestration components to build against.

## Scope

**In scope:**

- WIT interface definitions for `eval` (in `[../wit/model.wit](../wit/model.wit)`); the `references` resolve fallback is served by `wasi:filesystem/preopens`, not a standalone interface.
- The `working-tree` capability over `wasi:filesystem` — a host-materialized mutable project tree (a `descriptor` for guest code, an optional `local-path` for the agent `eval` backend) — and the content-addressed `change-set` records that carry a build's result across nodes to `merge`.
- Host-side handlers that satisfy these interfaces using existing machinery.
- A record/replay backend for `eval` sufficient for CI.
- Typing the agent handoff by projecting request/report records into handoff envelopes, replacing hand-maintained JSON schema constants.

**Out of scope:**

- Orchestration components (execution remains a two-phase handoff).
- Async ABI commitments (e.g., streaming or cancellation).
- Brief frontmatter expansion.

## The model

The authoritative WIT interfaces are complete and defined in `[../wit/model.wit](../wit/model.wit)` (the `eval` effect) and `[../wit/specify.wit](../wit/specify.wit)` (the per-axis worlds).

The host satisfies `eval` with the existing two-phase handoff; the typed interface is the contract, while the handoff is the temporary implementation. Passing `path` instead of the brief body prevents context-budget blowup. Filesystem-capable backends resolve links themselves, while others use the `references` fallback.

**wasi:filesystem narrows the blast radius.** The `wasi:filesystem/preopens` accessor restricts adapters to exact capabilities via preopens. They govern input artifacts and assets, but explicitly do **not** handle an adapter reading its own bundled prose (handled via relative paths or `references`).

**The working tree is a host-materialized capability.** `build` does not generate into a green field; it edits a pre-existing project tree in place. Rather than passing a bare `project-path` string — a node-local shared-disk assumption — the host materializes the tree from a content-addressed base `revision` and lends the operation a `working-tree` capability. It exposes a `wasi:filesystem` `descriptor` for deterministic guest reads, and an *optional* node-local `local-path` for the one consumer a descriptor cannot reach: the filesystem-capable `eval` backend (the agent), which reads existing code and writes changes through real OS paths. The agent's local read-modify-write loop is quarantined between two portable boundaries — a materialized tree in, and a content-addressed `change-set` (a delta against `base`) out — so the operation no longer depends on a shared mount and can be dispatched to any node. The delta is not returned by the operation: the `report` carries only judgment (status + findings), and the caller extracts the `change-set` from the tree via `working-tree.changes()`. `build` and `merge` use the capability symmetrically — `build` is lent the slice tree and the caller extracts its delta; `merge` is lent the *baseline* tree and folds a `change-set` into it in place (a 3-way merge against `changes.base`). The backend that satisfies the capability — materializing the tree from a `revision`, resolving `slice → revision`, and layering out-of-sequence dependencies — is [RFC-55](rfc-55-working-tree.md): a **custom git-aware backend behind Omnia's existing `wasi:filesystem` host** (native code, so git stays native), distinct from the `eval` model host, which is a sanctioned *new* generic host ([RFC-54](rfc-54-model-host.md)). See [architecture.md](architecture.md#the-working-tree).

**Typing the agent handoff.** Naming the `eval` seam types the live handoff envelope. The host serializes the structured build request into the brief handoff and validates the report against the WIT-derived type, allowing us to remove duplicate JSON schema constants and parity tests.

## Phased plan

1. **Done — interfaces defined.** The effect interfaces are authored and complete in `wit/model.wit` and `wit/specify.wit`; remaining work is wiring the host bindings and asserting no behavior change.
2. Implement the host handlers over existing machinery.
3. Add the `eval` record/replay backend; prove a single operation replays deterministically in CI.
4. Project request/report records into live handoff envelopes and retire the JSON schema constants against the package.

## Acceptance criteria

1. The effect vocabulary exists as typed WIT interfaces.
2. A replay stub satisfies `eval`, making at least one operation deterministic end-to-end.
3. No runtime behavior changes; existing checks pass.
4. Every effect passes handles/references—no bodies or artifacts cross as inlined values.
5. Agent handoff envelopes are typed against structured records; JSON schema constants are retired or generated from WIT.

## Risks and invariants

- **Pivot must be behavior-neutral:** Only names what exists.
- **No corpus across the boundary:** `eval` takes `path(string)`; `references` is pull-by-id.
- **Adapter agnostic:** Effect interfaces are generic—no adapter name, taxonomy, or LLM vendor in the contract.
- **The agent path is a bounded escape hatch:** `working-tree.local-path` deliberately exposes a real OS path to the filesystem-capable `eval` backend, because an external agent cannot hold a Wasm `descriptor`. It is scoped to one node and one operation and bracketed by the materialized tree in and the `change-set` out — not a reintroduction of the global shared disk.

