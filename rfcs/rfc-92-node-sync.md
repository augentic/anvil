# RFC-92: Node Sync

> Status: Draft — step 7 of the platform-migration series, scale track ([platform.md](platform.md))
>
> Owns: multi-node execution for one change: fact and snapshot transport, fenced claims, remote private workspaces and worker pools, round-boundary convergence, concurrent plan entries, and trial integration.
>
> Depends on completed [RFC-86](rfc-86-change-facts.md), [RFC-88](rfc-88-detached-changes.md), [RFC-87](rfc-87-working-trees.md), [RFC-90](rfc-90-verify-profiles.md), and [RFC-91](rfc-91-concurrent-execution.md).
>
> Related: [RFC-89](rfc-89-publication-sets.md) binds publication across repositories after this RFC's distributed execution; [RM-18](roadmap.md#rm-18-cloud-hosted-execute-loop) is the first hosted deployment.

## Intent

Let one change execute across desktop peers, a hosted fleet, or both without sharing a filesystem.

RFC-86 already makes change state safe to exchange: each actor owns an append-only log, logs form a deterministic union, status is projected, and claims assign work. This RFC adds transport and fencing around that model. It does not move authority, add a second lifecycle, or change how facts are interpreted.

Coordination is near realtime; code convergence is deliberately coarser. A dependent observes a producer's result when a judgment leg ends and its immutable snapshot has been captured and verified. Emery does not synchronize keystrokes or live directories.

The single-node desktop remains the degenerate deployment with these transports absent.

## Flow and terms

1. An operator starts `emery plan execute --hosted` from an authored RFC-88 change home.
2. Emery records the privileged start, attaches the change to its coordination and value transports, and registers the node's actor identity.
3. Nodes claim eligible entries. Each worker resolves immutable inputs into a fresh private workspace.
4. At a round boundary, the worker captures its result and verifies the stored snapshot objects before publishing the result fact.
5. The orchestrator composes outstanding results per target, runs trial integration, and streams an aggregated advisory finding set.
6. `emery slice merge` remains the serial writer of baselines and the merge fact from which per-entry `done` projects.

The three **sync planes** are:

- **coordination** — RFC-86 claims, `plan.execute.started`, per-actor event logs, and deterministic projections over their union;
- **convergence** — immutable project, source, and result snapshot objects exchanged between private workspaces;
- **publication** — branches and pull requests on the forge, unchanged and operator-owned under RFC-89.

A **round boundary** is the point after a judgment leg completes and `capture` records its result. A **fencing generation** is the monotonically increasing token attached to a claim and every write made under that claim. A **code patch** is RFC-87's `{ base snapshot, result snapshot, touched paths }` relation. It is not a patch blob, and RFC-89's publication set never enters the value plane.

## Worked example: attach and resume on two nodes

Suppose node A holds the authored `checkout-v2` change home. It starts hosted execution:

```bash
emery plan execute --hosted
```

The deployment supplies `NATS_URL` and `NATS_CREDS`, where `NATS_CREDS` names a credentials file. Emery appends `plan.execute.started`, generates and prints a UUIDv7 change id such as `0198a40f-…`, attaches the change, and stores only the endpoint, change id, and last observed per-actor sequences in `.emery/hosted.yaml`. Credentials never enter the change home or fact logs.

Node B can resume the same change into an empty directory:

```bash
emery plan execute --hosted \
  --change-id 0198a40f-… \
  --project-dir /tmp/checkout-v2
```

Node B verifies the received per-actor sequences and artifact digests, reconstructs the change tree, registers a distinct actor identity, and claims eligible entries with fresh fencing tokens. Each node continues to append only its own authoritative facts; replicated remote events are stored under their original authors' logs.

If node B disappears while holding `mobile-shell` at generation 18, expiry reports suspected loss but does not transfer the claim. On re-entry, `plan execute` can explicitly confirm recovery, validate the last fact round, and compare-and-set the generation to 19. Any later event, result, or release from the stale generation-18 worker is rejected.

Detach first brings the local change home current and then stops streaming. The local fact union remains readable through the ordinary RFC-86 projection. Reattachment observes the same state; there is no authority cutover or one-way door.

## Worked example: trial integration across targets

Consider three entries:

