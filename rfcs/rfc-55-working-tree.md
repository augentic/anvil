# RFC-55: Working-Tree Materialization (the Data Backend and the value↔tree boundary)

> Status: Draft (skeleton) · Implements: the effect-oriented architecture (Stage 4 — deterministic effects) · Depends: [RFC-51](rfc-51-adapter-wit.md) (the `working-tree` / `change-set` / `revision` records), [RFC-52](rfc-52-effect.md) (the `working-tree` capability surface) · Companion to: [RFC-56](rfc-56-runtime-move.md) (which binds this as its data backend) · Mirrors: [RFC-54](rfc-54-model-host.md) on the deterministic side

## Abstract

[RFC-51](rfc-51-adapter-wit.md) authored the `working-tree` / `change-set` / `revision` records and [RFC-52](rfc-52-effect.md) named the `working-tree` capability, but both leave it *"backed by existing machinery."* Today that machinery is a **persistent, local, single checkout** in which `refine → build → merge` run synchronously, slice by slice — the working tree's identity *is* its on-disk path, and each slice's edits accumulate physically in one directory. This RFC specifies the real backend that retires that assumption: the **data host backend** that *materializes* a `working-tree` from a content-addressed `revision` (plus any not-yet-merged dependency `change-set`s) onto whichever node runs an operation, and the inverse extraction the capability's `changes()` performs. It is the deterministic-effects mirror of the model host ([RFC-54](rfc-54-model-host.md)): where RFC-54 carved the `eval` backend out of [RFC-56](rfc-56-runtime-move.md) because it *"deserves an explicit contract, not an assumption buried in RFC-56,"* this carves out the data/working-tree backend, so the runtime move **binds** it rather than burying it.

## Motivation

The architecture's portability bet ([architecture.md](architecture.md#the-working-tree)) is that an operation can run on any node and that `build` and `merge` can happen on different nodes and out of order, connected only by content-addressed values. The single-checkout model blocks that on three counts:

- **Identity is a path.** When the working tree's identity is a local directory, nothing connecting two operations can cross a process — let alone a node — boundary. Instance-per-call ([architecture.md](architecture.md) — stateless guests, host-held state) means the handle dies at every call edge, so the connective tissue *must* be a value, and a value needs a backend that can project it back into a tree.
- **Sequence is physical.** Slice N building "on top of" slices 1..N-1 works today only because they share one accreting directory. Remove the shared directory and that prior state has to be carried by the `revision` you materialize from (and, for un-merged dependencies, by layering producer `change-set`s) — not by physical accumulation.
- **The backend is unspecified.** *"The host materializes the tree"* is a single line in [RFC-56](rfc-56-runtime-move.md). It has real surface — `slice → revision` resolution, object-store acquisition, checkout, out-of-sequence dependency-layering, scratch lifecycle, cache / GC — that earns a contract for the same reason the model host did.

## Scope

**In scope:**

