# RFC-92: Node Sync

> Status: Draft — step 7 of the platform-migration series, scale track ([platform.md](platform.md))
>
> Owns: multi-node placement of RFC-91's completed local execution model: fact, planning-artifact, domain-round, and snapshot transport; fenced claims; remote private workspaces and worker pools; attach, resume, and detach. It adds no scheduler, convergence, merge, authority, or lifecycle semantics.
>
> Depends on completed [RFC-86](rfc-86-change-facts.md), [RFC-88](rfc-88-detached-changes.md), [RFC-87](rfc-87-working-trees.md), [RFC-90](rfc-90-build-verification.md), and [RFC-91](rfc-91-concurrent-execution.md).
>
> Related: [RFC-89](rfc-89-publication-sets.md) binds publication across repositories after this RFC's distributed execution; [RM-18](roadmap.md#rm-18-cloud-hosted-execute-loop) is the first hosted deployment.

## Intent

Let one change execute across desktop peers, a hosted fleet, or both without sharing a filesystem.

RFC-86 already makes change state safe to exchange: each actor owns an append-only log, logs form a deterministic union, status is projected, and claims assign work. RFC-88 defines atomic target-wave commit, and RFC-91 defines concurrent leaf eligibility, multi-member waves, and durable bottom-up domain convergence on one node. This RFC adds transport, placement, and fencing around that model. It does not move authority, add a second lifecycle, or change how facts are interpreted.

Coordination is near realtime; code convergence is deliberately coarser. A dependent observes a producer's result when a judgment leg ends and its immutable snapshot has been captured and verified. Emery does not synchronize keystrokes or live directories.

The single-node desktop remains the degenerate deployment with these transports absent.

## Flow and terms

1. An operator starts `emery plan execute --hosted` from an authored RFC-88 change home.
2. Emery records the privileged start, attaches the change to its coordination and value transports, and registers the node's actor identity.
3. RFC-91's scheduler identifies eligible leaf entries; nodes acquire their fenced claims. Each worker resolves immutable inputs into a fresh private workspace.
4. At a round boundary, the worker captures its result and verifies the stored snapshot objects before publishing the result fact.
5. Completed leaf and domain results replicate by digest. Any node with the required facts and values may continue RFC-91's unchanged bottom-up fold.
6. RFC-88's atomic target-wave commit remains the only accepted-CID writer and projects every member leaf `done`.

The three **sync planes** are:

- **coordination** — RFC-86 claims, `plan.execute.started`, per-actor event logs, and deterministic projections over their union;
- **convergence** — immutable project, source, and result snapshot objects exchanged between private workspaces;
- **publication** — branches and pull requests on the forge, unchanged and operator-owned under RFC-89.

A **round boundary** is the point after a judgment leg completes and `capture` records its result. A **fencing generation** is the monotonically increasing token attached to a claim and every write made under that claim. A **code patch** is RFC-87's `{ base snapshot, result snapshot, touched paths }` relation. A **domain result** is RFC-91's immutable record: either one composed candidate CID for a single-target domain or an ordered target→CID/report set for a multi-target coordination domain. It is not a patch blob, and RFC-89's publication set never enters the value plane.

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

RFC-88's decomposition places the first two entries under one `payment-behaviour` domain inside the `payments-api` target domain. They share the recorded base and have disjoint write manifests, so RFC-91 may run them concurrently whether both workers are local or one is remote. At the round boundary, the same RFC-91 kernel composes their patches, writes the domain-round record, dispatches RFC-90's model-assisted `verify`, and folds the result upward.

The `mobile` dependency controls scheduling only. Once `add-refund-endpoint` has produced the required result, `adopt-refund-ui` may run against the `mobile` base. Emery never applies the payments patch to the mobile repository. Trial integration creates a separate `mobile` candidate and dispatches the mobile target's verification phase.

The root coordination domain receives the same aggregated advisory report as a desktop-only run. Every finding names its target, nearest domain, and owning entry. Cross-repository CI is outside this gate, and RFC-88's target-wave commit retains accepted-CID authority.

If the two payments results unexpectedly both touch `src/errors.rs`, RFC-91 stops their nearest common domain before either result is composed and writes the same inert recovery proposal it would locally. The immutable results and proposal replicate; the mobile or any domain outside that branch may continue.

The operator applies that proposal through RFC-88's `plan amend --proposal` surface and starts a new closed-plan execution epoch. Transport does not gain an amendment writer or hidden recovery path.

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

### D8 — Remote placement preserves the local recursive scheduler

RFC-91 remains the sole definition of concurrent leaf eligibility, domain readiness, composition, verification, and multi-member wave selection; RFC-88 remains the definition of target-wave commit. RFC-92 may place any claimed leaf, target-build worker, or ready domain operation on a capable node. Placement uses the digest-matched decomposition revision and never infers a replacement hierarchy from plan order.

Every remote claimed result carries its authorization epoch, fencing generation, and RFC-91 input fence. The receiving node rejects stale generations or mismatched lead, decomposition, model-capability-profile, wave, leaf, dependency-frontier, spec, or base digests before recording or folding it.

