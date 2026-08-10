# RFC-93: Distributed Execution

> Status: Draft — step 8 of the platform-migration series, scale track ([platform.md](platform.md))
>
> Owns: running one change across multiple nodes without changing its execution semantics. Adds distribution for facts, planning artifacts, domain rounds, and snapshots; slice ownership generations and stale-work rejection; claim-based remote execution and worker pools; and attach, resume, and detach.
>
> Does not own: scheduling policy, result convergence, merge semantics, workflow authority, or lifecycle.
>
> Depends on completed [RFC-86](rfc-86-change-facts.md), [RFC-87](rfc-87-working-trees.md), [RFC-88](rfc-88-detached-changes.md), [RFC-90](rfc-90-build-verification.md), [RFC-91](rfc-91-staged-refinement.md), and [RFC-92](rfc-92-concurrent-execution.md).
>
> Related: [RFC-89](rfc-89-publication-sets.md) binds publication across repositories after this RFC's distributed execution.
>
> Runtime dependency: Omnia bindings for `wasi:documentstore`, native `wasi:keyvalue` atomics, `wasi:blobstore`, `wasi:messaging` wake-up announcements, and durable asynchronous trigger supervision. Where an interface or backend lacks a required guarantee, this RFC requires improving the general Omnia capability rather than adding an Emery-specific transport API.
>
> Foundation already laid: the snapshot values plane speaks `wasi:blobstore` locally today — the engine guest's workspace kernel stores every snapshot object through that import (the omnia-backends filesystem backend under one desktop deployment). Worker placement is therefore a backend/container-namespace swap behind the same import — one line in the host, guests untouched — and object-closure movement can reuse the kernel's `Objects` seam.

## Intent

*Let one change run across peer nodes **without a shared filesystem**.*

Emery can already run a change on a single machine. Each slice is given to a worker along with a private workspace. Progress events are logged, slice results combined, and commited as a group.

This RFC adds the transport and safeguards needed to preserve that same behavior **across sibling nodes**.

Attached engine runtimes publish and follow progress and ownership updates through the coordination plane. Code moves between nodes as immutable, verified snapshots after an operation completes.

Work reaches a node through claiming: the slice owner publishes a durable operation offer, eligible nodes race to claim it, and the winner executes entirely locally. Placement emerges from the first successful claim.

A single desktop — the degenerate case — uses the same execution model through local Omnia capability implementations, requiring no additional configuration: the local node claims every offer it publishes.

## Existing execution contract

This RFC distributes the existing contract:

- Each journal writer appends only to its own event log. Emery combines these logs to calculate status.
- An ownership record identifies which writer owns a slice and which ownership generation is current.
- Every code-writing worker starts with immutable inputs in a fresh private workspace and returns an immutable result. Verification and review receive fresh materializations of the current candidate. Their incidental writes are recorded for inspection and discarded; acceptance uses the candidate snapshot.
- Slices may run concurrently, with results combining upward from slice leaves through the recorded domain hierarchy.
- Emery groups slices for the same target repository into a **target wave** and commits all their results together. If any result cannot be accepted, none are committed.
- Recovery proposals remain inert until an operator invokes an amendment.

## Omnia runtime

Each participating node runs the `emery` binary as an Omnia runtime. The runtime embeds the engine guest, binds host capabilities before the guest starts, and resolves source and target adapter guests **locally**. Backend endpoints and credentials remain in the native host; guests receive only typed WIT capabilities.

This RFC composes existing storage interfaces with Omnia runtime capabilities:

- `**wasi:documentstore**` stores workflow events as immutable documents keyed by writer and sequence and queries each writer's events after a sequence cursor;
- `**wasi:keyvalue` atomics** provide native linearizable compare-exchange and conditional update for slice-ownership records, operation claims, and first-writer-wins publication;
- `**wasi:blobstore**` carries immutable coordination records — including operation offers — and snapshot values under separate logical namespaces;
- `**wasi:messaging**` announces new offers on capability-scoped topics as a pure wake-up; delivery may be lost or duplicated without affecting correctness.

These runtime primitives carry coordination and values. Emery supplies the workflow policy for eligibility, authorization, ownership, stale-work rejection, dependency visibility, convergence, and acceptance. Every bound Omnia interface and backend must provide the guarantees that policy requires.