- The **`materialize`** operation: a base `revision` (plus optional dependency `change-set`s) → a node-local `working-tree` (a `wasi:filesystem` `descriptor`, and an optional `local-path`), and its inverse — the capability's `changes()` extracting a `change-set` against `base`.
- **`slice → revision` resolution** from durable plan / journal state (where a slice's base is recorded, and the verb that resolves it).
- **Out-of-sequence dependency-layering**: composing a base `revision` with not-yet-merged producer `change-set`s before lending the tree — the multi-node replacement for a shared, accreting checkout.
- **Materialized-tree lifecycle**: per-call scratch, teardown, and optional caching / GC of materialized trees in `kv` / `state` (and its relationship to `specify archive prune`).
- **`local-path` provisioning** for filesystem-capable `eval` backends, and the `none` path on a backend with no real local tree (the RFC-52 capability signal that gates agent-driven operations).
- **Backend variants behind one `revision` abstraction**: git (the first backend), object-store snapshot (S3 / blobstore), copy-on-write / overlay.

### Non-goals

- **The records and the capability surface.** The `working-tree` / `change-set` / `revision` records are [RFC-51](rfc-51-adapter-wit.md); the `descriptor` / `local-path` / `changes` interface and its boundary invariants are [RFC-52](rfc-52-effect.md). This RFC is the backend that *satisfies* them.
- **`eval` and the model fleet.** The judgment backend is [RFC-54](rfc-54-model-host.md) / [RFC-57](rfc-57-eval-fleet.md); this RFC is its deterministic-effects counterpart.
- **The runtime move itself.** Instance-per-call, the component-on-both-axes mandate, and retiring the bespoke host are [RFC-56](rfc-56-runtime-move.md); this RFC supplies one of the backends that move binds.
- **The merge algorithm and conflict UX.** The adapter's `merge` brief owns conflict resolution; this RFC only provides the baseline tree it resolves *within* and anchors the 3-way apply via `change-set.base`.
- **Forge transport.** Branch push, PR / MR creation, and finalize are operator-owned (roadmap RM-17), downstream of a materialized result.

## The cross-repo boundary (who owns what)

| Concern | Owner |
| --- | --- |
| Generic filesystem / object-store / scratch **host services** (the raw plumbing, swappable like KV) | **Omnia** (the generic framework) |
| The `slice → revision` resolution, `change-set` apply, dependency-layering, and the git / worktree projection | **Specify** — this RFC |
| The `working-tree` **capability surface** the backend satisfies | **Specify** — [RFC-52](rfc-52-effect.md) |
| Binding this backend to the data effect during the runtime move | **Specify** — [RFC-56](rfc-56-runtime-move.md) |

Unlike [RFC-54](rfc-54-model-host.md) (mostly Omnia-side), this is **mostly Specify-side**: the raw FS / object access may be a generic Omnia host service, but the slice / revision / change-set projection is Specify-domain logic riding on top. The seam between them is versioned, not released in lockstep.

## The model (sketch)

**Materialization is the inverse of extraction.** The host owns a value↔tree boundary with two directions:

- **`changes()`** (RFC-52): a node-local directory → a content-addressed `change-set` (a delta vs `base`).
- **`materialize`** (this RFC): a content-addressed `revision` (+ optional dependency `change-set`s) → a node-local `working-tree`.

Guests deal only in values; the host *projects* a value into a real directory on whichever node is about to run the operation, lends the `working-tree`, and reclaims it after — *"stateless guests, host-held state"* ([architecture.md](architecture.md)) applied to the filesystem.

**The git backend, concretely.** To materialize a slice tree the host needs *which revision* and *where to get the bytes*:

1. Resolve `slice → base revision` from durable plan / journal state (every `change-set` already carries its `base`, so values are self-describing about their anchor).
2. Ensure the object store is present on this node — the existing clone on a desktop; a fetch / clone (likely shallow or partial) from the canonical remote or an OCI-backed object cache on a fresh node.
3. Check the revision out into a node-local scratch directory — `git worktree add` over one object database is the natural primitive.
4. Wrap the directory as a `working-tree`: a `wasi:filesystem` `descriptor` for `root()`, the scratch path for `local-path()`, the resolved `revision` for `base()`.
5. Lend it (borrowed) for the call, then tear it down — or cache it as an optimization.

**Out-of-sequence dependency-layering.** A slice that depends on a not-yet-merged producer materializes from its base `revision` and then has the producer's `change-set` applied on top before the tree is lent (each layer anchored by its `change-set.base`). This is the value-composition replacement for "they shared a checkout," and the materialization-time counterpart to the `depends-on` slice graph and the dependency gates (roadmap RM-11).

## Decisions to record (open until reviewed)

- **`slice → revision` resolution surface.** Where a slice's base revision is recorded (plan.yaml / slice metadata / journal) and the verb that resolves it.
- **Dependency-layering semantics.** Ordering, conflict handling when layering multiple producer `change-set`s, and the exact relationship to `depends-on` / RM-11.
- **Object-store transport.** Canonical remote vs OCI-backed object cache; the shallow / partial / sparse checkout policy.
- **Cache / GC.** Whether materialized trees are cached in `kv`, the retention policy, and how it relates to `specify archive prune`.
- **The merge baseline.** Whether `merge`'s tree is a fresh mainline checkout (rebased) or the built tree re-used, and exactly how `change-set.base` anchors the 3-way apply.
- **Git leakage.** How much git surfaces in the backend versus stays behind the neutral `revision` / `change-set` abstraction, so a non-git backend remains a drop-in.

## Phased plan

1. Specify `materialize` / `changes()` over today's single local checkout — behaviour-neutral, one node, synchronous (the current world re-expressed through the value↔tree boundary).
2. Add `slice → revision` resolution + content-addressed object access, so a tree can be re-materialized on a fresh node from values alone.
3. Add out-of-sequence dependency-layering; prove a dependent slice builds against an un-merged producer's `change-set`.
4. Add cache / GC; prove a materialized tree survives node loss by re-materializing elsewhere from the `revision`.

## Acceptance criteria

1. The data backend materializes a `working-tree` from a `revision`, and the capability's `changes()` extracts a `change-set` against it; the two round-trip faithfully across the value↔tree boundary.
2. A build's `working-tree` re-materializes on a different node from values alone (revision + object store), with no shared mount.
3. A dependent slice builds against a base `revision` layered with an un-merged producer `change-set`.
4. `local-path` is present on nodes with a real checkout and `none` elsewhere, gating agent-driven operations per the RFC-52 capability signal.
5. [RFC-56](rfc-56-runtime-move.md) binds this backend for the data effect; `make lint` and `cargo make ci` stay green.

## Risks and invariants

- **Values are the only connective tissue.** No live handle and no shared mount crosses build→merge or node→node; continuity is the `slice` id plus the content-addressed `revision` / `change-set` (architecture instance-per-call).
- **Round-trip fidelity.** `revision → tree → change-set` must round-trip faithfully — whole-operation replay (RFC-52) depends on it.
- **Deterministic layering.** Dependency-layering must not silently reorder or drop edits; it is deterministic and anchored by `change-set.base`.
- **Backend-swappable, git unprivileged.** Git is the first backend, not a privileged one; the `revision` / `change-set` abstraction must keep git out of the typed surface so an object-store or CoW backend is a drop-in.
- **Law 2 at the floor.** The generic FS / object plumbing stays domain-agnostic in Omnia; all slice / revision logic lives in the Specify backend.