```yaml
slices:
  - name: add-refund-endpoint
    target: payments-api
  - name: normalize-payment-errors
    target: payments-api
  - name: adopt-refund-ui
    target: mobile
    depends-on: [add-refund-endpoint]
```

The first two entries share the recorded `payments-api` base and have disjoint write manifests, so they may run concurrently. At the round boundary, Emery composes their RFC-91 code patches into one `payments-api` candidate and runs that target's RFC-90 verify profiles.

The `mobile` dependency controls scheduling only. Once `add-refund-endpoint` has produced the required result, `adopt-refund-ui` may run against the `mobile` base. Emery never applies the payments patch to the mobile repository. Trial integration creates a separate `mobile` candidate and runs the mobile verify profiles.

The coordination plane receives one aggregated advisory report containing findings from both candidates. Every finding names its target and owning entry. Cross-repository CI is outside this gate, and the existing serial merge and verify gates retain lifecycle authority.

If the two payments results unexpectedly both touch `src/errors.rs`, the payments convergence wave stops before either result is composed. The immutable results remain available for recovery, and the mobile or any other target domain may continue. The producers can recapture without the shared path; otherwise Emery proposes a fan-in integration slice that depends on both producers and exclusively owns the path, or serializes the entries.

Adding an integration slice changes the plan digest. The affected target stays paused until the operator invokes `plan execute` again, appending a fresh `plan.execute.started` for the amended plan. The integration slice then starts from the repaired composed snapshot and uses the ordinary RFC-91 gate.

## Decisions

### D1 — The three planes stay separate

Coordination state never rides the value plane, code never rides the event stream, and publication stays forge-side.

Each plane therefore uses the consistency model it needs. A dashboard needs only coordination facts. A worker needs the facts relevant to its execution request plus immutable objects from the value plane.

### D2 — Every remote input and result crosses as an RFC-87 snapshot

The value plane transports project bases and results, source inputs, and the objects they reference. No shared volume, network filesystem, patch blob, persistent source copy, or live directory handle crosses the wire.

Every node prepares its own disk-backed writable or read-only private workspace and gives the agent a real `local-path`.

### D3 — Claims gain fencing tokens

An RFC-86 D7 claim transported through the coordination plane carries a fencing generation. A node prepares a private workspace only for a claimed slice.

A timeout can report suspected loss, but it never transfers ownership. A claim is stolen only when `plan execute` reconciles an explicitly confirmed recovery, validates the last fact round, and atomically increments the generation. Every event, result, and release carries the token; writes from an older generation fail closed.

Private workspaces need no second lock because every execution receives a fresh directory. RFC-91 manifests partition writes within worker pools.

### D4 — Convergence happens at round boundaries

A worker records its result only after `capture` has stored and verified the snapshot objects. Dependents prepare from that new immutable result. There is no sub-round synchronization.

Coordination latency is therefore the round length plus transport latency. This matches the existing cold-spawn-per-leg agent backend and leaves publication reserved for the forge boundary.

### D5 — The value plane is transport-neutral with one shipped binding

Nothing in `emery:adapter` or the engine guest names a transport. This RFC ships NATS JetStream Object Store as the first complete value backend.

One deployable path is sufficient for completion. Iroh, S3, or another object backend may implement the settled capability later; no second backend is part of RFC-92.

### D6 — The coordination plane transports facts but never becomes their authority

One per-change JetStream stream carries the same RFC-86 events stored in each node's ordinary change home. It is a low-latency durable carrier, not a second event model.

Each actor remains the only author of its log. A local change home stores that actor's facts and the union received from other actors, and every dashboard, status view, and waiting worker runs the ordinary deterministic projection over that union. Per-actor sequences make replication idempotent; no global order is required.

There is no authority cutover, dual-write protocol, or reconciliation protocol. A disconnected node retains its last received projection and pauses distributed work until transport reconnects.

### D7 — Live multi-writer file synchronization is out of contract

Emery does not provide CRDT trees, synchronized editor buffers, or two agents writing the same path concurrently across nodes.

Concurrent work happens in private workspaces with partitioned ownership. RFC-91's deterministic kernel composes the results.

### D8 — Independent entries run concurrently, with layering per target

