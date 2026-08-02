# RFC-86: Working Trees

> Status: Draft — step 2 of the platform-migration series ([next-stage.md](next-stage.md))
>
> Owns: the value↔tree boundary and everything that polices it — `materialize` / `changes()` over content-addressed `revision` / `changeset` values, dependency-layering, the exclusive working-tree lease, managed slot policy (modes, exact-base, branch, cleanliness), and source/target tree separation.
>
> Absorbs: [RFC-55 Working-Tree Materialization](archive/rfc-55-working-tree.md) (the git-aware `wasi:filesystem` backend) and [RFC-72 Managed Workspace Materialization](archive/rfc-72-materialization.md) (slot policy and the lease) — one capability, mechanics plus policy.
>
> Depends on: [RFC-85](rfc-85-migration-program.md) Part B (intake / immutable source snapshots — the read-side sibling of the writable tree). Consumed by: [RFC-88](rfc-88-concurrent-execution.md) (per-worker trees), [RFC-89](rfc-89-node-sync.md) (the values its planes move), [RFC-90](rfc-90-detached-changes.md) (ephemeral slot population).

## Intent

Make the working tree a capability materialized from values, never a place the workflow assumes. A `build` or `merge` receives a tree materialized from a content-addressed base `revision` (plus any not-yet-merged dependency `changeset`s), mutates it under an exclusive lease, and the host extracts the inverse `changeset` — so operations stop being pinned to one directory on one machine, and everything concurrent or distributed downstream of this RFC composes values instead of sharing mounts.

One-line summary:

```text
materialize(revision [, changesets…]) → leased tree (descriptor + local-path);
work happens in the tree; changes() → changeset (delta vs base);
values are the only thing that crosses operations or nodes.
```

## The model

Two directions across the value↔tree boundary:

- **`materialize`** — a `revision` (+ optional dependency `changeset`s) → a node-local working tree: a `wasi:filesystem` descriptor for deterministic guest code, and a `local-path` for the spawned-agent (cursor) model backend. An absent `local-path` is the typed signal that agent-driven operations are unavailable on this node.
- **`changes()`** — a node-local tree → a `changeset` (adds / modifies / deletes vs `base`), extracted by native orchestration (`git diff` against `base` in the git backend). Neither `build` nor `merge` returns the delta; reports carry judgment only, and the host extracts the changeset from the tree.

**Dependency-layering**: a dependent operation materializes `base` with its producers' un-merged `changeset`s applied on top, each layer anchored by its `changeset.base` — the value-composition replacement for a shared checkout.

The workflow consumes one deployment-neutral capability (per the archived RFC-72 shape):

```text
ensure(project, requested-base, purpose) → working-tree lease
inspect(project)                         → materialization status
release(lease, outcome)
```

## Decisions

| # | Decision | Consequence |
| - | -------- | ----------- |
| D1 | **Values are the only connective tissue.** No live handle, shared mount, or descriptor crosses operations or nodes; continuity is the slice id plus `revision` / `changeset`. `revision → tree → changeset` must round-trip faithfully. | `build` and `merge` can run on different nodes; whole-operation replay is possible; every distributed design downstream inherits the invariant instead of re-arguing it. |
| D2 | **One exclusive lease per writable tree.** Locally an advisory lock file plus cleanliness classification (no expiry reaper; recovery is the explicit `lease recover` path); the expiry field exists for hosted backends. Lease records live out of tree; the journal carries the audit trail; a journal event never grants ownership by itself. | No two writers ever hold one tree. Serial local work pays for a lock file, not a distributed lease system; hosted execution (RM-18) binds the same contract unchanged. |
| D3 | **Materialization policy is a binding, not the capability.** The capability (`ensure` / `inspect` / `release`) is neutral; its first policy binding is registry slot modes (`operator` — validate, never create; `managed` — create and refresh under declared base/branch policy; `external` — reserved for a hosted lease with no local slot), and its second is [RFC-90](rfc-90-detached-changes.md)'s ephemeral change-directory slots. | `operator` stays the default — existing workflows see no behavior change. Detached mode arrives later as a policy layer, not a fork of the mechanics. |
| D4 | **Exact base before mutation; deterministic branches.** Managed preparation resolves the declared base ref, records the exact commit, creates the change branch (`<prefix><change>/<project>`), verifies cleanliness, and acquires the lease — before any workflow write. Emery never implicitly resets an operator-owned tree. | Every execution records an exact base revision — the anchor `changes()`, layering, re-entry, and [RFC-91](rfc-91-cross-repo-changesets.md) verification all key on. |
| D5 | **Cleanliness is classified, not guessed.** Closed states: clean-at-base, clean-on-expected-branch, dirty-explained-by-active-slice, dirty-unaccounted, base-drifted, branch-diverged. Only the first three proceed; the rest stop with a structured diagnostic and an explicit recovery verb. Re-entry resumes only when lease, branch, base, and journal agree. | Interrupted work is resumable without being destructive; an unaccounted dirty tree can never be silently absorbed or reset. |
| D6 | **Source snapshots and target trees are separate capabilities.** Source adapters read immutable snapshots (RFC-85 B8); target adapters write leased trees; a repository that is both source and target gets both, pinned to the same commit unless the program declares otherwise. No source adapter receives the target tree preopen. | Evidence integrity survives in-place migration: extraction stays reproducible while generation mutates the tree. |
| D7 | **Git is the first backend, behind the neutral abstraction.** Resolve slice → base revision from durable plan/journal state; ensure objects (clone, fetch, or a `wasi:blobstore` object cache); `git worktree add` into node-local scratch; lend; extract; tear down or cache. Object-store snapshot and copy-on-write / overlay backends are drop-ins later. Git stays native host code — no in-guest VCS. | The `revision` / `changeset` types already in `emery:adapter` gain their backend. Omnia's runtime core stays domain-agnostic: slice/revision logic lives in the Emery backend and native orchestration. |

