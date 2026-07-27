# RFC-55: Working-Tree Materialization — the git-aware `wasi:filesystem` backend

> **Status: Deferred.** Every guest runs against the shared `[[mount]]` preopens of the operator's live project tree, so materialized working trees are not needed until a multi-node deployment exists. The `revision` / `changeset` types stay in `emery:adapter` as this RFC's forward hook. Owns: the value↔tree boundary

## Abstract

The contract's `revision` / `changeset` forward hook needs a backend before trees can cross nodes. This RFC specifies it: a **custom git-aware backend behind Omnia's `wasi:filesystem` host** that *materializes* a `working-tree` from a content-addressed `revision` (plus any not-yet-merged dependency `changeset`s) onto whichever node runs an operation, and extracts the inverse `changeset` afterward. It adds **no new host** — it rides `wasi:filesystem` as a backend, keeping git native (host code; no in-guest VCS). `slice -> revision` resolution, `changeset` extraction, and forge push live in the binary's native orchestration layer.

## Why a backend

The architecture's portability bet is that `build` and `merge` can run on different nodes, connected only by content-addressed values. A persistent single checkout blocks that: its identity is a local path, sequence is physical accumulation in one directory, and instance-per-call kills any live handle at the call edge. The connective tissue must be a value, and a value needs a backend that can project it back into a tree.

## The model

Two directions across the value↔tree boundary:

- **`materialize`** — a `revision` (+ optional dependency `changeset`s) -> a node-local `working-tree` (a `wasi:filesystem` `descriptor`, and an optional `local-path`).
- **`changes()`** — a node-local tree -> a `changeset` (a delta vs `base`), extracted by the native orchestration layer (`git diff` against `base`).

The git backend, concretely: resolve `slice -> base revision` from durable plan / journal state; ensure the object store is present (existing clone, native fetch, or hydrate from a `wasi:blobstore` object cache); check the revision out into node-local scratch (`git worktree add`); lend the `descriptor` (and report `local-path` when disk-backed); tear down or cache after. **Out-of-sequence dependency-layering** applies a not-yet-merged producer's `changeset` on top of the base before lending the tree — the value-composition replacement for a shared checkout, each layer anchored by its `changeset.base`.

## Scope

- `materialize` and native `changes()` over the value↔tree boundary.
- `slice -> revision` resolution from durable plan / journal state.
- Out-of-sequence dependency-layering of producer `changeset`s.
- Materialized-tree lifecycle: scratch, teardown, optional cache / GC (and its relation to `emery archive prune`).
- `local-path` provisioning for the spawned-agent (cursor) model backend, and the `none` signal on a node with no local tree.
- Backend variants behind one `revision` abstraction: git first, then object-store snapshot (`wasi:blobstore`) and copy-on-write / overlay.

## Open questions

- Where a slice's base `revision` is recorded, and the resolving verb.
- Dependency-layering ordering and conflict handling; relation to the `depends-on` graph (roadmap RM-11).
- Object-store transport (a `wasi:blobstore` object cache; shallow / partial / sparse checkout policy).
- Cache / GC retention; the `merge` baseline (fresh mainline checkout vs reused built tree) and how `changeset.base` anchors the 3-way apply.

## Acceptance criteria

1. The backend materializes a `working-tree` from a `revision`, and the native layer extracts a `changeset` against it; the two round-trip faithfully.
2. A build's tree re-materializes on a different node from values alone (revision + object store), with no shared mount.
3. A dependent slice builds against a base `revision` layered with an un-merged producer `changeset`.
4. `local-path` is present on nodes with a real checkout and `none` elsewhere, gating agent-driven operations.
5. The Emery runtime binary binds this backend behind `wasi:filesystem`; `cargo make ci` stays green.

## Risks and invariants

- **Values are the only connective tissue.** No live handle or shared mount crosses build->merge or node->node; continuity is the `slice` id plus the `revision` / `changeset` values.
- **Round-trip fidelity.** `revision -> tree -> changeset` must round-trip faithfully — whole-operation replay depends on it.
- **Deterministic layering.** Dependency-layering never silently reorders or drops edits; it is anchored by `changeset.base`.
- **Git unprivileged.** Git is the first `wasi:filesystem` backend, behind the neutral `revision` / `changeset` abstraction, so an object-store or CoW backend is a drop-in.
- **Law 2 in the runtime core.** The generic `wasi:filesystem` / `wasi:blobstore` host stays domain-agnostic in Omnia; slice / revision logic lives in the Emery backend and native orchestration.