Claimed operations execute locally. A node that claims an operation prepares the workspace through its own RFC-87 workspace capability and invokes its locally resolved adapter guest against it. Immutable input and result identities cross nodes; workspace IDs, live WIT resources, and filesystem paths remain node-local.

Coordination belongs to the engine runtime. Source and target adapters continue to receive one operation request and one private workspace. Claiming is engine-runtime behavior on the worker node; the claimed execution returns a subordinate result to the slice owner, which records it in its workflow log.

Operator ingress is separate from inter-node execution. The engine exposes the same typed workflow operations through CLI and HTTP. An interactive deployment may invoke `emery plan execute --distributed` and remain attached to the process. A hosted deployment submits the execute operation through `POST /plan/execute`; the native host authenticates the request, allocates or selects its managed change home, and supervises the engine invocation after returning `202 Accepted` with an opaque attachment ID. The host-supervised attachment survives request disconnection. General Omnia invocation-control surfaces provide status, event follow, and graceful detach within the same Emery workflow.

Once the execute operation starts, offers, claims, writer logs, slice-ownership records, and snapshot values flow through the coordination and value capabilities above. HTTP remains the operator control surface.

## Distributed execution

1. The operator invokes the distributed execute operation against an authored change home, either through the attached CLI or authenticated HTTP control surface.
2. Emery records `plan.execute.started`, opens the change's distributed session through the configured Omnia capabilities, and registers a writer ID for the local engine runtime.
3. The RFC-92 execution loop identifies slices whose dependencies and workflow gates are satisfied. Before offering any operation for a slice, an attached Emery runtime must atomically acquire slice execution ownership.
4. For each ready operation, the slice owner durably publishes an **operation offer** — the guest identity to invoke, the content-addressed input tree identity, the access manifest, capability requirements, the authorization epoch, the owner's writer ID, and the current ownership generation — then announces it on a capability-scoped `wasi:messaging` topic. Eligible nodes race to claim the offer through a linearizable compare-exchange; the first successful claim wins.
5. The claiming node fetches the input closure through the value plane, verifies it, prepares a fresh private workspace through its local workspace capability, and invokes its locally resolved adapter guest. When a writing operation finishes, capture freezes the resulting repository tree into an immutable snapshot and verifies every object needed to reconstruct it before the node publishes the result record under the offer's identity.
6. Other nodes follow the writer logs and fetch referenced records or snapshot values as needed. A node may use a result only after every dependency is present and digest-verified.
7. Any attached engine runtime with the required inputs may continue the existing bottom-up convergence. A target-wave commit remains the only operation that advances a target repository's accepted state.

### Transport boundaries

This RFC keeps three concerns separate even when one backend implements more than one capability:

- **Coordination** carries slice-ownership records, operation offers and claims, and durable writer events, plus the immutable planning and domain records those events reference. It uses `wasi:documentstore`, `wasi:keyvalue` atomics, and a coordination namespace in `wasi:blobstore`.
- **Values** are immutable project, source, and result snapshots. Each node's workspace capability moves their content-addressed object closure through a logically separate `wasi:blobstore` namespace.
- **Publication** remains operator-owned on the forge through branches and pull requests.

`wasi:messaging` provides non-authoritative wake-ups that shorten the interval between offer publication and claim. A worker node may equally discover unclaimed offers by scanning the coordination plane. Durable offers and claim arbitration preserve correctness across lost, duplicated, or reordered announcements.

Coordination may report that work exists before its larger snapshot values become locally available. Projection exposes a result after its complete dependency set is present and verified. One host backend may satisfy several capabilities while keeping its resource names behind the guest contract.

The two planes have opposite trust and liveness profiles, and backend selection must respect the asymmetry. Values are self-certifying: a CID names its own bytes, so a snapshot object may arrive from a hosted store, a relay, or a nearby peer and verify on read. Coordination is authority-bearing: slice ownership, operation claims, and per-writer cursors bind to a linearizable, durably hosted point of serialization. This gives the value plane flexible transfer paths while making resume independent of participant availability.

### Key terms

