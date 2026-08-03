# RFC-92: Node Sync

> Status: Draft — step 7 of the platform-migration series (scale track) ([platform.md](platform.md))
>
> Owns: the complete multi-node execution binding for one change: transport of RFC-86 facts and RFC-87 values between nodes (with no authority cutover and no second lifecycle model), fenced claims, hosted materialization of RFC-87 trees, remote placement of RFC-91 worker pools, the three sync planes and their separation, round-boundary convergence, concurrent execution of independent plan entries, and the trial-integration gate whose findings measure overall quality.
>
> Depends on completed [RFC-86](rfc-86-change-facts.md) (the fact substrate: per-actor logs, projected status, claims, pinned values — the state model this RFC transports but never changes), [RFC-88](rfc-88-detached-changes.md) (change-scoped state and member bindings), [RFC-87](rfc-87-working-trees.md) (local `materialize` / `changes()` semantics and value formats), [RFC-90](rfc-90-verify-profiles.md) (trial-integration verification), and [RFC-91](rfc-91-concurrent-execution.md) (worker pools, ownership, local per-worker trees, and changeset composition).
>
> Related: [RFC-89](rfc-89-publication-sets.md) (orthogonal — that RFC binds one change's *publication* across repositories on the forge; this RFC coordinates one change's *execution* across nodes before anything is published), [RM-18](roadmap.md#rm-18-cloud-hosted-execute-loop) (the hosted execute loop is the first deployment that needs this fabric).

## Intent

Let one change execute across several nodes — desktop peers, a hosted fleet, or a mix — with near-realtime coordination and without ever sharing a filesystem. Three planes with different consistency needs are kept separate: **coordination** ([RFC-86](rfc-86-change-facts.md) facts — claims, approvals, per-actor event logs, and the projections over them), **convergence** (the code itself — `revision` / `changeset` values moving between private trees), and **publication** (branches and PRs on the forge, unchanged and operator-owned).

This RFC adds **only transport and fencing**. RFC-86 already made the state model multi-node-correct — per-actor append-only logs union deterministically, status is projected, work is claimed by fact — so nothing here changes what state *is*, who may author it, or how it is read. A node participates in a change the way a second operator already does (exchange facts, exchange values, claim work); this RFC makes that exchange fast, durable, and fenced. The desktop remains the degenerate deployment with the transports absent.

"Near realtime" is defined honestly: dependents observe a producer's work at round boundaries — when a judgment leg completes and its changeset is extracted — not at keystroke granularity. That matches how the spawned-agent backend already works (cold spawn per leg), so the fabric adds coordination without changing the execution model.

Every `changeset` in this RFC is RFC-87's tree-delta value. RFC-89's publication set is a separate forge-side record and never enters the value plane.

## Decisions

| # | Decision | Consequence |
| - | -------- | ----------- |
| D1 | **The three planes stay separate.** Coordination state never rides the value plane; code never rides the event stream; publication stays forge-side. | Each plane picks its own transport and consistency model. A dashboard needs only the coordination plane; a worker needs only the value plane plus its own lease. |
| D2 | **Values-only between trees.** The only things that cross nodes carrying code are `revision` and `changeset` ([RFC-87](rfc-87-working-trees.md)); no shared volume, no network filesystem, no live descriptor over the wire. | Every node's tree is private, disk-backed, and lendable to the spawned-agent backend as a real `local-path` — the cursor backend is unchanged. Shared volumes are rejected below. |
| D3 | **Claims gain fencing tokens; one exclusive lease per working tree.** An [RFC-86](rfc-86-change-facts.md) D7 claim transported through the control plane carries a fencing generation; a node materializes only under a claimed slice and a local RFC-87 lease; a claim is stolen only through explicit recovery, never by timeout alone. | No two writers ever hold the same slice or tree. [RFC-91](rfc-91-concurrent-execution.md) D3's write-ownership manifests partition *within* a tree; the RFC-87 lease partitions *between* trees on one machine; the fenced claim partitions *between* nodes. One ownership ladder, three rungs, no new noun. |
| D4 | **Convergence happens at round boundaries.** A worker publishes its `changeset` when its round ends (judgment leg complete, `changes()` extracted); dependents re-materialize with the new layer. There is no sub-round sync. | "Near realtime" is honest: latency equals round length plus transport. Keystroke-level sync is a non-goal (D7). |
| D5 | **The value plane is a transport-neutral capability with one shipped binding.** Nothing in `emery:adapter` or the engine guest names a transport; this RFC ships NATS JetStream Object Store as the first complete backend. | One deployable path is sufficient for completion. Iroh, S3, or another object backend can implement the settled capability later without becoming RFC-92 phases. |
| D6 | **The coordination plane transports RFC-86 facts; it is never a second authority.** The per-change JetStream stream carries the same per-actor events the change repository holds — a low-latency, durable *carrier*, with each actor still the only author of its own log and every projection deterministic over the received union. The change repository remains the at-rest record; git remains a valid (slower) transport. | No authority cutover, no dual-write, no reconciliation protocol — the properties RFC-86 bought are exactly the ones that make a carrier sufficient. Dashboards, waiting workers, and status views run the ordinary projection over the streamed union. A node that falls off the stream degrades to git-paced sync instead of losing lifecycle access. |
| D7 | **Live multi-writer file sync is out of contract.** No CRDT tree, no synced editor buffers, no two agents writing one path concurrently across nodes. | Concurrent work on one change is expressed as partitioned ownership within a tree or separate trees composed by [RFC-91](rfc-91-concurrent-execution.md)'s deterministic changeset composer — both verifiable. |
| D8 | **Independent plan entries build concurrently; layering is per project.** Entries with no `depends-on` path are eligible in parallel. Entries targeting the same `plan.yaml.projects` row share its approved base and use RFC-91's composer for producer changesets. A dependency across projects orders scheduling but never applies one repository's patch to another. `emery slice merge` stays the serial writer of baselines and of the merge fact from which per-entry `done` projects. | The existing dependency graph is the concurrency declaration, while the project key is the composition boundary. RFC-92 owns the graph→per-project-layer projection and does not reopen RFC-91's composer. |
| D9 | **Trial integration is one candidate tree per project.** At round boundaries the orchestrator groups outstanding changesets by project, composes each group in `depends-on` order, runs that project's RFC-90 verify profiles, and emits one aggregated finding set on the coordination plane. Cross-project dependencies affect order only; cross-repository CI is outside this gate. | Integration health is meaningful for multi-repository plans: no impossible `base + patches-from-other-repos` tree, and each finding names its project and owning entry. |
| D10 | **Disjointness over smallness.** `plan author` records a per-slice write manifest and checks overlap only among entries targeting the same project. Unknown or overlapping same-project ownership becomes a `depends-on` edge or rejection; different projects are structurally disjoint. | Changesets become more frequent, not necessarily smaller. Per-slice overhead still sets a cost floor under slice size. |
| D11 | **Hosted trees preserve RFC-87 semantics.** A remote worker resolves a revision and any composed changesets from the value plane, materializes a private disk-backed tree with `local-path`, runs under a fenced claim plus the local RFC-87 lease, publishes `changes()` at the round boundary, and releases the tree. | Hosted execution is a backend binding over completed local semantics, not another tree model. Byte-identical values round-trip locally and remotely. |
| D12 | **Attach configures transport; authority never moves.** Binding an RFC-88 change to the control plane uploads the change repository's facts and referenced values, then streams each actor's subsequent events as they are appended locally. Per-actor sequence numbers (RFC-86 D3) make replication idempotent; the projection needs no global order. Detach is symmetric: stop streaming, and the change repository is already complete. | There is no cutover event, no read-only demotion of local files, and no one-way door. A change moves between desktop-only, git-synced, and streamed operation freely; every mode reads the same facts through the same projection. |
| D13 | **`plan execute --hosted` is the attach and resume surface.** In an RFC-88 change directory it configures transports, registers this node's actor identity, and executes; with `--change-id` and an empty `--project-dir` it reconstructs the change tree from the stream (or a git remote) and resumes under a fresh actor claim. | RFC-92 is directly operable without waiting for RM-18's background-submit product surface or adding a second lifecycle command family. Resume is a clone, not a recovery protocol. |

## Attach contract

The operator surface is:

```text
# First attach, from the authored RFC-88 change directory
emery plan execute --hosted

# Resume from another node into an empty directory
emery plan execute --hosted --change-id <id> --project-dir <dir>
```

The deployment supplies `NATS_URL` and `NATS_CREDS` (a credentials-file path). Emery prints the generated UUIDv7 change id on attach and stores only the endpoint, id, and last observed per-actor sequences in `.emery/hosted.yaml`; credentials never enter the change repository or the fact logs.

Attach is transport configuration, not a cutover:

1. Refuse unless the plan is authored and a covering [RFC-86](rfc-86-change-facts.md) approval fact exists.
2. Append the local `plan.hosted.attach-started` fact with the generated id, create the JetStream namespaces idempotently, and upload the change repository's committed artifacts and fact logs, plus referenced revision/changeset values to the value bucket.
3. Append `plan.hosted.attached`. From here each actor's new events stream as they are appended locally; incoming remote events append to their authors' log files in the change repository.
4. Atomically write `.emery/hosted.yaml`. Local files remain authoritative facts owned by their authoring actors; the stream replicates, it never demotes.

Resume verifies the received fact union's per-actor sequences and artifact digests, reconstructs the change tree in the required empty directory, registers a distinct actor identity, acquires fenced claims for the entries it takes, and continues the drained loop. Attach retry uses the locally journaled id, so failure before or after remote namespace creation cannot duplicate a hosted change. Detach is stopping the stream: the change repository is complete at every moment, and git-paced sync remains available. Finalize closes the stream and the ordinary archive/delete posture applies.

## Lifecycle sketch

```text
control plane                       node A (project: billing)         node B (project: mobile)
─────────────                       ──────────────────────────         ─────────────────────────
plan approved; entries eligible
  ├─ claim(billing-api) → A
  └─ claim(mobile-shell) → B
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
- **Coordination via the value plane** (e.g. control records as blobs) — collapses D1; coordination needs liveness and per-actor ordering, not content addressing.
- **A second event store beside the fact logs** — dual-write drift; one set of per-actor logs with a streaming carrier (D6) is strictly simpler.
- **Journal authority cutover** (this RFC's pre-RFC-86 shape: the hosted stream becomes the single durable event authority and local files demote to read-only projections) — creates two lifecycle models with a one-way door between them, a reconciliation protocol at the boundary, and a hosted dependency for reading your own change; RFC-86's per-actor logs make the entire problem disappear.

## Fixed implementation cut

- The first hosted binding is NATS JetStream: one stream per change carrying every actor's RFC-86 events (idempotently replicated by actor + per-actor sequence), one KV bucket for compare-and-set claim records and fencing generations, one coordination-artifact Object Store bucket for the change repository's committed artifacts, and a separate value Object Store bucket for revision/changeset bytes. The capability remains transport-neutral; git-paced sync of the change repository is the ever-present fallback, and no second streaming backend is required for completion.
- Claim expiry reports suspected loss but never transfers ownership. `claim recover` validates the last fact round, increments the fencing generation atomically, and invalidates every write carrying the old token.
- The fact follow surface is an authenticated ordered event stream resumed by per-actor sequence. The deployment supplies NATS credentials; Emery defines no user directory or auth service.
- Value objects are chunked by the JetStream backend, named by SHA-256, and verified after download before materialization. RFC-87 binary-patch changesets remain the payload format.
- Trial-integration findings are advisory. Patch/base conflicts block only the affected composition and therefore its dependent scheduling; the existing serial merge and verify gates retain lifecycle authority.
- `plan author` records agent-proposed path manifests. The CLI normalizes them and compares ownership only within the same project; unknown or overlapping ownership inserts a `depends-on` edge when order is unambiguous and otherwise rejects the plan. This rule is target-neutral.
- The completion workload is an Omnia/Rust multi-project change, where RFC-90 verification and RFC-91 remote pools are available. Projects with unavailable verify profiles execute serially and emit typed trial-integration unavailability; they do not silently bypass a claimed gate.

## Phased delivery

- **Phase A — Fact and value transport.** Bind RFC-87 materialization to JetStream values, stream the per-actor fact logs both ways, add fenced claim records, explicit recovery, `plan execute --hosted` attach/resume, and remote re-entry (D3, D6, D11–D13).
- **Phase B — Two-node changeset handoff.** One producer and one dependent exchange values at round boundaries through private trees, proving D1–D5 end to end.
- **Phase C — Remote worker pools.** Place completed RFC-91 pools on remote nodes; each worker receives a private hosted tree and returns a changeset through the same value plane.
- **Phase D — Concurrent plan entries and trial integration.** Add plan-level manifests, deterministic parallel eligibility, composed candidate baselines, the advisory verify gate, and its finding taxonomy (D8–D10). RFC-92 is complete when Phase D passes.

## Acceptance criteria

1. `emery plan execute --hosted` attaches an approved RFC-88 change idempotently, prints/stores its id without credentials, and resumes with `--change-id` on a second node to a byte-identical projection; both nodes keep appending their own authoritative fact logs throughout, and stopping the stream leaves each change repository complete.
2. Two nodes cannot hold one slice claim. Explicit recovery increments the fencing generation, and stale workers cannot append events, publish values, or release the recovered claim.
3. A revision and changeset produced locally materialize remotely to the same tree digest; downloaded values fail closed on digest mismatch.
4. An RFC-91 Omnia worker pool runs remotely with private trees and returns the same composed result and normalized verify findings as the single-node run.
5. Independent plan entries execute concurrently; same-project dependents compose producer values over their shared base, while cross-project dependencies order scheduling without applying foreign patches. Unknown or overlapping same-project manifests serialize or reject deterministically.
6. Trial integration produces and verifies one candidate tree per project, then streams an aggregated advisory report without moving `slice merge` or per-entry `done` authority.
7. Process loss at every phase boundary resumes from the per-actor fact sequences, claim generation, and content-addressed values without shared filesystem state.
8. Failure injection before namespace creation, after artifact upload, and mid-stream resumes idempotently without duplicate change ids, duplicated facts, or forge/tree mutations; a node that streams, detaches to git-paced sync, and re-attaches observes the same projection at each step.
9. `cargo make ci` is green in touched repositories; two-node integration tests cover attach/resume, fact replication, fencing, value integrity, remote materialization, worker placement, concurrent entries, and trial integration.
