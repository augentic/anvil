# RFC-52: Effect Interfaces

> Status: Reviewed · Implements: Effect-oriented architecture · Depends: RFC-51 · Sequences into: RFC-54 · Judgment: [RFC-53](rfc-53-tool-server.md) (the native tool-use loop; record/replay at the `ModelClient` boundary)

## Abstract

This RFC defines explicit, typed WIT interfaces for the runtime's *deterministic* effects: the `wasi:filesystem` accessors, the working-tree capability, and the adapter-exported `references` shelf (`resolve(id) → bytes`). Note that `kv` (host-held state) and lifecycle hooks (`journal` / `transition`) are provided by the Omnia runtime as `KeyValue` and `JsonDb` WIT worlds, respectively. **Judgment is not one of these effects.** It runs through a native tool-use loop ([RFC-53](rfc-53-tool-server.md)), and the determinism this RFC seeks — record/replay — lives at the native `ModelClient` boundary, not a WIT `eval` import. The goal here is to make the *deterministic* boundaries explicit, typed, and mockable.

## Motivation

Defining the deterministic effects as typed WIT interfaces:

- Creates a single, typed surface for filesystem access, working-tree materialization, and reference resolution.
- Replaces argv conventions and embedded JSON-schema constants with generated, type-checked records.
- Provides a stable import for the orchestration components and the native loop to build against.

Making LLM runs deterministic in CI — the **replay** goal — is served at the native `ModelClient` boundary ([RFC-53](rfc-53-tool-server.md)), not a WIT import. That keeps Omnia core free of any model knowledge (law 2 at the floor) while preserving whole-operation replay.

## Scope

**In scope:**

- The `references` **shelf** interface — `resolve(id) → bytes`, a stateless adapter export authored in [`../wit/specify.wit`](../wit/specify.wit) that the native loop calls to follow a brief's internal references; this is now a real guest export, not a `wasi:filesystem/preopens` fallback.
- The `working-tree` capability over `wasi:filesystem` — a host-materialized mutable project tree (a `descriptor` for guest code, and a host-reported `local-path` for the filesystem-capable spawned-agent strategy) — and the content-addressed `change-set` records that carry a build's result across nodes to `merge`.
- Host-side handlers that satisfy these interfaces using existing machinery.
- Typing the agent handoff by projecting request/report records into handoff envelopes, replacing hand-maintained JSON schema constants.

**Out of scope:**

- **Judgment.** Out of scope here: judgment is the native tool-use loop, and record/replay rides the `ModelClient` boundary ([RFC-53](rfc-53-tool-server.md)), not a WIT effect. No `eval` interface is defined.
- Orchestration components (the native loop owns judgment sequencing; deterministic tool dispatch is [RFC-54](rfc-54-orchestration.md)).
- Async ABI commitments (e.g., streaming or cancellation).
- Brief frontmatter expansion.

## The model

The authoritative WIT interfaces are defined in [`../wit/specify.wit`](../wit/specify.wit) — the per-axis `target` / `source` worlds and the `references` shelf each adapter exports. There is no `eval` WIT: judgment is the native tool-use loop ([RFC-53](rfc-53-tool-server.md)).

The native loop hands the model a whole brief; passing a `path` (not the brief body) keeps context budgets lean. A filesystem-capable spawned-agent strategy resolves the brief's links itself, while an API model emits `resolve(id)` tool calls that the loop forwards to the adapter's `references` shelf — the same id space, served as a real guest export rather than a `preopens` fallback.

**wasi:filesystem narrows the blast radius.** The `wasi:filesystem/preopens` accessor restricts adapters to exact capabilities via preopens. They govern input artifacts and assets, but explicitly do **not** handle an adapter reading its own bundled prose (handled via relative paths or the `references` shelf).

**The working tree is a host-materialized capability.** `build` does not generate into a green field; it edits a pre-existing project tree in place. Rather than passing a bare `project-path` string — a node-local shared-disk assumption — the host materializes the tree from a content-addressed base `revision` and lends the operation a `working-tree` capability. It exposes a `wasi:filesystem` `descriptor` for deterministic guest reads, and the host provisions a node-local `local-path` for the one consumer a descriptor cannot reach: the filesystem-capable **spawned-agent** strategy ([RFC-58](rfc-58-eval-fleet.md)), which reads existing code and writes changes through real OS paths. That strategy's local read-modify-write loop is quarantined between two portable boundaries — a materialized tree in, and a content-addressed `change-set` (a delta against `base`) out — so the operation no longer depends on a shared mount and can be dispatched to any node. The delta is not returned by the operation: the `report` carries only judgment (status + findings), and the native orchestration layer extracts the `change-set` from the tree (a `git diff` against `base`). `build` and `merge` use the capability symmetrically — `build` is lent the slice tree and the caller extracts its delta; `merge` is lent the *baseline* tree and folds a `change-set` into it in place (a 3-way merge against `delta.base`). The backend that satisfies the capability — materializing the tree from a `revision`, resolving `slice → revision`, and layering out-of-sequence dependencies — is [RFC-55](rfc-55-working-tree.md): a **custom git-aware backend behind Omnia's existing `wasi:filesystem` host** (native code, so git stays native). Unlike judgment — which is native and adds no host at all ([RFC-53](rfc-53-tool-server.md)) — the working tree also adds **no new host**; it rides an existing one. See [architecture.md](architecture.md#the-working-tree).

**Typing the operation handoff.** Projecting the structured request/report records into the brief handoff types the live envelope: the native loop serializes the build request into the handoff and validates the model's answer against the WIT-derived report type, allowing us to remove duplicate JSON schema constants and parity tests.

## Phased plan

1. **Done — interfaces defined.** The deterministic effect interfaces and the `references` shelf are authored in `wit/specify.wit`; remaining work is wiring the host bindings and asserting no behavior change.
2. Implement the host handlers over existing machinery.
3. Project request/report records into live handoff envelopes and retire the JSON schema constants against the package.
4. Record/replay lands at the `ModelClient` boundary in [RFC-53](rfc-53-tool-server.md), not here; a single operation replaying deterministically in CI is proven there.

## Acceptance criteria

1. The deterministic effect vocabulary (and the `references` shelf) exists as typed WIT interfaces.
2. Record/replay is proven at the `ModelClient` boundary ([RFC-53](rfc-53-tool-server.md)), making at least one operation deterministic end-to-end — no `eval` stub required.
3. No runtime behavior changes; existing checks pass.
4. Every effect passes handles/references—no bodies or artifacts cross as inlined values.
5. Agent handoff envelopes are typed against structured records; JSON schema constants are retired or generated from WIT.

## Risks and invariants

- **Pivot must be behavior-neutral:** Only names what exists.
- **No corpus across the boundary:** the native loop hands the model a brief `path`; `references` is pull-by-id through the shelf.
- **Adapter agnostic:** Effect interfaces are generic—no adapter name, taxonomy, or LLM vendor in the contract. The model id lives behind the native `ModelClient` boundary ([RFC-53](rfc-53-tool-server.md)), never in a WIT effect.
- **The agent path is a bounded escape hatch:** the host-provisioned node-local path (`local-path`) deliberately exposes a real OS path to the filesystem-capable spawned-agent strategy, because an external agent cannot hold a Wasm `descriptor`. It is scoped to one node and one operation and bracketed by the materialized tree in and the `change-set` out — not a reintroduction of the global shared disk.

