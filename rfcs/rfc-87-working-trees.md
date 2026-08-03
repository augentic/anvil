# RFC-86: Local Working Trees

> Status: Draft — step 1 of the platform-migration series ([platform.md](platform.md))
>
> Owns: the single-node value↔tree boundary — `materialize` / `changes()` over content-addressed `revision` / `changeset` values, read-only source grants and snapshots, the exclusive local working-tree lease, exact-base and cleanliness policy, and source/target tree separation.

## Intent

Implement the working tree Emery's WIT already promises: **immutable snapshot trees** for sources and leased **writable trees** for targets, both routed beneath the deployment's existing mounts (D8).

The contract defines `revision` (a content-addressed snapshot identity) and `working-tree` (the tree a build or merge operates on). Every `build` / `merge` dispatch carries a `working-tree` argument. Today that argument is always the placeholder `WorkingTree::live()`: whatever directory the operator invoked Emery from, in whatever state it happens to be.

This RFC replaces the placeholder with real mechanics:


| Today                                                                   | After this RFC                                                                                                       |
| ----------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------- |
| `base: "live"` — the tree is whatever state the invoked directory is in | `base` is an exact recorded commit; the tree is reconstructed on demand                                              |
| The operator's checkout is the execution environment                    | The host materializes a scratch worktree from its own bare mirror, leases it to one owner, and removes it at release |
| Source extraction and target generation share one live directory        | Source adapters read immutable snapshots; target adapters write a separate leased tree                               |
| An operation's edits live only in the directory it mutated              | `changes()` extracts the edits as a content-addressed `changeset` that can be stored and reapplied                   |


The loop, in short:

```text
recorded revision
    → materialize
    → exclusively leased local worktree
    → run operation
    → changes() against the recorded revision
    → changeset
    → release worktree
```

The revision and changeset carry continuity after an operation ends. The retained mirror is just a cache.

**Why now.** Every later step of the series consumes this primitive: [RFC-87](rfc-87-detached-changes.md) materializes per-change slots from it, [RFC-90](rfc-90-concurrent-execution.md) gives each concurrent worker its own tree, and [RFC-91](rfc-91-node-sync.md) moves the settled values between nodes.

The serial loop wins today too. Builds anchor to an exact base instead of ambient directory state. Evidence stays stable while generation writes. Interrupted work resumes through classified recovery instead of manual cleanup.

## The model

Four concepts:

- A **revision** identifies one immutable source tree. For Git repositories it is an exact commit.
- A **working tree** is the mutable, local projection of a revision.
- A **changeset** is the complete adds / modifies / deletes delta between a working tree and its recorded base revision.
- A **lease** gives one owner exclusive use of one materialized writable tree. The lease carries ownership; the journal carries workflow state and audit events.

Two backend transformations cross the value↔tree boundary:

- `materialize` — one `revision` → a local working tree beneath the anchored mount: deterministic guest code resolves it through the shared `.` preopen by `working-tree.subpath`, and the spawned-agent (cursor) model backend receives a `local-path` lend of the same directory — two views of one tree (D8).
- `changes()` — a local working tree → one `changeset` against its recorded base. The Git backend extracts through a temporary index, so untracked additions, deletions, empty files, and binary files are all included.

The workflow never touches those mechanics directly. It consumes one deployment-neutral capability:

```text
ensure(project, requested-base, purpose) → working-tree lease
inspect(project)                         → materialization status
release(lease, outcome)
```

`ensure` creates the exact-base worktree and takes its lease. `release` drops the lease and removes the worktree. Re-entry validates the recorded base, branch, lease, journal, and tree state, preserving unexplained work for explicit recovery.

Nothing else moves. Build and merge reports still carry judgment. Native orchestration captures or applies the changeset around those calls. Merge still owns folding the result into the baseline. The materializer prepares trees and branches — nothing more.

This RFC's **changeset** is a tree-level delta. It is distinct from RFC-88's **publication set**, the forge-side record grouping one change's branches and pull requests across repositories.

### Source-tree discipline

A path-bound source is a read capability scoped to one canonical root. The host canonicalizes the requested directory, keeps symlink resolution inside it, and lends a read-only grant. The persisted binding keeps the operator's locator; every operation re-resolves it through the same containment policy.

Phase A requires source and target roots to be disjoint, reporting `plan-source-tree-overlap` (exit 2) otherwise. Phase C lifts that restriction: the source adapter reads an immutable snapshot while the target adapter writes a separate tree.

## Decisions