Plan entries with no `depends-on` path are eligible in parallel. Entries bound to the same `plan.yaml.targets` row share its recorded base and use RFC-91's composer for producer code patches.

A dependency across targets orders scheduling but never composes one repository's result into another. The target key is the composition boundary, while the existing plan graph remains the concurrency declaration. RFC-92 owns that graph-to-per-target-layer projection and does not reopen RFC-91's composer.

`emery slice merge` remains the serial writer of baselines and the merge fact from which per-entry `done` projects.

### D9 — Trial integration creates one candidate tree per target

At each round boundary, the orchestrator groups outstanding code patches by target and RFC-91 convergence wave. It composes each wave into the target's next candidate snapshot, runs that target's RFC-90 verify profiles, and emits one aggregated finding set on the coordination plane.

Every finding identifies its target and owning entry. Cross-target dependencies affect order only, and cross-repository CI is outside this gate.

The aggregated findings measure overall quality but remain advisory. Patch or base conflicts block only the affected composition and its dependent scheduling; they do not move authority away from the existing serial merge and verify gates.

### D10 — Prefer disjoint ownership over artificially small slices

`plan author` records an agent-proposed write manifest for each slice. The CLI normalizes those paths and compares ownership only among entries bound to the same target. Predicted shared paths become a `depends-on` edge when order is unambiguous or a fan-in integration task; ambiguous overlap rejects the plan. This rule is adapter-neutral.

Captured touched paths remain authoritative at runtime. Different targets are structurally disjoint. Result snapshots may become more frequent without becoming smaller, and per-slice overhead still places a practical floor under slice size.

When runtime results unexpectedly overlap, the affected target wave follows the recovery shown in the trial-integration example. A bad prediction can never become a silent merge.

### D11 — Remote workspaces preserve RFC-87 semantics

A remote worker resolves the requested snapshot objects, prepares a private disk-backed workspace with `local-path`, runs under a fenced claim, captures its result at the round boundary, and discards the workspace.

Remote RFC-91 worker pools use the same sequence for each worker. Hosted execution is a backend binding over the completed local model, not a second workspace model. Byte-identical snapshots round-trip locally and remotely.

### D12 — Attach configures transport; authority never moves

Attaching an RFC-88 change uploads the change home's facts, artifacts, and referenced values, then streams each actor's later events as they are appended locally. Incoming events are replicated into their authors' log files in the local change home.

Per-actor sequences make replication idempotent, and projection needs no global order. Detach is symmetric: bring the local union current, stop streaming, and retain the complete ordinary change home.

There is no cutover event, read-only demotion of local files, or one-way door. Desktop-only and attached operation read the same facts through the same projection.

### D13 — `plan execute --hosted` is the attach and resume surface

In an authored RFC-88 change directory, `emery plan execute --hosted` configures the transports, registers the node's actor identity, and executes.

With `--change-id` and an empty `--project-dir`, the same command reconstructs the change tree from the stream, registers a fresh actor, acquires fenced claims for the entries it takes, and resumes the drained loop. Resume is deterministic reconstruction, not a separate recovery protocol.

This makes RFC-92 directly operable without waiting for RM-18's background-submit surface or adding a second lifecycle command family.

## Implementation requirements