- A **journal writer** is a stable identity with exclusive append authority over one `.emery/events/<writer-id>.jsonl` log and its sequence namespace. In distributed execution, an attached Emery runtime becomes a slice owner under its writer ID; nodes executing claimed operations return subordinate results to that writer.
- A **CID** is the content digest of an immutable snapshot.
- **Slice execution ownership**, shortened below to **slice ownership**, is the exclusive responsibility of one journal writer to progress one slice. RFC-86 records its acquisition as `slice.claimed`; distributed execution adds an ownership generation to reject stale work. Workflow authority continues to come from the recorded execution epoch and artifacts.
- An **ownership generation** is a number that increases whenever slice ownership is recovered. Every event, result, and release carries the current generation. After it increases, Emery rejects anything carrying an older generation.
- An **operation offer** is the durable, domain-neutral record by which a slice owner exposes one ready operation for execution: operation ID, guest identity, input tree CID, access manifest, capability requirements, authorization epoch, writer ID, and ownership generation.
- A **claim** is the lease-bound record, acquired by linearizable compare-exchange, that binds one node to one offer for the lease's duration and determines workspace placement.
- A **code patch** is the relation `{ base snapshot, result snapshot, touched paths }`.
- A **domain result** records either one composed CID for a single-target domain or an ordered target-to-CID/report set for a multi-target domain.

## Example: attach, resume, and recover

Suppose node A's runtime is rooted at the authored `checkout-v2` change home. An interactive operator can attach through the CLI:

```bash
emery plan execute --distributed
```

The equivalent hosted ingress is asynchronous:

```http
POST /plan/execute HTTP/1.1
Content-Type: application/json

{"distributed":true}
```

The host returns `202 Accepted` with an opaque attachment ID. The Omnia host runtime connects its configured capabilities before the engine guest starts. Emery records the privileged start, generates and returns a UUIDv7 change ID such as `0198a40f-…`, publishes the change state and referenced values, and begins following writer events. The local attachment sidecar at `.emery/distributed.yaml` stores only the change ID and last observed sequence for each writer. It is not part of the detached fact tree or product configuration. Backend configuration, ingress credentials, and managed host paths never enter the guest, change home, or event logs.

Node B can resume the same change through its hosted control surface:

```http
POST /plan/execute HTTP/1.1
Content-Type: application/json

{"distributed":true,"change-id":"0198a40f-…"}
```

Node B's host allocates an empty managed change home; an HTTP caller never supplies a path meaningful only on the caller's filesystem. The equivalent interactive CLI remains `emery plan execute --distributed --change-id 0198a40f-… --project-dir ./checkout-v2`. Node B verifies the received sequences and artifact digests, reconstructs the change tree, registers its own writer ID, and acquires execution ownership of any eligible unowned slices with fresh ownership generations. It also begins subscribing to the offer topics its capabilities match, so operations offered by node A may execute on node B. Each writer continues to append only to its own event log. Replicated events retain their original writer IDs.

Suppose node B disappears while owning `mobile-shell` at generation 18. An expiry notification reports suspected loss but does not transfer ownership. During a later `plan execute`, the operator may explicitly confirm recovery after Emery verifies that the slice's immutable base or most recently published result can be fully reconstructed and passes digest verification. Emery then atomically advances the ownership record to generation 19. Any event, result, or release carrying generation 18 is rejected before it can affect projection. Any operation claim node B held simply expires; the owning runtime re-offers the operation, and a late result under the expired claim is rejected by its stale claim identity.

A graceful stop first brings the local change home current and then detaches from the distributed session. Reattaching reconstructs the same projected state. `plan archive` closes the attachment before performing its ordinary archive or delete behavior.

## Decisions

### D1 — Omnia capabilities preserve the transport boundaries

Events use `wasi:documentstore`. Slice-ownership records, operation claims, and first-writer-wins records use native `wasi:keyvalue` atomics. Immutable planning records and operation offers use a coordination namespace in `wasi:blobstore`. Product and source snapshots move through each node's workspace value path over a separate `wasi:blobstore` namespace. `wasi:messaging` carries only capability-scoped offer announcements. Publication remains on the forge.

This lets each concern use the consistency model it needs and prevents large code objects from blocking coordination updates.

The engine guest imports capability contracts. Backend identities, endpoints, subjects, bucket names, credentials, and mappings remain native host policy. Source and target adapter worlds receive their operation-specific interfaces. A future blob backend may fetch snapshot values peer-to-peer without changing the contract: verify-on-read makes the transfer path irrelevant to correctness. Coordination capabilities bind to a linearizable, durably hosted backend.

