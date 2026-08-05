# RFC-92: Node Sync

> Status: Draft — step 7 of the platform-migration series (scale track) ([platform.md](platform.md))
>
> Owns: the complete multi-node execution binding for one change: transport of RFC-86 facts and RFC-87 snapshot values between nodes (with no authority cutover and no second lifecycle model), fenced claims, remote preparation of RFC-87 private workspaces, remote placement of RFC-91 worker pools, the three sync planes and their separation, round-boundary convergence, concurrent execution of independent plan entries, and the trial-integration gate whose findings measure overall quality.
>
> Depends on completed [RFC-86](rfc-86-change-facts.md) (the fact substrate: per-actor logs, projected status, claims, pinned values — the state model this RFC transports but never changes), [RFC-88](rfc-88-detached-changes.md) (change-scoped state and member bindings), [RFC-87](rfc-87-working-trees.md) (`prepare` / `capture` / `discard`, immutable snapshots, and private workspaces), [RFC-90](rfc-90-verify-profiles.md) (trial-integration verification), and [RFC-91](rfc-91-concurrent-execution.md) (worker pools, ownership, and code-patch composition).
>
> Related: [RFC-89](rfc-89-publication-sets.md) (orthogonal — that RFC binds one change's *publication* across repositories on the forge; this RFC coordinates one change's *execution* across nodes before anything is published), [RM-18](roadmap.md#rm-18-cloud-hosted-execute-loop) (the hosted execute loop is the first deployment that needs this fabric).

## Intent

Let one change execute across several nodes — desktop peers, a hosted fleet, or a mix — with near-realtime coordination and without ever sharing a filesystem. Three planes with different consistency needs are kept separate: **coordination** ([RFC-86](rfc-86-change-facts.md) facts — claims, `plan.execute.started`, per-actor event logs, and the projections over them), **convergence** (immutable project and source snapshot objects moving between private workspaces), and **publication** (branches and PRs on the forge, unchanged and operator-owned).

This RFC adds **only transport and fencing**. RFC-86 already made the state model multi-node-correct — per-actor append-only logs union deterministically, status is projected, work is claimed by fact — so nothing here changes what state *is*, who may author it, or how it is read. A node participates in a change the way a second operator already does (exchange facts, exchange values, claim work); this RFC makes that exchange fast, durable, and fenced. The desktop remains the degenerate deployment with the transports absent.

"Near realtime" is defined honestly: dependents observe a producer's work at round boundaries — when a judgment leg completes and its result snapshot is recorded — not at keystroke granularity. That matches how the spawned-agent backend already works (cold spawn per leg), so the fabric adds coordination without changing the execution model.

Every **code patch** in this RFC is RFC-87's `{ base snapshot, result snapshot, touched paths }` relation. RFC-89's publication set is a separate forge-side record and never enters the value plane.

## Decisions

| # | Decision | Consequence |
| - | -------- | ----------- |
| D1 | **The three planes stay separate.** Coordination state never rides the value plane; code never rides the event stream; publication stays forge-side. | Each plane picks its own transport and consistency model. A dashboard needs only the coordination plane; a worker needs facts for its execution request plus snapshot objects from the value plane. |
| D2 | **Every remote input and result crosses as an RFC-87 snapshot.** The value plane transports project bases and results, source inputs, and the objects they reference; no shared volume, network filesystem, patch blob, persistent source copy, or live directory handle crosses the wire. | Every node prepares its own private disk-backed writable or read-only workspace and gives the agent a real `local-path`. |
| D3 | **Claims gain fencing tokens.** An [RFC-86](rfc-86-change-facts.md) D7 claim transported through the control plane carries a fencing generation; a node prepares a private workspace only for a claimed slice, and a claim is stolen only when `plan execute` reconciles an explicitly confirmed recovery, never by timeout alone. | Cross-node ownership is fenced by the claim. Local workspaces need no second lock because each execution receives a fresh directory; RFC-91 manifests partition worker writes. |
| D4 | **Convergence happens at round boundaries.** A worker records its result after `capture` stores and verifies the snapshot objects; dependents prepare from the new immutable result. There is no sub-round sync. | "Near realtime" is honest: latency equals round length plus transport. Keystroke-level sync is a non-goal (D7), and *publication* remains reserved for the forge boundary. |
| D5 | **The value plane is a transport-neutral capability with one shipped binding.** Nothing in `emery:adapter` or the engine guest names a transport; this RFC ships NATS JetStream Object Store as the first complete backend. | One deployable path is sufficient for completion. Iroh, S3, or another object backend can implement the settled capability later without becoming RFC-92 phases. |
| D6 | **The coordination plane transports RFC-86 facts; it is never a second authority.** The per-change JetStream stream carries the same per-actor events recorded in each node's ordinary change home — a low-latency, durable *carrier*, with each actor still the only author of its own log and every projection deterministic over the received union. Each local change home remains the at-rest record of its actor's facts and the union it has received. | No authority cutover, no dual-write, no reconciliation protocol — the properties RFC-86 bought are exactly the ones that make a carrier sufficient. Dashboards, waiting workers, and status views run the ordinary projection over the streamed union. A disconnected node retains its last received projection and pauses distributed work until the transport reconnects. |
| D7 | **Live multi-writer file sync is out of contract.** No CRDT tree, synced editor buffers, or two agents writing one path concurrently across nodes. | Concurrent work is expressed as private workspaces with partitioned ownership, then composed by [RFC-91](rfc-91-concurrent-execution.md)'s deterministic kernel. |
| D8 | **Independent plan entries build concurrently; layering is per target.** Entries with no `depends-on` path are eligible in parallel. Entries binding the same `plan.yaml.targets` row share its recorded base and use RFC-91's composer for producer code patches. A dependency across targets orders scheduling but never composes one repository's result into another. `emery slice merge` stays the serial writer of baselines and of the merge fact from which per-entry `done` projects. | The existing dependency graph is the concurrency declaration, while the target key is the composition boundary. RFC-92 owns the graph→per-target-layer projection and does not reopen RFC-91's composer. |
| D9 | **Trial integration is one candidate tree per target.** At round boundaries the orchestrator groups outstanding code patches by target and RFC-91 convergence wave, composes each wave into the next candidate snapshot, runs that target's RFC-90 verify profiles, and emits one aggregated finding set on the coordination plane. Cross-target dependencies affect order only; cross-repository CI is outside this gate. | Integration health is meaningful for multi-repository plans: each candidate contains only one target's composed results, and each finding names its target and owning entry. |
| D10 | **Disjointness over smallness.** `plan author` records a per-slice write manifest and checks overlap only among entries binding the same target. Predicted shared paths become a `depends-on` edge or fan-in integration task; ambiguous overlap is rejected. Captured touched paths remain authoritative at runtime. Different targets are structurally disjoint. | Result snapshots become more frequent, not necessarily smaller. Per-slice overhead still sets a cost floor under slice size, and a bad prediction cannot become a silent merge. |
| D11 | **Remote workspaces preserve RFC-87 semantics.** A remote worker resolves the requested snapshot objects, prepares a private disk-backed workspace with `local-path`, runs under a fenced claim, captures its result snapshot at the round boundary, and discards the workspace. | Hosted execution is a backend binding over completed local semantics, not another workspace model. Byte-identical snapshots round-trip locally and remotely. |
| D12 | **Attach configures transport; authority never moves.** Binding an RFC-88 change to the control plane uploads the change home's facts and referenced values, then streams each actor's subsequent events as they are appended locally. Per-actor sequence numbers (RFC-86 D3) make replication idempotent; the projection needs no global order. Detach is symmetric: stop streaming, and the local change home retains the received union. | There is no cutover event, no read-only demotion of local files, and no one-way door. A change can attach from desktop-only operation and return to a complete local copy on detach; both states read the same facts through the same projection. |
| D13 | **`plan execute --hosted` is the attach and resume surface.** In an RFC-88 change directory it configures transports, registers this node's actor identity, and executes; with `--change-id` and an empty `--project-dir` it reconstructs the change tree from the stream and resumes under a fresh actor claim. | RFC-92 is directly operable without waiting for RM-18's background-submit product surface or adding a second lifecycle command family. Resume is deterministic reconstruction, not a recovery protocol. |

## Runtime-discovered cross-slice overlap

If captured results from separate slices unexpectedly touch the same target path, the target convergence gate rejects that wave before composing any result. Other target domains continue.

The conflicting result snapshots remain immutable inputs to recovery. Each producer recaptures a result without the shared path. When the shared edit is a coherent cross-slice unit of intent, Emery proposes a new integration slice that depends on those producers and exclusively owns the path; otherwise the entries serialize. Adding the slice changes the plan digest, so the affected target domain remains paused until the operator reruns `plan execute`, which appends a fresh `plan.execute.started` for the amended plan. The integration slice then starts from the repaired composed snapshot and emits the next target snapshot through the ordinary RFC-91 gate.

## Attach contract

The operator surface is:

```text
# First attach, from the authored RFC-88 change directory
emery plan execute --hosted

# Resume from another node into an empty directory
emery plan execute --hosted --change-id <id> --project-dir <dir>
```

The deployment supplies `NATS_URL` and `NATS_CREDS` (a credentials-file path). Emery prints the generated UUIDv7 change id on attach and stores only the endpoint, id, and last observed per-actor sequences in `.emery/hosted.yaml`; credentials never enter the change home or the fact logs.

Attach is transport configuration, not a cutover:

1. Refuse unless the plan is authored, then append [RFC-86](rfc-86-change-facts.md)'s `plan.execute.started` for the current plan and artifacts.
2. Append the local `plan.hosted.attach-started` fact with the generated id, create the JetStream namespaces idempotently, and upload the change home's artifacts and fact logs, plus referenced snapshot objects to the value bucket.
3. Append `plan.hosted.attached`. From here each actor's new events stream as they are appended locally; incoming remote events append to their authors' log files in the local change home.
4. Atomically write `.emery/hosted.yaml`. Local files remain authoritative facts owned by their authoring actors; the stream replicates, it never demotes.

Resume verifies the received fact union's per-actor sequences and artifact digests, reconstructs the change tree in the required empty directory, registers a distinct actor identity, acquires fenced claims for the entries it takes, and continues the drained loop. Attach retry uses the locally journaled id, so failure before or after remote namespace creation cannot duplicate a hosted change. Detach stops the stream after bringing the local change home current; further distributed work waits for reattachment. `plan archive` closes the stream before applying the ordinary archive/delete posture.

## Lifecycle sketch

```text
control plane                       node A (project: billing)         node B (project: mobile)
─────────────                       ──────────────────────────         ─────────────────────────
execution authorized; entries eligible
  ├─ claim(billing-api) → A
  └─ claim(mobile-shell) → B
                                    materialize(billing-base)          materialize(mobile-base)
                                    …judgment rounds…                  …judgment rounds…
                                    round ends → capture()             round ends → capture()
  ◄─ result snapshot α published    ─┘                                  │
  ◄─ result snapshot β published    ────────────────────────────────────┘
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

- The first hosted binding is NATS JetStream: one stream per change carrying every actor's RFC-86 events (idempotently replicated by actor + per-actor sequence), one KV bucket for compare-and-set claim records and fencing generations, one coordination-artifact Object Store bucket for the change home's artifacts, and a separate value Object Store bucket for snapshot objects. The capability remains transport-neutral, and no second coordination or value backend is required for completion.
- Claim expiry reports suspected loss but never transfers ownership. On re-entry, `plan execute` can confirm recovery, validate the last fact round, increment the fencing generation atomically, and invalidate every write carrying the old token; there is no separate recovery subcommand.
- The fact follow surface is an authenticated ordered event stream resumed by per-actor sequence. The deployment supplies NATS credentials; Emery defines no user directory or auth service.
- Snapshot objects are chunked by the JetStream backend, content-addressed, and verified after download before materialization. RFC-87 defines no separate patch payload.
- Trial-integration findings are advisory. Patch/base conflicts block only the affected composition and therefore its dependent scheduling; the existing serial merge and verify gates retain lifecycle authority.
- `plan author` records agent-proposed path manifests. The CLI normalizes them and compares ownership only within the same target; unknown or overlapping ownership inserts a `depends-on` edge when order is unambiguous and otherwise rejects the plan. This rule is adapter-neutral.
- The completion workload is an Omnia/Rust multi-project change, where RFC-90 verification and RFC-91 remote pools are available. Projects with unavailable verify profiles execute serially and emit typed trial-integration unavailability; they do not silently bypass a claimed gate.

## Phased delivery

- **Phase A — Fact and value transport.** Bind RFC-87 materialization to JetStream values, stream the per-actor fact logs both ways, add fenced claim records, explicit recovery, `plan execute --hosted` attach/resume, and remote re-entry (D3, D6, D11–D13).
- **Phase B — Two-node snapshot handoff.** One producer and one dependent exchange result snapshots at round boundaries through private workspaces, proving D1–D5 end to end.
- **Phase C — Remote worker pools.** Place completed RFC-91 pools on remote nodes; each worker receives a private workspace and returns a result snapshot through the same value plane.
- **Phase D — Concurrent plan entries and trial integration.** Add plan-level manifests, deterministic parallel eligibility, composed candidate baselines, the advisory verify gate, and its finding taxonomy (D8–D10). RFC-92 is complete when Phase D passes.

## Acceptance criteria

1. `emery plan execute --hosted` appends `plan.execute.started` and attaches an authored RFC-88 change idempotently, prints/stores its id without credentials, and resumes with `--change-id` on a second node to a byte-identical projection; both nodes keep appending their own authoritative fact logs throughout, and stopping the stream brings each ordinary change home current without requiring Git metadata.
2. Two nodes cannot own one slice claim. Explicit recovery increments the fencing generation, and stale workers cannot append events, record results, or release the recovered claim.
3. Project snapshots captured locally and source snapshots ingested locally materialize remotely to the same tree digests; downloaded objects fail closed on digest mismatch.
4. An RFC-91 Omnia worker pool runs remotely with private trees and returns the same composed result and normalized verify findings as the single-node run.
5. Independent plan entries execute concurrently; same-target dependents compose producer values over their shared base, while cross-target dependencies order scheduling without applying foreign patches. A runtime-discovered shared path rejects only the affected target wave, retains every result, and proposes an integration slice requiring execution to be invoked again, or deterministic serialization, while unrelated target domains continue.
6. Trial integration produces and verifies one candidate tree per target, then streams an aggregated advisory report without moving `slice merge` or per-entry `done` authority.
7. Process loss at every phase boundary resumes from the per-actor fact sequences, claim generation, and content-addressed values without shared filesystem state.
8. Failure injection before namespace creation, after artifact upload, and mid-stream resumes idempotently without duplicate change ids, duplicated facts, or forge/tree mutations; a node that streams, detaches after synchronization, and re-attaches observes the same projection at each step.
9. `cargo make ci` is green in touched repositories; two-node integration tests cover attach/resume, fact replication, fencing, value integrity, remote materialization, worker placement, concurrent entries, and trial integration.