Domain convergence itself remains claimless and content-addressed. A remote domain operation carries RFC-91's operation key over its complete inputs and accepted frontier; compare-and-set publication accepts the first byte-valid record for that key and treats an identical duplicate as success. A different byte-valid result loses the compare-and-set and is not authoritative—the model-assisted gate is not falsely treated as deterministic. No synthetic domain claim or lifecycle state is introduced.

### D9 — Domain records, facts, and values travel together

The coordination-artifact plane transports retained lead and decomposition revisions, including their embedded model-capability profiles. It also transports amendment proposals and RFC-91 domain-round records. The coordination stream transports the facts that reference them. The value plane transports every CID reachable from build and domain records. A referenced fact becomes visible to projection only after its artifact and value dependencies are present and digest-verified.

This ordering makes a completed domain gate resumable on any node without recomputation. Detach first synchronizes the same closure; garbage collection treats replicated live records as roots.

### D10 — Recovery proposals do not acquire authority in transport

RFC-88 and RFC-91 may write validated amendment proposals after runtime overlap, refinement boundary escalation, or target-decomposition escalation. RFC-92 replicates them as immutable coordination artifacts but never applies them. Only RFC-88's operator-invoked compare-and-set amendment surface may revise lead, decomposition, and plan authority.

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
- Place RFC-91 eligible leaves, workers, and domain operations remotely without changing readiness or composition. Require authorization epoch, fencing generation, and complete RFC-91 input fences on claimed results; require the operation key and claimless first-valid compare-and-set publication on domain results.
- Replicate retained lead/decomposition revisions with embedded model-capability profiles, amendment proposals, and domain-round records through the coordination-artifact store before exposing their referencing facts; replicate every reachable CID through the value plane.
- Use an Omnia/Rust multi-project change, where RFC-90's `build` / `repair` / `verify` / `review` loop and RFC-91 remote pools are available, as the completion workload. Every remote verification and repair dispatch records the same typed phase report as its local equivalent; distribution never upgrades model-assisted evidence into a deterministic claim.

## Acceptance criteria

1. `emery plan execute --hosted` appends `plan.execute.started` and attaches an authored RFC-88 change idempotently, prints and stores its id without credentials, and resumes with `--change-id` on a second node to a byte-identical projection. Both nodes keep appending their own authoritative fact logs, and stopping the stream brings each ordinary change home current without requiring Git metadata.
2. Two nodes cannot own one slice claim. Explicit recovery increments the fencing generation, and stale workers cannot append events, record results, or release the recovered claim.
3. Project snapshots captured locally and source snapshots ingested locally materialize remotely to the same tree digests. Downloaded objects fail closed on digest mismatch.
4. An RFC-91 Omnia worker pool runs remotely with private trees. Given the same recorded worker patches, local and remote composition return the same candidate; remote verification retains the same typed report and model-assisted assurance contract without promising byte-identical findings from a fresh model call.
5. One desktop-only and one two-node projection over the same accepted facts, planning revisions, model-capability-profile digests, domain records, and values produces the same target-wave CID and leaf statuses. Duplicate remote publication of one operation key is idempotent; when independently evaluated results differ, the first byte-valid compare-and-set winner remains authoritative and the loser cannot alter projection. No RFC-92 code path defines alternative readiness, convergence, or acceptance policy.
6. A domain-round fact arriving before its record or referenced CIDs remains invisible; once all dependencies verify, resume reuses the completed round without rerunning composition or verification. Detach retains the same resumable closure locally.
7. Process loss at every phase boundary resumes from per-actor fact sequences, the claim generation, and content-addressed values without shared filesystem state.
8. Failure injection before namespace creation, after artifact upload, and mid-stream resumes idempotently without duplicate change ids, duplicated facts, or forge or tree mutations. A node that streams, detaches after synchronization, and reattaches observes the same projection at each step.
9. `cargo make ci` is green in touched repositories. Two-node integration tests cover attach and resume, fact replication, fencing, value integrity, remote materialization, worker placement, concurrent entries, and trial integration.

## Rejected alternatives

- **Shared volumes or network filesystems** — couple failure domains, introduce distributed locking semantics, depend on location, and break the `local-path` lending model.
- **CRDT-synchronized live trees** — solve a problem the round-boundary rhythm does not have while making intermediate states difficult to verify.
- **A hosted-only scheduler or convergence policy** — creates different desktop and fleet workflow semantics. RFC-92 distributes RFC-91's completed local model and owns placement only.
- **Coordination records in the value plane** — collapse D1. Coordination needs liveness and per-actor ordering, not content addressing.
- **A second event store beside the fact logs** — creates dual-write drift. One authoritative set of per-actor logs with a streaming carrier is sufficient.
- **Hosted journal authority cutover** — making the stream the sole durable event authority and demoting local files to projections creates two lifecycle models, a one-way boundary, and a hosted dependency for reading a local change. RFC-86's per-actor logs remove the need.