### D2 — Remote code results cross nodes only as verified snapshots

A claiming node performs each code-writing operation entirely in its private workspace. After the operation finishes, the workspace freezes the resulting repository tree into an immutable snapshot, stores and verifies every object needed to reconstruct it, and derives the touched paths by comparing it with the operation's base snapshot. This is the capability's `capture` operation, and the immutable snapshot is the code representation published for another node.

Only after this process succeeds may the result be published for another node to use.

Dependants create fresh private workspaces from that immutable result. Domain gates publish immutable domain-round records instead of code snapshots. Result-availability latency is therefore the operation's runtime plus snapshot storage, verification, and transfer latency.

### D3 — Ownership generations reject stale work

An attached Emery runtime offers operations for a slice only after acquiring slice ownership through a linearizable compare-exchange. The ownership record stores that runtime's writer ID, and its ownership generation accompanies every offer, event, result, and release produced under that ownership.

An expired ownership record marks its owner as possibly unavailable. Operator-authorized recovery transfers responsibility to another runtime.

To recover, the slice’s latest code state is reconstructed—either its immutable base or its latest published result—and digest verified. Slice ownership is then incremented in one atomic operation. Events, results, and releases carrying an older generation are rejected and cannot affect workflow state.

An operation claim is subordinate to slice ownership and carries the offer's generation; recovery invalidates outstanding claims along with everything else minted under the old generation. Claim fencing protects each fresh private workspace, while task grants partition path ownership within a worker pool.

### D4 — Claims preserve execution semantics

RFC-92's execution rules define slice eligibility, domain readiness, composition, verification, and target-wave membership. Claims select where already-eligible work runs while preserving the recorded hierarchy and acceptance policy.

The slice owner controls what is offered and remains the sole acceptor of results. A claiming node receives a validated operation, task grant, and immutable inputs through the offer. Every result produced under that ownership carries its authorization epoch, writer ID, ownership generation, and complete input identity. Emery rejects a result if its slice identity or its lead-catalog, decomposition, model-capability-profile, wave, dependency-frontier, spec, or base digest does not match the current operation.

Domain convergence checks are keyed by their immutable child results and current accepted target state. Once the child results are ready, any attached runtime may perform the identified check.

Two runtimes may finish the same check concurrently, and model-assisted verification may produce different results. Emery therefore records the first structurally valid result atomically. Repeating that result is harmless; a later different result is ignored. `wasi:blobstore` holds the result, while a `wasi:keyvalue` compare-exchange records which result digest won. This selects one authoritative result while allowing model-assisted evaluations to vary.

### D5 — Writer facts retain workflow authority

Each journal writer remains the only author of its workflow events. Events are persisted (`wasi:documentstore`) keyed by writer ID and sequence. Other nodes query each writer's events in sequence order and resume after their last received sequence when reconnecting.

Per-writer sequence numbers make delivery idempotent and let each follower resume independently. A node stores received events under their original writer IDs and runs the existing projection over the combined logs. Delivery may be at least once; gaps remain pending until missing sequences arrive.

An attached change's coordination state is durable independently of every participating node, so resume proceeds independently of participant availability. A disconnected node retains its last received state and pauses distributed work until its capabilities reconnect.

### D6 — Referenced records and values arrive before their facts become visible

Immutable workflow records—planning revisions, model profiles, amendment proposals, and domain-round records—are stored centrally in a **coordination container** in `wasi:blobstore`.

Project, source, and result snapshot objects are stored in a separate `wasi:blobstore` container. Each node's workspace implementation reads those objects when creating a private workspace and writes them when freezing or capturing a workspace.

An event that references one of those objects remains invisible to projection until the complete dependency set is present and digest-verified. This ordering lets another node resume a completed domain round without recomputing it. Garbage collection treats replicated live records as roots.

### D7 — Placement is claim-based: eligible nodes pull work

The slice owner publishes a durable operation offer on the coordination plane; nodes able to satisfy it claim it; the first successful claim wins. This pull model makes placement the emergent result of claiming.

An offer is domain-neutral at the transport: it carries an operation ID, the guest identity to invoke, the content-addressed input tree identity, the access manifest, capability requirements, the authorization epoch, the slice owner's writer ID, and the current ownership generation. Emery decides which operation is ready and translates its task grants into the access manifest.