## Phased delivery

### Phase A — sync, inspect, and the lease

Local git clone/fetch into slots, cleanliness classification, `prepare` / `release` with the advisory lease, exact-base recording, deterministic branches. `operator` mode default; `managed` opt-in per entry. This alone removes the biggest operator friction in RFC-85's serial program.

### Phase B — the value boundary

`materialize` / `changes()` proper: trees created from a `revision` into node-local scratch, changesets extracted against the recorded base, round-trip tested. Single node; no transport.

### Phase C — dependency-layering

Materialize `base + [changesets…]` for dependent slices and for [RFC-88](rfc-88-concurrent-execution.md) Stage C workers; deterministic layering anchored on `changeset.base`; conflict on apply is a typed error, not a silent merge.

### Phase D — hosted backend

Opaque working-tree leases over hosted clones (RM-18), durable lease ownership and recovery, same capability contract. Object-store / CoW backend variants as evidence demands.

## Non-goals

- Publication — push, PRs, forge writes ([RFC-90](rfc-90-detached-changes.md) adds one provisioning verb; [RFC-91](rfc-91-cross-repo-changesets.md) verifies; the operator publishes).
- Concurrent writers in one tree — concurrency is separate trees (this RFC) or partitioned ownership within one ([RFC-88](rfc-88-concurrent-execution.md) D3).
- Moving lifecycle authority: merge orchestration remains the only commit writer; the materializer prepares branches but never commits workflow content.
- Replacing git with an Emery version-control model.
- Value transport between nodes — [RFC-89](rfc-89-node-sync.md) owns how values move; this RFC owns what they are.

## Acceptance criteria

1. A managed entry can be synced, inspected, prepared (exact base + branch + lease), and released through the CLI; `operator` mode registries see no behavior change.
2. One tree cannot have two live leases; recovery is explicit and validates tree, branch, base, and journal before changing ownership; an unaccounted dirty tree blocks without being reset.
3. `materialize` from a `revision` and `changes()` against it round-trip faithfully; a build's tree re-materializes on a second machine from values alone.
4. A dependent slice builds against `base` layered with an un-merged producer changeset.
5. `local-path` is present on nodes with a real checkout and absent elsewhere, and the absence is a typed capability signal.
6. Source snapshots and target trees are verifiably separate (no source preopen on the target tree); a both-source-and-target repository round-trips a full slice.
7. `cargo make ci` green; lease, cleanliness, and layering matrices covered as crate-level integration tests over local fixture repositories.

## Open questions

- Local slot mechanics: full clones, shared-object clones, or worktrees from a bare mirror?
- Lease scope under concurrent plan entries ([RFC-89](rfc-89-node-sync.md) D8): one lease per project per change, or per entry with layering?
- Base-branch advance during a long-running change: re-pin policy and its interaction with [RFC-91](rfc-91-cross-repo-changesets.md) verification.
- Changeset encoding: git patch/bundle vs a path→blob index — decided with [RFC-89](rfc-89-node-sync.md)'s transport evidence.
- Cache / GC retention for materialized trees and its relation to `emery archive prune`.