- Implement the first hosted binding with NATS JetStream: one stream per change carrying every actor's RFC-86 events, idempotently replicated by actor and per-actor sequence; one KV bucket holding compare-and-set claim records and fencing generations; one coordination-artifact Object Store bucket holding change-home artifacts; and a separate value Object Store bucket holding snapshot objects.
- Keep coordination and value capabilities transport-neutral. No second backend is required for completion, and no transport name enters `emery:adapter` or the engine guest.
- Accept `NATS_URL` and the `NATS_CREDS` credentials-file path from the deployment. Define no Emery user directory or authentication service. Expose fact follow as an authenticated ordered event stream resumed by per-actor sequence.
- On first attach, refuse an unauthored plan; append `plan.execute.started` for the current plan and artifacts; append local `plan.hosted.attach-started` with a generated UUIDv7 id; create JetStream namespaces idempotently; upload change artifacts, fact logs, and referenced values; append `plan.hosted.attached`; begin bidirectional fact streaming; and atomically write `.emery/hosted.yaml`.
- Store only the endpoint, change id, and last observed per-actor sequences in `.emery/hosted.yaml`. Never store credentials in the change home or fact logs. Retry with the locally journaled id so failure before or after namespace creation cannot duplicate a hosted change.
- On resume, require an empty destination, verify per-actor sequences and artifact digests, reconstruct the change tree, register a distinct actor identity, and continue through fenced claims. Detach only after synchronization; further distributed work waits for reattachment. `plan archive` closes the stream before applying its ordinary archive or delete posture.
- Treat claim expiry only as suspected loss. Recovery stays inside `plan execute`: explicitly confirm it, validate the last fact round, atomically increment the fencing generation, and reject every event, result, or release carrying the old token. Add no recovery subcommand.
- Chunk snapshot objects in the JetStream backend, address them by content, and verify them after download before materialization. Transport project bases and results and source inputs through this path. RFC-87 defines no separate patch payload.
- Place completed RFC-91 worker pools on remote nodes without changing their ownership or composition contracts. Every worker receives a private workspace and returns a result snapshot through the value plane.
- Project the plan graph into deterministic parallel eligibility and per-target convergence layers. Normalize agent-proposed manifests, insert an unambiguous `depends-on` edge for unknown or overlapping ownership, and otherwise reject the plan.
- At round boundaries, compose one candidate per target and convergence wave, run the target's RFC-90 profiles, and stream one normalized aggregated advisory report. Patch or base conflicts block only the affected composition and dependent scheduling.
- Use an Omnia/Rust multi-project change, where RFC-90 verification and RFC-91 remote pools are available, as the completion workload. A project with unavailable verify profiles runs serially and emits typed trial-integration unavailability; it never silently bypasses a claimed gate.

## Acceptance criteria

1. `emery plan execute --hosted` appends `plan.execute.started` and attaches an authored RFC-88 change idempotently, prints and stores its id without credentials, and resumes with `--change-id` on a second node to a byte-identical projection. Both nodes keep appending their own authoritative fact logs, and stopping the stream brings each ordinary change home current without requiring Git metadata.
2. Two nodes cannot own one slice claim. Explicit recovery increments the fencing generation, and stale workers cannot append events, record results, or release the recovered claim.
3. Project snapshots captured locally and source snapshots ingested locally materialize remotely to the same tree digests. Downloaded objects fail closed on digest mismatch.
4. An RFC-91 Omnia worker pool runs remotely with private trees and returns the same composed result and normalized verify findings as the single-node run.
5. Independent plan entries execute concurrently. Same-target dependents compose producer values over their shared base, while cross-target dependencies order scheduling without applying foreign patches. A runtime-discovered shared path rejects only the affected target wave, retains every result, and proposes an integration slice requiring execution to be invoked again, or deterministic serialization, while unrelated target domains continue.
6. Trial integration produces and verifies one candidate tree per target, then streams an aggregated advisory report without moving `slice merge` or per-entry `done` authority.
7. Process loss at every phase boundary resumes from per-actor fact sequences, the claim generation, and content-addressed values without shared filesystem state.
8. Failure injection before namespace creation, after artifact upload, and mid-stream resumes idempotently without duplicate change ids, duplicated facts, or forge or tree mutations. A node that streams, detaches after synchronization, and reattaches observes the same projection at each step.
9. `cargo make ci` is green in touched repositories. Two-node integration tests cover attach and resume, fact replication, fencing, value integrity, remote materialization, worker placement, concurrent entries, and trial integration.

## Rejected alternatives

- **Shared volumes or network filesystems** — couple failure domains, introduce distributed locking semantics, depend on location, and break the `local-path` lending model.
- **CRDT-synchronized live trees** — solve a problem the round-boundary rhythm does not have while making intermediate states difficult to verify.
- **Coordination records in the value plane** — collapse D1. Coordination needs liveness and per-actor ordering, not content addressing.
- **A second event store beside the fact logs** — creates dual-write drift. One authoritative set of per-actor logs with a streaming carrier is sufficient.
- **Hosted journal authority cutover** — making the stream the sole durable event authority and demoting local files to projections creates two lifecycle models, a one-way boundary, and a hosted dependency for reading a local change. RFC-86's per-actor logs remove the need.