The offer record is authoritative and lives in the coordination plane. `wasi:messaging` announces it on a capability-scoped topic — for example, one topic per guest identity and platform requirement — so only suitable nodes wake. Messaging is strictly a wake-up: delivery may be lost or duplicated without affecting correctness, because a worker node may also discover unclaimed offers by scanning the coordination plane, and every claim is arbitrated by a linearizable `wasi:keyvalue` compare-exchange. The claim record carries the claiming node's identity, a liveness lease, and the offer's ownership generation. A node self-assesses eligibility before claiming; a misconfigured node that claims work it cannot complete degrades progress through lease expiry and re-offer, never correctness.

The claiming node then executes entirely locally. It fetches the input closure through the value plane, verifies it, prepares a fresh private workspace through its own RFC-87 workspace capability, and invokes its locally resolved adapter guest against it. `freeze`, `prepare`, `capture`, `compose`, `discard`, cancellation, workspace IDs, live WIT resources, and local paths remain node-local. After the operation finishes, capture stores and verifies every result object (D2), and the node publishes the result record — carrying the operation ID, epoch, writer ID, ownership generation, and complete input identity — for the slice owner to accept under D4's rejection rules.

A claim is disposable, exactly as its workspace is. If the claiming node or its lease is lost, the claim expires as suspected loss; the slice owner re-offers the operation under the same fencing rules that govern slice ownership, and a late result from the expired claim is rejected by its stale claim identity. A completed snapshot remains available by digest. Explicit cancellation is a coordination-plane record the claiming node observes; lease expiry bounds the window in which an unobserved cancellation can consume resources. Duplicate execution after a lost claim is safe: results are immutable, digest-named, and accepted at most once by the slice owner.

The initial placement policy is first-claim-wins with capability topic scoping. Workers may later add claim back-off heuristics while preserving this contract.

Worker pools use the same ownership, composition, verification, and reporting sequence whether their workers claimed locally or remotely. Each worker has isolated writable-tree, workspace, MCP, and prompt state and returns a subordinate result to the slice owner. The host scopes claims to their lease and generation and rejects unknown or stale claim identities. Byte-identical snapshots round-trip between claiming nodes.

### D8 — Plan amendments remain operator-authorized

Runtime overlap, refinement-boundary escalation, and target decomposition may produce validated amendment proposals. Distribution replicates those proposals as inert records; the operator-invoked compare-and-set amendment API applies revisions to lead, decomposition, or plan authority.

### D9 — The execute operation attaches and resumes through CLI or HTTP

The execute operation is transport-neutral. In an authored change directory, `emery plan execute --distributed` invokes it directly and remains attached. In a hosted deployment, authenticated `POST /plan/execute` invokes the same operation against a host-managed change home and returns `202 Accepted` with opaque change and attachment identities.

Supplying a change ID reconstructs the change tree, registers a fresh writer ID, acquires ownership of eligible slices, and resumes the ordinary execution loop. The CLI names an empty host-local `--project-dir`; HTTP deployments allocate that directory server-side. Resume uses deterministic reconstruction through the same execute operation.

The CLI process lifetime may remain the attachment lifetime for an interactive deployment. A hosted attachment is a durable host-supervised invocation: it survives request disconnection and is addressed by its attachment ID for status, event follow, and graceful detach. A graceful stop synchronizes before detaching. The retained change home remains readable through the ordinary operations, and a later invocation may reattach it.

### D10 — Omnia supplies the required primitives

Safety-critical behavior requires native guarantees: create-only event persistence, key-value compare-exchange, create-if-absent publication, and lease-backed claims.

The required improvements land in the existing Omnia capability bindings and backend conformance:

- create-only document insertion plus stable writer-and-sequence queries after a cursor;
- native `wasi:keyvalue` compare-exchange, conditional update, and lease-liveness observation for ownership records and operation claims;
- topic-scoped `wasi:messaging` announcement conformance — deliberately weak: at-most-once delivery and duplicate delivery are both acceptable, because announcements are wake-ups over durable coordination state;
- durable asynchronous trigger supervision with authenticated submit, opaque invocation identity, status, event follow, cancellation, and disconnect-independent execution.

