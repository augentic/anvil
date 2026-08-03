# RFC-89: Node Sync

> Status: Draft — step 5 of the platform-migration series ([next-stage.md](next-stage.md)); bones only — mechanics deferred to the RFCs it composes
>
> Owns: the coordination fabric for one change executing across several nodes: the three sync planes and their separation, the values-only transport posture between working trees, the exclusive tree lease, the round-boundary convergence rhythm that defines "near realtime", the transport-neutral value plane the deployment binds, concurrent execution of one plan's independent slices against the same codebase, and the trial-integration gate whose findings measure the build's overall quality.
>
> Depends: [RFC-86](rfc-86-working-trees.md) (`materialize` / `changes()` and the lease over the value↔tree boundary — this RFC moves the values, it does not redefine them), [RFC-87](rfc-87-verify-profiles.md) (the verify the trial-integration gate runs), [RFC-88](rfc-88-concurrent-execution.md) D4/D6 (the worker pool and the distributed deployment expression this fabric carries).
>
> Related: [RFC-91](rfc-91-cross-repo-changesets.md) (orthogonal — that RFC binds one change's *publication* across repositories on the forge; this RFC coordinates one change's *execution* across nodes before anything is published), [RM-18](roadmap.md#rm-18-cloud-hosted-execute-loop) (the hosted execute loop is the first deployment that needs this fabric).

## Intent

Let one change execute across several nodes — desktop peers, a hosted fleet, or a mix — with near-realtime coordination and without ever sharing a filesystem. Three planes with different consistency needs are kept separate: **coordination** (who is doing what — leases, plan status, journal events), **convergence** (the code itself — `revision` / `changeset` values moving between private trees), and **publication** (branches and PRs on the forge, unchanged and operator-owned).

"Near realtime" is defined honestly: dependents observe a producer's work at round boundaries — when a judgment leg completes and its changeset is extracted — not at keystroke granularity. That matches how the spawned-agent backend already works (cold spawn per leg), so the fabric adds coordination without changing the execution model.

## Decisions

| # | Decision | Consequence |
| - | -------- | ----------- |
| D1 | **The three planes stay separate.** Coordination state never rides the value plane; code never rides the event stream; publication stays forge-side. | Each plane picks its own transport and consistency model. A dashboard needs only the coordination plane; a worker needs only the value plane plus its own lease. |
| D2 | **Values-only between trees.** The only things that cross nodes carrying code are `revision` and `changeset` ([RFC-86](rfc-86-working-trees.md)); no shared volume, no network filesystem, no live descriptor over the wire. | Every node's tree is private, disk-backed, and lendable to the spawned-agent backend as a real `local-path` — the cursor backend is unchanged. Shared volumes are rejected below. |
| D3 | **One exclusive lease per working tree**, held in the control plane with a fencing token. A node materializes only under a lease; a lease is stolen only through explicit recovery, never by timeout alone. | No two writers ever hold the same tree. [RFC-88](rfc-88-concurrent-execution.md) D3's write-ownership manifests partition *within* a tree; the lease partitions *between* trees. Policy shape is [RFC-86](rfc-86-working-trees.md) D2's lease, control-plane-held. |
| D4 | **Convergence happens at round boundaries.** A worker publishes its `changeset` when its round ends (judgment leg complete, `changes()` extracted); dependents re-materialize with the new layer. There is no sub-round sync. | "Near realtime" is honest: latency equals round length plus transport. Keystroke-level sync is a non-goal (D7). |
| D5 | **The value plane is a bound backend, transport-neutral.** Candidates: iroh (blobs + tickets — content-addressed, peer-to-peer, no central store required), NATS + object store, or plain S3. The choice is deployment configuration; nothing in `emery:adapter` or the engine guest names the transport. | Desktop-to-desktop peers and a hosted fleet bind different transports over the same contract, per the architecture's swappable-backend law. Iroh's blob hashes align naturally with the content-addressed `revision` / `changeset` values. |
| D6 | **The coordination plane is a journal projection.** The journal stays the single durable write authority; near-realtime observation is a streamed projection of it (plus lease and plan-status reads), never a second writable store. | No dual-write. Dashboards, waiting workers, and `emery plan status`-style surfaces consume one taxonomy. Hosted replicas share the control plane, not each other's filesystems. |
| D7 | **Live multi-writer file sync is out of contract.** No CRDT tree, no synced editor buffers, no two agents writing one path concurrently across nodes. | Concurrent work on one change is expressed as partitioned ownership within a tree ([RFC-88](rfc-88-concurrent-execution.md) D3) or separate trees composed by changeset layering ([RFC-86](rfc-86-working-trees.md)) — both deterministic, both verifiable. |
| D8 | **Independent plan entries build concurrently, same codebase included.** Entries with no `depends-on` path between them are eligible to build in parallel, each on its own lease from the same base `revision` (a dependent entry materializes `base` layered with its producers' changesets). Scheduling is deterministic control-plane logic; `emery slice merge` stays the serial single writer of the baseline and of per-entry `done`. | The plan's existing dependency graph *is* the concurrency declaration — no second scheduling surface. Lifecycle authority does not move; parallelism ends at the merge gate. |
| D9 | **A trial-integration gate measures build quality continuously.** At round boundaries the orchestrator composes the outstanding changesets of in-flight entries — in `depends-on` topological order — into a candidate baseline, runs verify ([RFC-87](rfc-87-verify-profiles.md)) over it, and emits the findings on the coordination plane (D6). The gate is advisory ahead of the real merge; the merge-queue pattern (Bors, Zuul gating) applied inside one plan. | Integration health becomes an observable, not a surprise at merge: conflict and joint-verify findings arrive while the slices are still building, and a clean composition is journaled as a positive signal. |
| D10 | **Disjointness over smallness.** Concurrent entries are made safe by disjoint write ownership, not by shrinking slices: `plan author` derives a per-slice write manifest ([RFC-88](rfc-88-concurrent-execution.md) D3's ownership manifests lifted to plan level), validates that parallel entries do not overlap, and turns an unavoidable overlap into a `depends-on` edge or a plan-time rejection. The slice grain — one coherent behavioral unit with its own spec — is unchanged. | Changesets get *more frequent* (D4's round-boundary publication feeds D9), not necessarily smaller. Per-slice overhead (refine, synthesis, judgment legs) puts a cost floor under slice size that this RFC respects rather than fights. |

## Lifecycle sketch

```text
control plane                       node A (entry: billing-api)        node B (entry: mobile-shell)
─────────────                       ────────────────────────────       ────────────────────────────
plan approved; entries eligible
  ├─ lease(billing-api) → A
  └─ lease(mobile-shell) → B
                                    materialize(base)                  materialize(base)
                                    …judgment rounds…                  …judgment rounds…
                                    round ends → changes()             round ends → changes()
  ◄─ changeset α published          ─┘                                  │
  ◄─ changeset β published          ────────────────────────────────────┘
trial integration: base + α + β
  → verify → findings on
    coordination plane
                                    (repair round if owned finding)    (repair round if owned finding)
serial merge gate: emery slice merge, one entry at a time → per-entry done
```

## Rejected alternatives

- **Shared volume / network filesystem** — reintroduces coupled failure domains, locking semantics, and location dependence; breaks the `local-path` lending model.
- **CRDT-synced live tree** — solves a problem the round-boundary rhythm doesn't have, at the cost of unverifiable intermediate states.
- **Coordination via the value plane** (e.g. control records as blobs) — collapses D1; coordination needs ordering and liveness, not content addressing.
- **A second event store beside the journal** — dual-write drift; the journal projection (D6) is strictly simpler.

## Phased delivery

- **Phase A — Coordination plane.** Journal streaming projection + control-plane lease service; single node still executes everything. Observability lands first.
- **Phase B — Two-node changeset handoff.** One producer, one dependent, values over a bound transport; proves D2/D4 end to end.
- **Phase C — Swarm binding.** [RFC-88](rfc-88-concurrent-execution.md) Stage C workers bind the value plane; per-worker trees on remote nodes.
- **Phase D — Concurrent plan entries + trial integration.** D8–D10: plan-level manifests, parallel eligibility, the advisory gate and its finding taxonomy.

## Open questions

- Where does the control plane live in a pure desktop-peer deployment (embedded iroh gossip vs a rendezvous service)?
- Lease recovery when a node dies mid-round: who arbitrates, and what does the fencing token invalidate?
- The journal follow surface: streamed projection protocol and its auth story.
- Changeset transport size in practice — is patch-shaped enough, or do large binary assets need blob chunking?
- Which trial-integration findings block the merge gate vs stay advisory?
- Write-manifest derivation for non-Omnia targets (vectis shells, contracts) — what grounds plan-level disjointness there?
