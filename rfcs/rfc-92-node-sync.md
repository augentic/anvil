# RFC-91: Node Sync

> Status: Draft — step 6 of the platform-migration series (scale track) ([platform.md](platform.md))
>
> Owns: the complete hosted and multi-node execution binding for one change: hosted materialization of RFC-86 trees, the durable control-plane journal and fenced leases, remote placement of RFC-90 worker pools, the three sync planes and their separation, values-only transport between private trees, round-boundary convergence, concurrent execution of independent plan entries, and the trial-integration gate whose findings measure overall quality.
>
> Depends on completed [RFC-87](rfc-87-detached-changes.md) (change-scoped state and member bindings), [RFC-86](rfc-86-working-trees.md) (local `materialize` / `changes()` semantics and value formats), [RFC-89](rfc-89-verify-profiles.md) (trial-integration verification), and [RFC-90](rfc-90-concurrent-execution.md) (worker pools, ownership, local per-worker trees, and changeset composition).
>
> Related: [RFC-88](rfc-88-publication-sets.md) (orthogonal — that RFC binds one change's *publication* across repositories on the forge; this RFC coordinates one change's *execution* across nodes before anything is published), [RM-18](roadmap.md#rm-18-cloud-hosted-execute-loop) (the hosted execute loop is the first deployment that needs this fabric).

## Intent

Let one change execute across several nodes — desktop peers, a hosted fleet, or a mix — with near-realtime coordination and without ever sharing a filesystem. Three planes with different consistency needs are kept separate: **coordination** (who is doing what — leases, plan status, journal events), **convergence** (the code itself — `revision` / `changeset` values moving between private trees), and **publication** (branches and PRs on the forge, unchanged and operator-owned).

"Near realtime" is defined honestly: dependents observe a producer's work at round boundaries — when a judgment leg completes and its changeset is extracted — not at keystroke granularity. That matches how the spawned-agent backend already works (cold spawn per leg), so the fabric adds coordination without changing the execution model.

Every `changeset` in this RFC is RFC-86's tree-delta value. RFC-88's publication set is a separate forge-side record and never enters the value plane.

## Decisions

| # | Decision | Consequence |
| - | -------- | ----------- |
| D1 | **The three planes stay separate.** Coordination state never rides the value plane; code never rides the event stream; publication stays forge-side. | Each plane picks its own transport and consistency model. A dashboard needs only the coordination plane; a worker needs only the value plane plus its own lease. |
| D2 | **Values-only between trees.** The only things that cross nodes carrying code are `revision` and `changeset` ([RFC-86](rfc-86-working-trees.md)); no shared volume, no network filesystem, no live descriptor over the wire. | Every node's tree is private, disk-backed, and lendable to the spawned-agent backend as a real `local-path` — the cursor backend is unchanged. Shared volumes are rejected below. |
| D3 | **One exclusive lease per working tree**, held in the control plane with a fencing token. A node materializes only under a lease; a lease is stolen only through explicit recovery, never by timeout alone. | No two writers ever hold the same tree. [RFC-90](rfc-90-concurrent-execution.md) D3's write-ownership manifests partition *within* a tree; the lease partitions *between* trees. Policy shape is [RFC-86](rfc-86-working-trees.md) D2's lease, control-plane-held. |
| D4 | **Convergence happens at round boundaries.** A worker publishes its `changeset` when its round ends (judgment leg complete, `changes()` extracted); dependents re-materialize with the new layer. There is no sub-round sync. | "Near realtime" is honest: latency equals round length plus transport. Keystroke-level sync is a non-goal (D7). |
| D5 | **The value plane is a transport-neutral capability with one shipped binding.** Nothing in `emery:adapter` or the engine guest names a transport; this RFC ships NATS JetStream Object Store as the first complete backend. | One deployable path is sufficient for completion. Iroh, S3, or another object backend can implement the settled capability later without becoming RFC-91 phases. |
| D6 | **The coordination plane is the hosted journal plus read projections.** The per-change JetStream stream is the journal backend and single durable event authority; dashboards, waiting workers, and status views project it alongside lease reads. | No second event store and no dual-write. Hosted replicas share the control plane, not each other's filesystems. |
| D7 | **Live multi-writer file sync is out of contract.** No CRDT tree, no synced editor buffers, no two agents writing one path concurrently across nodes. | Concurrent work on one change is expressed as partitioned ownership within a tree or separate trees composed by [RFC-90](rfc-90-concurrent-execution.md)'s deterministic changeset composer — both verifiable. |
| D8 | **Independent plan entries build concurrently; layering is per project.** Entries with no `depends-on` path are eligible in parallel. Entries targeting the same `plan.yaml.projects` row share its approved base and use RFC-90's composer for producer changesets. A dependency across projects orders scheduling but never applies one repository's patch to another. `emery slice merge` stays the serial writer of baselines and per-entry `done`. | The existing dependency graph is the concurrency declaration, while the project key is the composition boundary. RFC-91 owns the graph→per-project-layer projection and does not reopen RFC-90's composer. |
| D9 | **Trial integration is one candidate tree per project.** At round boundaries the orchestrator groups outstanding changesets by project, composes each group in `depends-on` order, runs that project's RFC-89 verify profiles, and emits one aggregated finding set on the coordination plane. Cross-project dependencies affect order only; cross-repository CI is outside this gate. | Integration health is meaningful for multi-repository plans: no impossible `base + patches-from-other-repos` tree, and each finding names its project and owning entry. |
| D10 | **Disjointness over smallness.** `plan author` records a per-slice write manifest and checks overlap only among entries targeting the same project. Unknown or overlapping same-project ownership becomes a `depends-on` edge or rejection; different projects are structurally disjoint. | Changesets become more frequent, not necessarily smaller. Per-slice overhead still sets a cost floor under slice size. |
| D11 | **Hosted trees preserve RFC-86 semantics.** A remote worker resolves a revision and any composed changesets from the value plane, materializes a private disk-backed tree with `local-path`, runs under a fenced lease, publishes `changes()` at the round boundary, and releases the tree. | Hosted execution is a backend binding over completed local semantics, not another tree model. Byte-identical values round-trip locally and remotely. |
| D12 | **A hosted change has one journal backend.** Binding an RFC-87 change to the control plane moves journal authority to its durable stream for the remainder of the run; any local file is a read-only projection. Events receive a monotonic per-change sequence through compare-and-set append. | There is no dual-write or reconciliation protocol. Re-entry from any node observes one lifecycle order, and returning to local execution reads the same projection. |
| D13 | **`plan execute --hosted` is the only attach and resume surface.** In an RFC-87 change directory it attaches the authored plan then executes; with `--change-id` and an empty `--project-dir` it reconstructs the change projection and resumes. | RFC-91 is directly operable without waiting for RM-18's background-submit product surface or adding a second lifecycle command family. |

## Hosted attach contract

The operator surface is:

```text
# First attach, from the authored RFC-87 change directory
emery plan execute --hosted

# Resume from another node into an empty directory
emery plan execute --hosted --change-id <id> --project-dir <dir>
```

The deployment supplies `NATS_URL` and `NATS_CREDS` (a credentials-file path). Emery prints the generated UUIDv7 change id on attach and stores only the endpoint, id, and last observed sequence in `.emery/hosted.yaml`; credentials never enter the change directory or journal.

Attach is an explicit cutover:

1. Refuse unless the detached plan is fully authored, no operation/lease is active, and the local journal tail matches plan status.
2. Append local `plan.hosted.attach-started` with the generated id, create the JetStream namespaces idempotently, and upload the canonical plan/change/slice artifact snapshot to a coordination-artifact bucket separate from revision/changeset values.
3. Import the local journal with original sequence/fingerprints, then compare-and-set append `plan.hosted.attached` as the first hosted event.
4. Atomically write `.emery/hosted.yaml`. From that event onward the stream is journal authority; ordinary local `plan execute` refuses and the file journal is a read-only projection.

Every lifecycle artifact mutation uploads a new content-addressed coordination snapshot, then compare-and-set appends the event that makes its digest authoritative; an unreferenced upload is inert. Resume verifies the hosted event chain and snapshot digest, reconstructs an RFC-87 change projection in the required empty directory, writes a read-only local journal projection, acquires a fenced lease, and continues the drained loop. Attach/retry uses the locally journaled id, so failure before or after remote namespace creation cannot duplicate a hosted change. There is no detach back to a writable local journal; finalize closes the hosted stream and the ordinary archive/delete posture applies.

## Lifecycle sketch

```text
control plane                       node A (project: billing)         node B (project: mobile)
─────────────                       ──────────────────────────         ─────────────────────────
plan approved; entries eligible
  ├─ lease(billing-api) → A
  └─ lease(mobile-shell) → B
                                    materialize(billing-base)          materialize(mobile-base)
                                    …judgment rounds…                  …judgment rounds…
                                    round ends → changes()             round ends → changes()
  ◄─ changeset α published          ─┘                                  │
  ◄─ changeset β published          ────────────────────────────────────┘
trial integration:
  billing-base + α → billing verify
  mobile-base + β  → mobile verify
  → aggregated findings on coordination plane
                                    (repair round if owned finding)    (repair round if owned finding)
serial merge gate: emery slice merge, one entry at a time → per-entry done
```

## Rejected alternatives

- **Shared volume / network filesystem** — reintroduces coupled failure domains, locking semantics, and location dependence; breaks the `local-path` lending model.
- **CRDT-synced live tree** — solves a problem the round-boundary rhythm doesn't have, at the cost of unverifiable intermediate states.
- **Coordination via the value plane** (e.g. control records as blobs) — collapses D1; coordination needs ordering and liveness, not content addressing.
- **A second event store beside the journal** — dual-write drift; the journal projection (D6) is strictly simpler.

## Fixed implementation cut

- The first hosted binding is NATS JetStream: one stream per change for ordered journal events, one KV bucket for compare-and-set lease records and fencing generations, one coordination-artifact Object Store bucket for plan/change/slice snapshots, and a separate value Object Store bucket for revision/changeset bytes. The capability remains transport-neutral; no second backend is required for completion.
- Lease expiry reports suspected loss but never transfers ownership. `lease recover` validates the last journal round, increments the fencing generation atomically, and invalidates every write carrying the old token.
- The journal follow surface is an authenticated ordered event stream resumed by sequence number. The deployment supplies NATS credentials; Emery defines no user directory or auth service.
- Value objects are chunked by the JetStream backend, named by SHA-256, and verified after download before materialization. RFC-86 binary-patch changesets remain the payload format.
- Trial-integration findings are advisory. Patch/base conflicts block only the affected composition and therefore its dependent scheduling; the existing serial merge and verify gates retain lifecycle authority.
- `plan author` records agent-proposed path manifests. The CLI normalizes them and compares ownership only within the same project; unknown or overlapping ownership inserts a `depends-on` edge when order is unambiguous and otherwise rejects the plan. This rule is target-neutral.
- The completion workload is an Omnia/Rust multi-project change, where RFC-89 verification and RFC-90 remote pools are available. Projects with unavailable verify profiles execute serially and emit typed trial-integration unavailability; they do not silently bypass a claimed gate.

## Phased delivery

- **Phase A — Hosted tree and control plane.** Bind RFC-86 materialization to JetStream values, add coordination snapshots, the durable per-change journal stream, fenced lease records, explicit recovery, `plan execute --hosted` attach/resume, and remote re-entry (D3, D6, D11–D13).
- **Phase B — Two-node changeset handoff.** One producer and one dependent exchange values at round boundaries through private trees, proving D1–D5 end to end.
- **Phase C — Remote worker pools.** Place completed RFC-90 pools on remote nodes; each worker receives a private hosted tree and returns a changeset through the same value plane.
- **Phase D — Concurrent plan entries and trial integration.** Add plan-level manifests, deterministic parallel eligibility, composed candidate baselines, the advisory verify gate, and its finding taxonomy (D8–D10). RFC-91 is complete when Phase D passes.

## Acceptance criteria

1. `emery plan execute --hosted` attaches an authored RFC-87 change, prints/stores its id without credentials, cuts journal authority over exactly once, and resumes with `--change-id` on a second node to byte-identical plan/artifact status and no writable local journal copy.
2. Two nodes cannot hold one tree lease. Explicit recovery increments the fencing generation, and stale workers cannot append events, publish values, or release the recovered lease.
3. A revision and changeset produced locally materialize remotely to the same tree digest; downloaded values fail closed on digest mismatch.
4. An RFC-90 Omnia worker pool runs remotely with private trees and returns the same composed result and normalized verify findings as the single-node run.
5. Independent plan entries execute concurrently; same-project dependents compose producer values over their shared base, while cross-project dependencies order scheduling without applying foreign patches. Unknown or overlapping same-project manifests serialize or reject deterministically.
6. Trial integration produces and verifies one candidate tree per project, then streams an aggregated advisory report without moving `slice merge` or per-entry `done` authority.
7. Process loss at every phase boundary resumes from journal sequence, lease generation, and content-addressed values without shared filesystem state.
8. Failure injection before namespace creation, after snapshot upload, after journal import, and after hosted cutover resumes idempotently without duplicate change ids, authoritative events, or forge/tree mutations.
9. `cargo make ci` is green in touched repositories; two-node integration tests cover attach/resume, coordination snapshots, journal order, fencing, value integrity, remote materialization, worker placement, concurrent entries, and trial integration.