Emery supplies the workflow semantics layered over those primitives. Omnia supplies reusable capability contracts, local test implementations, and production backends. Adapters run on the node that claimed the work through the existing local dispatch path. Exact backend configuration remains deployment documentation.

Omnia additions remain domain-neutral: their vocabulary is limited to content identities, access manifests, guest identities, capability requirements, claim leases, and invocation IDs. Emery owns the workflow-specific mapping to changes, slices, tasks, writers, waves, and lifecycle states.

## Example: concurrent work across targets

Consider three slices:

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

The recorded decomposition places the two `payments-api` slices under one `payment-behaviour` domain. They share a recorded base but own disjoint paths, so their operations may be claimed concurrently by the same node or different nodes. Their results combine at their nearest recorded frontier domain and pass the same verification gate as a desktop-only run.

The `mobile` dependency affects scheduling only. Once the required `add-refund-endpoint` result is accepted, `adopt-refund-ui` runs against the recorded `mobile` base — offered on the mobile capability topic and claimed by whichever suitable node responds first. Each target produces and verifies its own candidate from its recorded base.

If both payments results unexpectedly touch `src/errors.rs`, their nearest shared domain stops before composition. Emery replicates the immutable results and an inert amendment proposal. Unaffected domains may continue, but only the operator may apply the proposal and start a new closed-plan execution epoch.

## Evaluation

Distributed evaluation must separate judgment time, slice-ownership wait, offer-to-claim latency, object transfer, and result-publication latency. Local and remote runs use the same source and input set, model configuration, time budget, and blind acceptance set.

The completion workload is an Omnia/Rust multi-project change using the engine-owned `build` / `repair` / `verify` / `review` loop and remote worker pools. Every remote verification and repair dispatch records the same typed phase report as its local equivalent. Model-assisted evidence retains its existing variability.

Evaluation treats throughput as one metric alongside the accepted result, blind grade, and separately reported claim and transfer costs.

## Implementation requirements

### Omnia capabilities

