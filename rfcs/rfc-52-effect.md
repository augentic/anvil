# RFC-52: Effect Interfaces

> Status: Draft · Order 2 of 8 · Stage S2 · Depends: [RFC-51](rfc-51-adapter-wit.md) · Enables: [RFC-53](rfc-53-wasi-model.md), [RFC-54](rfc-54-orchestration.md), [RFC-55](rfc-55-working-tree.md), [RFC-56](rfc-56-runtime-move.md) · Owns: the typed effect imports

## Abstract

Every capability a guest needs from the outside is a typed WIT effect it imports. This RFC names them: `wasi:filesystem` (inputs, assets, and the working-tree capability), `wasi:keyvalue` (host-held state), the lifecycle hooks (`journal` / `transition`), the adapter-exported `references` shelf, and Omnia's `wasi-model` host (`eval`). Naming the effects as typed interfaces replaces argv conventions and embedded JSON-schema constants with generated, type-checked records, and gives the guests and the model backend a stable surface to build against.

## The effects

- **`wasi:filesystem`** — capability-scoped access to input artifacts and assets, and the **working-tree** capability: a host-materialized mutable project tree exposing a `descriptor` for guest code and a host-reported `local-path` for the filesystem-capable spawned-agent backend. The git-aware backend that materializes it is [RFC-55](rfc-55-working-tree.md).
- **`wasi:keyvalue`** (`state`) — host-held scratch and memoization (a computed reference; a model session's base + accumulating edits). Filesystem-backed locally, Redis / NATS for fleet-shared state. Provided by Omnia.
- **lifecycle** (`journal` / `transition`) — the durable lifecycle log and its legal moves, over Omnia's `JsonDb`. Authority lives in this host service, never in a guest.
- **`references`** — the adapter shelf (`resolve(adapter-id, reference) -> bytes`, [RFC-51](rfc-51-adapter-wit.md)): a real guest export served from prose embedded in the module at build time, not a `wasi:filesystem/preopens` fallback.
- **`wasi-model`** — Omnia's judgment host: `eval(prompt) -> result<answer, error>`, imported by the workflow (and any guest needing judgment). The model backend behind it is [RFC-53](rfc-53-wasi-model.md); the model id lives in that backend, never in the contract.

`eval`'s interface is Omnia-owned (like `wasi:keyvalue`), so the `augentic:specify` worlds gain it as an upstream import once the Omnia dependency is pinned; the `references` shelf and the per-axis operations stay in `augentic:specify` ([RFC-51](rfc-51-adapter-wit.md)).

## Content-addressed build I/O

`build` edits a pre-existing tree in place. The host materializes that tree from a base `revision` and lends a `working-tree` capability; the operation reads and writes through it; the caller extracts the result as a `changeset` against `base`. `build` is lent the slice tree; `merge` is lent the baseline tree and folds a `changeset` into it. What crosses between nodes is the content-addressed `changeset`, never a shared mount — so `build` and `merge` can run on different nodes ([RFC-55](rfc-55-working-tree.md)).

## Scope

- Author the effect interfaces and the `references` shelf in `wit/specify.wit`; wire the host bindings.
- Implement host handlers over existing machinery (behaviour-neutral).
- Project the typed request/report records into the operation handoff, retiring the hand-maintained JSON-schema constants.

## Acceptance criteria

1. The effect vocabulary (`wasi:filesystem` + working tree, `wasi:keyvalue`, lifecycle, `references`, `wasi-model`) exists as typed imports/exports.
2. Every effect passes handles/references — no bodies or artifacts cross as inlined values.
3. Operation handoffs are typed against the records; JSON-schema constants are retired or generated from WIT.
4. No runtime behaviour changes; existing checks pass.

## Risks and invariants

- **Handles, not corpora.** A brief crosses as a `path`; references resolve by id through the shelf; build I/O crosses as `revision` / `changeset`.
- **Prose ships in the module.** The `references` shelf resolves from embedded prose; no host filesystem capability serves an adapter's own briefs. A prose preopen would re-couple resolution to a granted root and break the pure-wasi path.
- **Agnostic contract.** No adapter name, taxonomy, or model id in any effect; the model id stays in the `wasi-model` backend ([RFC-53](rfc-53-wasi-model.md)).