- **D1 — Values are the operation boundary.** Live handles and descriptors expire with the operation. Continuity is the slice id plus `revision` / `changeset`, which round-trip faithfully in a fresh materialization root — the settled contract RFC-90 composition and RFC-91 transport build on.
- **D2 — One exclusive lease per writable tree.** An advisory lock plus cleanliness classification guards each tree. A lease is held until release or an explicit `lease recover`, which validates the tree before changing ownership. Lease records live out of tree; the journal carries the audit trail.
- **D3 — Every writable tree is materialized.** The host creates an exact-base scratch worktree from its mirror and leases it. No target operation runs in an operator-tended checkout. RFC-87 reuses the same capability for change-local slots.
- **D4 — Exact base before mutation; one branch convention.** Materialization resolves the declared base ref, records the exact commit, creates `change/<plan>` in each repository, verifies cleanliness, and takes the lease — all before any workflow write. RFC-88 observes the same branch name; repository identity disambiguates equal names across members.
- **D5 — Cleanliness has a closed classification.** Three states proceed: clean-at-base, clean-on-expected-branch, and dirty-explained-by-active-slice. Three stop with a structured diagnostic and an explicit recovery verb: dirty-unaccounted, base-drifted, and branch-diverged. Recovery keeps unaccounted changes intact for inspection.
- **D6 — Source snapshots and target trees are separate capabilities.** Source adapters read immutable snapshot trees; target adapters write leased writable trees — both routed per D8. A repository used in both roles gets both trees, pinned to the same commit unless its approved bindings say otherwise. Extraction stays reproducible while generation writes.
- **D7 — Git is the writable backend; directory copy is the non-Git source backend.** Writable trees come from a host-owned bare mirror, one linked worktree per lease. Git source snapshots pin a commit; non-Git source directories copy into content-addressed read-only scratch. Both run in native host code.
- **D8 — Trees route beneath the anchored mount; mounts and WIT do not change.** The `.` preopen remains the anchored root (the project root today, the change directory under RFC-87), and every materialized tree — leased worktrees and content-addressed source snapshots — is created under the gitignored `.emery/scratch/`, so a mid-run materialization is guest-visible through the existing preopen set. `working-tree.subpath` names the tree for deterministic guest code; the spawned-agent backend is lent only that subtree as its `local-path` — two views of one directory that cannot drift apart. Local isolation is an audit posture, not a capability boundary: subpath discipline plus lend scoping, enforced by `changes()` extracting exactly the lease's delta and the cleanliness classification flagging out-of-tree writes. A source original outside the anchored root is never guest-visible — host code snapshots it into scratch and guests read only the copy. Per-tree capability preopens are deferred: RFC-91's private per-node trees deliver enforced isolation without a runtime feature.

## Ownership

| Concern | Repo |
| ------- | ---- |
| Materializer, leases, host-side Git backend; the `ensure` / `inspect` / `release` capability trait in `project::seam` | `augentic/emery` (`crates/project`, `crates/native`, `crates/launcher`) |
| Scratch routing, `subpath` dispatch, changeset capture/apply around build and merge | `augentic/emery` (`crates/slice`, `crates/change`) |
| Adapters stop assuming `subpath == None`; prompts anchor the lent tree | `augentic/emery-adapters` |
| Omnia runtime | No change (see Rejected alternatives) |

## Fixed implementation cut

- One host-owned bare mirror per Git repository backs every writable worktree, including RFC-87's ephemeral slots.
- Trees materialize under the anchored root: leased worktrees at `.emery/scratch/<lease>/`, content-addressed source snapshots at `.emery/scratch/<digest>/`. The prefix is already init-managed `.gitignore` state; the enclosing repository's cleanliness classification excludes it, and scratch content never rides a changeset.
- The workspace lend for a target operation is the leased subtree, never the whole mount.
- Lease scope is one writable tree. The lock and lease record live beside the tree.
- A plan pins its base revision until finalize. Base-branch movement produces `base-drifted` and preserves the recorded revision for explicit recovery.
- A Git `changeset` is a SHA-256-addressed binary patch from a temporary index over the complete working tree against its exact base — additions, deletions, empty files, and binary files included. It carries the recorded base required for application.
- The local backend applies one changeset to its recorded base, supporting the serial build → merge path and proving the round-trip.
- Releasing a lease removes its worktree and lease record. The bare mirror remains as the materialization cache.

## Rejected alternatives

- **Dynamic per-operation preopens** (an Omnia runtime feature) — purchases capability enforcement that D8's detection posture already provides; RFC-91's private per-node trees deliver enforced isolation without a runtime change.
- **Changing the WIT `working-tree` record** — breaks RFC-90's "the WIT seam does not change" for no functional gain; `subpath` already routes beneath the shared mount and is unused today.
- **Flat `.emery/<lease>/` tree roots** — generated lease names in the closed top-level `.emery/` namespace; `scratch/` is the reserved, already-gitignored boundary between engine state and regenerable trees.

## Phased delivery

- **Phase A — Source grants and snapshots.** Canonical read-only grants, symlink containment, content-addressed directory copies, pinned Git source snapshots, and disjoint source/target roots.
- **Phase B — Local slots and leases.** Bare-mirror sync, worktree materialization, cleanliness classification, exact-base recording, deterministic branches, `ensure` / `inspect` / `release`, the advisory lease, and explicit recovery.
- **Phase C — The complete value boundary.** `materialize(revision)` / `changes()` over local linked worktrees, binary-patch persistence and application, round-trip in a fresh materialization root, and routing to separate immutable source and writable target trees. This phase enables same-repository source and target roles and completes RFC-86.

## Acceptance criteria

1. The Git backend syncs its mirror, resolves and records an exact revision, materializes the expected branch under a lease, reports status, and releases — all through the public CLI/capability surface. Every writable tree is mirror-backed and host-owned.
2. Lease acquisition establishes exclusive ownership. Recovery validates tree, branch, base, and journal before changing ownership, and preserves unaccounted changes for inspection.
3. `materialize` from a Git revision, `changes()` against it, and application back to the recorded base round-trip additions, modifications, deletions, empty files, and binary files in a fresh materialization root.
4. Local source bindings canonicalize into read-only grants whose symlink resolution stays inside the granted root. Git and non-Git source snapshots remain unchanged while a target tree mutates.
5. Phase A enforces disjoint source and target roots. Phase C gives a both-source-and-target repository separate trees pinned to the same base and round-trips a full slice.
6. The Git backend supplies `local-path`; releasing a lease removes its scratch worktree and retains the mirror.
7. Deterministic guest code (routing by `working-tree.subpath`) and the spawned-agent lend resolve the same materialized tree; the deployment ships no new mount, no Omnia runtime change, and no WIT change.
8. `cargo make ci` is green; source-grant, snapshot, lease, cleanliness, recovery, patch round-trip, and overlap matrices are covered as crate-level integration tests over local fixtures.