- Use `wasi:documentstore` for workflow events. Store one immutable document per `(writer, sequence)` using create-only insertion. On an existing ID, accept byte-identical content idempotently and reject different content. Query one writer at a time after its last received sequence in ascending sequence order. Production backends must retain acknowledged inserts and preserve stable ordered queries across reconnects; local test backends must expose the same contract. Cassandra may be a candidate backend for this.
- Use native `wasi:keyvalue` atomics for linearizable compare-exchange and conditional update. A read followed by an unconditional write does not qualify. Ownership and claim liveness may use native expiry or heartbeat values plus observation, but expiry reports only suspected loss and never transfers ownership. Backend conformance tests must race slice-ownership acquisition, recovery, release, first-result publication, claim acquisition, claim-lease lapse against late result publication, and heartbeat lapse against recovery confirmation.
- Use `wasi:blobstore` for immutable coordination records — including operation offers — and snapshot values under separate logical namespaces. Emery names immutable objects by digest, verifies existing and fetched bytes, and never relies on backend overwrite behavior for authority. The value namespace admits peer-to-peer backends — verified-streaming transfer between nodes (for example, iroh-blobs' BLAKE3 verified streaming) suits large snapshot closures — provided the backend's native addressing stays behind the capability: Emery verifies fetched bytes against its own recorded digests regardless of what the transfer layer verified, so a backend whose transfer hash differs from Emery's digest maps between them internally.
- Use `wasi:messaging` only to announce offers on capability-scoped topics. The contract is deliberately weak: at-most-once delivery, duplicate delivery, and reordering are all acceptable. No claim, result, event, or recovery decision may depend on an announcement arriving; a worker that misses an announcement must be able to find the same offer by scanning the coordination plane. Backends need prove nothing beyond topic-scoped delivery when connected.
- Add general durable HTTP-trigger supervision for long-running guest operations. Submission authenticates at the native host, returns `202 Accepted` with an opaque invocation ID, and lets the invocation continue after client disconnect. Status, event follow, graceful cancellation, and result retrieval use that ID. Emery maps the invocation to an attachment, while coordination and value transport continue through their dedicated capabilities.
- Keep backend service discovery, endpoints, credentials, resource names, transfer encoding, and chunking in the native Omnia backend.
- Refuse a distributed session when its bound `wasi:documentstore`, `wasi:keyvalue`, or `wasi:blobstore` backend cannot prove the required contract. Development defaults that are lossy, racy, or process-local do not qualify for those three; `wasi:messaging` is exempt because it is wake-up-only. As of this draft, no shipped backend combination qualifies: document-store backends have not proved durable ordered replay, the default and NATS key-value compare-exchange paths are read-then-write, and blob-store writes are unconditional replacement. These are backend and conformance gaps, not reasons to add another storage interface.

### Attach, resume, and detach

- On first attach, refuse an unauthored plan and append `plan.execute.started` for the current plan and artifacts.
- Accept first attach and resume through the same typed execute input over CLI or HTTP. For HTTP, allocate the change home server-side, return `202 Accepted` with change and attachment IDs, and keep execution independent of the request connection.
- Append local `plan.distributed.attach-started` with a generated UUIDv7 change ID, provision the distributed change idempotently through the capabilities, publish change records and referenced values, append `plan.distributed.attached`, begin following writer events, and atomically write `.emery/distributed.yaml`.
- Store only the change ID and last observed per-writer sequences in `.emery/distributed.yaml`. Do not treat it as part of the detached fact tree or product configuration. Never store backend endpoints or credentials in the change home or event logs.
- Retry first attach with the locally journaled ID so failure before or after capability provisioning cannot create a duplicate distributed change.
- On resume, require an empty destination, verify per-writer sequences and artifact digests, reconstruct the change tree, register a distinct writer ID, and continue through slice ownership.
- Synchronize before a graceful detach. Pause distributed work while detached. Close the attachment before `plan archive` applies its ordinary archive or delete behavior.

### Offers, claims, and recovery

- Treat ownership expiry only as suspected loss. Keep recovery inside `plan execute`: require explicit confirmation, verify that the slice's immutable base or most recently published result can be fully reconstructed and passes digest verification, atomically increment the ownership generation, fail conditional operations carrying the old generation, and keep stale facts invisible.
- Keep slice execution ownership with the attached engine runtime. Claimed extract, build, repair, verify, review, and domain executions return subordinate results to that owner.
- Publish every operation offer durably before announcing it, and make the offer domain-neutral on the wire: operation ID, guest identity, input tree CID, access manifest, capability requirements, epoch, writer ID, and ownership generation.
- Arbitrate every claim by compare-exchange with a liveness lease and the offer's generation. On lease expiry, re-offer under the same fencing rules; reject a late result by its stale claim identity. Record explicit cancellation on the coordination plane for the claiming node to observe.
- Require the claiming node to execute locally: fetch and verify the input closure, prepare a fresh node-local private workspace, invoke the locally resolved adapter, and return a verified code patch and typed report through the workspace capability before publishing the result.
- Require the authorization epoch, ownership generation, claim identity, and complete input identity on every result produced under slice ownership.
- Require the validated-input and accepted-frontier digest as the operation key. Domain publication is owner-independent and atomically accepts the first byte-valid record matching that key.
- Publish planning revisions, embedded model-capability profiles, amendment proposals, and domain-round records before exposing their referencing events. Make every reachable CID available through the value plane.

## Acceptance criteria

1. CLI and authenticated HTTP ingress invoke the same typed distributed execute operation and record the same `plan.execute.started` authorization. HTTP returns `202 Accepted` with change and attachment IDs and continues after client disconnect.
2. First attach creates one distributed change idempotently and stores its ID without backend endpoints, ingress credentials, or managed host paths.
3. A second node resumes that change into a server-allocated empty directory and reconstructs a byte-identical projection. An HTTP caller supplies no client-local project path.
4. Both nodes continue to author only their own logs. Graceful detach by CLI stop or attachment control leaves an ordinary, current change home that requires no Git metadata to read.
5. Two nodes cannot own the same slice. Confirmed recovery advances its ownership generation; stale conditional writes fail, and stale events or results cannot affect projection or release the recovered ownership.
6. Project and source snapshots materialize on a claiming node to the same tree digests. Digest mismatches fail closed.
7. A remote worker pool claims offers and executes them in node-local private workspaces under the same ownership and composition rules as a local pool. Given the same recorded patches, both produce the same composed candidate.
8. Remote verification retains the same typed, model-assisted report contract as local verification without promising byte-identical findings from a fresh model call.
9. Given the same accepted facts, planning revisions, model-capability profiles, domain records, and values, desktop-only and two-node execution produce the same target-wave CID and slice statuses.
10. Publishing the same domain-operation result twice is idempotent. If independently evaluated results differ, the first byte-valid record matching the validated-input and accepted-frontier digest remains authoritative and the loser cannot alter projection.
11. An event arriving before its referenced records or CIDs remains invisible. Once its dependencies verify, resume reuses the completed domain round without recomposition or reverification.
12. Process loss at every phase boundary resumes from per-writer sequences, the ownership generation, and content-addressed values without shared filesystem state.
13. Failure before capability provisioning, after value publication, or while following events resumes without duplicate change IDs, duplicated facts, or forge or product-tree mutations.
14. General Omnia conformance tests prove create-only event insertion, conflicting-duplicate rejection, stable per-writer ordered queries across reconnects, native key-value compare-exchange under races, expiry without ownership transfer, and disconnect-independent HTTP invocation control. Claim tests prove that two nodes cannot hold one claim, an expired claim's late result cannot affect projection or release anything, a re-offered operation completes on another node from the same inputs to the same digests, offers announce only on matching capability topics, a lost announcement does not strand an offer, and a workspace ID never leaves the node that minted it.
15. `cargo make ci` is green in every touched repository. Two-node Emery integration tests cover CLI/HTTP ingress equivalence, asynchronous attachment control, attach, resume, event replication, stale-work rejection, value integrity, remote materialization, claim contention, lease expiry and re-offer, concurrent slices, and cross-target convergence.
16. Local and two-node live fixtures with the same source and input set, model configuration, time budget, and blind acceptance set report judgment, slice-ownership-wait, offer-to-claim, object-transfer, and result-publication latency separately. Blind grading and runtime metrics remain outside workflow authority.

## Prior art

- **[Bazel Remote Execution API](https://github.com/bazelbuild/remote-apis)** — the same coordination/value split proven at datacenter scale: a content-addressable store for values and an action cache keyed by the digest of a fully specified action. D4's domain publication without slice ownership — the operation key as the digest of validated inputs and accepted frontier, first byte-valid record wins, idempotent duplicates — is that pattern.
- **Pull-based CI runners (GitHub Actions, Buildkite agents)** — fleets of self-registered workers poll a queue and self-select by capability labels; the coordinator keeps no node inventory and makes no placement decision. D7's capability-scoped offers and first-claim-wins are that shape with linearizable claim arbitration and content-addressed inputs added.
- **[wasmCloud](https://wasmcloud.com/blog/wasmcloud-v2-is-here/) v1 → v2** — v1 routed every component import implicitly over a wRPC/NATS lattice; v2 reversed to in-process by default with explicit, deliberate distribution, because implicit distribution tied invocation semantics to transport failure modes. This RFC goes one further: no guest invocation crosses nodes at all — work moves to the guest, not the call to the work.
- **[iroh](https://www.iroh.computer/)** — dial-by-key peer-to-peer QUIC with content-addressed, BLAKE3-verified blob streaming. Relevant strictly on the value plane per the transport-boundary asymmetry; it offers no linearizable primitive, so it is not a coordination candidate.

## Rejected alternatives

- **Push placement through a control plane** — a node registry, capability inventory, placement scheduler, and remote guest dispatch would centralize a decision that claiming makes locally and require workspace-to-node routing. Pull-based claims make workspace and invocation collocation structural.
- **Messaging as claim or delivery authority** — broker delivery carries loss, duplication, and ordering behavior. Durable offers plus compare-exchange claims provide authority; messaging remains a wake-up.
- **HTTP for coordination and value transport** — this would conflate operator ingress with document-backed writer events, offers, claims, and value transfer. HTTP starts and controls an attachment; Omnia capabilities carry distributed execution.
- **Peer-to-peer coordination state** — gossip, CRDT replicas, or peer pubsub would require a consensus layer to provide linearizable ownership, claims, and first-writer-wins publication, while participant-hosted state would weaken durable resume. Peer transfer remains available on the self-certifying value plane.
- **Shared writable trees** — shared volumes, network filesystems, or CRDT-synchronized trees couple failure domains and require distributed write coordination. Immutable snapshots preserve node-local workspaces and verifiable results.

