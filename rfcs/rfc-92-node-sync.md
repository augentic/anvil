# RFC-92: Node Sync

> Status: Draft — step 7 of the platform-migration series, scale track ([platform.md](platform.md))
>
> Owns: running one change across multiple nodes without changing its execution semantics. Adds distribution for facts, planning artifacts, domain rounds, and snapshots; slice ownership generations and stale-work rejection; multi-node private workspaces and worker pools; and attach, resume, and detach.
>
> Does not own: scheduling policy, result convergence, merge semantics, workflow authority, or lifecycle.
>
> Depends on completed [RFC-86](rfc-86-change-facts.md), [RFC-87](rfc-87-working-trees.md), [RFC-88](rfc-88-detached-changes.md), [RFC-90](rfc-90-build-verification.md), and [RFC-91](rfc-91-concurrent-execution.md).
>
> Related: [RFC-89](rfc-89-publication-sets.md) binds publication across repositories after this RFC's distributed execution.
>
> Runtime dependency: general Omnia capabilities for durable cursor-based logs, linearizable atomic state, backend-neutral blob storage, remote guest dispatch, multi-node private workspaces, and durable asynchronous trigger supervision. Where Omnia lacks one of these general primitives, this RFC requires improving Omnia rather than adding an Emery-specific transport API.

## Intent

*Let one change run across peer nodes, a hosted fleet, or both **without sharing a filesystem**.*

Emery can already run a change on a single machine. Each slice is given to a worker along with a private workspace. Progress events are logged, slice results combined, and commited as a group.

This RFC adds the transport and safeguards needed to preserve that behavior **across nodes**.

Nodes continuously share progress and ownership updates. They exchange code only as immutable, verified snapshots after an operation completes. Emery will not synchronize keystrokes, editor buffers, or live directories.

A single desktop — the degenerate case — uses the same execution model through local Omnia capability implementations and requires no distributed-runtime configuration.

## Existing execution contract

This RFC distributes the existing contract:

- Each journal writer appends only to its own event log. Emery combines these logs to calculate status.
- An ownership record identifies which writer owns a slice and which ownership generation is current.
- Every code-writing worker starts with immutable inputs in a fresh private workspace and returns an immutable result. Verification and review receive fresh materializations of the current candidate. Their incidental writes are recorded for inspection and discarded; they do not become part of the accepted result.
- Slices may run concurrently, with results combining upward from slice leaves through the recorded domain hierarchy.
- Emery groups slices for the same target repository into a **target wave** and commits all their results together. If any result cannot be accepted, none are committed.
- Recovery proposals never amend a plan automatically. Only operator-invoked amendments change plan authority.

## Omnia runtime model

Each participating node runs the native `emery` executable as an Omnia host runtime. The runtime embeds the engine guest, binds host capabilities before the guest starts, and resolves source and target adapter guests locally or remotely. Backend endpoints and credentials remain in the native host; guests receive only typed WIT capabilities.

This RFC composes general Omnia capabilities:

- a **durable log** appends and follows events from independent per-writer sequence cursors;
- **atomic state** provides linearizable compare-exchange, conditional update, and expiry notification for slice-ownership records and first-writer-wins publication;
- **blob storage** carries immutable coordination records and snapshot values under separate logical namespaces;
- **multi-node workspaces** freeze, prepare, capture, compose, and discard private workspaces on the node selected for an operation;
- **remote guest dispatch** routes an adapter invocation to the node holding its workspace.

These are runtime primitives, not workflow policy. Emery still decides eligibility, authorization, ownership, stale-work rejection, dependency visibility, convergence, and acceptance. If an existing Omnia interface is lossy, racy, or local-only, its general contract and backends must be strengthened before Emery relies on it.

Source and target adapters import no coordination capabilities and cannot acquire slice ownership. They continue to receive one operation request and one private workspace. An attached engine runtime registers as a journal writer; a remote worker invocation is subordinate to the slice owner and does not author a second workflow log.

Operator ingress is separate from inter-node execution. The engine exposes the same typed workflow operations through CLI and HTTP. An interactive deployment may invoke `emery plan execute --distributed` and remain attached to the process. A hosted deployment submits the execute operation through `POST /plan/execute`; the native host authenticates the request, allocates or selects its managed change home, and supervises the engine invocation after returning `202 Accepted` with an opaque attachment ID. Request disconnection does not stop the attachment. Status, event follow, and graceful detach address that attachment through general Omnia invocation-control surfaces rather than a second Emery workflow.

The HTTP trigger does not carry worker calls, writer logs, slice-ownership records, or snapshot values. Once the execute operation starts, those flows still use the capabilities below and remote adapter calls still use wRPC. HTTP is an operator control surface, not the node-sync protocol.

## Distributed execution

1. The operator invokes the distributed execute operation against an authored change home, either through the attached CLI or authenticated HTTP control surface.
2. Emery records `plan.execute.started`, opens the change's distributed session through the configured Omnia capabilities, and registers a writer ID for the local engine runtime.
3. The existing scheduler identifies eligible slices. Before preparing a private workspace for a slice, an attached Emery runtime must atomically acquire slice execution ownership.
4. The slice owner dispatches each ready operation through Omnia. The host places a fresh private workspace on a capable node and routes the adapter invocation to that node. Every subordinate worker result carries the slice owner's writer ID, ownership generation, and complete input identity.
5. When a writing operation finishes, the workspace capability freezes the resulting repository tree into an immutable snapshot and verifies every object needed to reconstruct it before the slice owner publishes the result record.
6. Other nodes follow the writer logs and fetch referenced records or snapshot values as needed. A node may use a result only after every dependency is present and digest-verified.
7. Any attached engine runtime with the required inputs may continue the existing bottom-up convergence. A target-wave commit remains the only operation that advances a target repository's accepted state.

### Transport boundaries

This RFC keeps three concerns separate even when one backend implements more than one capability:

- **Coordination** carries slice-ownership records and durable writer events, plus the immutable planning and domain records those events reference. It uses the Omnia durable-log, atomic-state, and blob-storage capabilities.
- **Values** are immutable project, source, and result snapshots. The multi-node workspace capability moves their content-addressed object closure through a logically separate blob-storage namespace.
- **Publication** remains on the forge through branches and pull requests. It is unaffected and operator-owned.

Coordination may report that work exists before its larger snapshot values become locally available, but Emery does not expose a result to projection until its complete dependency set is present and verified. Logical separation does not require separate infrastructure: one host backend may satisfy several capabilities without making its resource names part of the guest contract.

The two planes have opposite trust and liveness profiles, and backend selection must respect the asymmetry. Values are self-certifying: a CID names its own bytes, so a snapshot object may arrive from a hosted store, a relay, or a nearby peer without weakening correctness — verification happens on read either way. Coordination is authority-bearing: slice ownership and per-writer cursors require a linearizable, always-on point of serialization, and the durability contract in D5 — resume requires no other node online — cannot be met by state that lives only on participating peers. Peer-to-peer transfer is therefore a legitimate value-plane backend option and never a coordination-plane substitute.

### Key terms

- A **journal writer** is a stable identity with exclusive append authority over one `.emery/events/<writer-id>.jsonl` log and its sequence namespace. In distributed execution, an attached Emery runtime becomes a slice owner under its writer ID; subordinate remote workers are not journal writers.
- A **CID** is the content digest of an immutable snapshot.
- **Slice execution ownership**, shortened below to **slice ownership**, is the exclusive responsibility of one journal writer to progress one slice. RFC-86 records its acquisition as `slice.claimed`; distributed execution adds an ownership generation to reject stale work. Ownership never grants workflow authority.
- An **ownership generation** is a number that increases whenever slice ownership is recovered. Every event, result, and release carries the current generation. After it increases, Emery rejects anything carrying an older generation.
- A **workspace placement** is an Omnia runtime decision that collocates a private workspace and the guest invocation using it. It does not change Emery's scheduler or ownership.
- A **code patch** is the relation `{ base snapshot, result snapshot, touched paths }`; it is not a separate patch blob.
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

Node B's host allocates an empty managed change home; an HTTP caller never supplies a path meaningful only on the caller's filesystem. The equivalent interactive CLI remains `emery plan execute --distributed --change-id 0198a40f-… --project-dir ./checkout-v2`. Node B verifies the received sequences and artifact digests, reconstructs the change tree, registers its own writer ID, and acquires execution ownership of any eligible unowned slices with fresh ownership generations. Each writer continues to append only to its own event log. Replicated events retain their original writer IDs.

Suppose node B disappears while owning `mobile-shell` at generation 18. An expiry notification reports suspected loss but does not transfer ownership. During a later `plan execute`, the operator may explicitly confirm recovery after Emery verifies that the slice's immutable base or most recently published result can be fully reconstructed and passes digest verification. Emery then atomically advances the ownership record to generation 19. Any event, result, or release carrying generation 18 is rejected before it can affect projection.

A graceful stop first brings the local change home current and then detaches from the distributed session. Reattaching reconstructs the same projected state. `plan archive` closes the attachment before performing its ordinary archive or delete behavior.

## Decisions

### D1 — Omnia capabilities preserve the transport boundaries

Events use the durable-log capability. Slice-ownership and first-writer-wins records use linearizable atomic state. Immutable planning records use a coordination blob namespace. Product and source snapshots use the multi-node workspace value path over a separate blob namespace. Publication remains on the forge.

This lets each concern use the consistency model it needs and prevents large code objects from blocking coordination updates.

The engine guest imports capability contracts, never backend identities, endpoints, subjects, bucket names, or credentials. Source and target adapter worlds import none of the distribution capabilities. A deployment profile may bind several contracts to one service, but that mapping remains native host policy. A future blob backend may fetch snapshot values peer-to-peer without changing the contract: verify-on-read makes the transfer path irrelevant to correctness. Per the transport-boundary asymmetry above, that peer-to-peer option exists only on the value plane; the coordination capabilities always bind to a linearizable, durably hosted backend.

### D2 — Remote code results cross nodes only as verified snapshots

A remote worker performs each code-writing operation entirely in its private workspace. No shared volume, network filesystem, patch blob, persistent source copy, live directory handle, or intermediate edit crosses the wire. After the operation finishes, the workspace freezes the resulting repository tree into an immutable snapshot, stores and verifies every object needed to reconstruct it, and derives the touched paths by comparing it with the operation's base snapshot. This is the capability's `capture` operation.

Only after this process succeeds may the slice owner publish the result for another node to use.

Dependants create fresh private workspaces from that immutable result. Domain gates publish immutable domain-round records instead of code snapshots. Result-availability latency is therefore the operation's runtime plus snapshot storage, verification, and transfer latency.

### D3 — Ownership generations reject stale work

An attached Emery runtime requests a private workspace only after acquiring slice ownership through a linearizable compare-exchange. The ownership record stores that runtime's writer ID, and its ownership generation accompanies every event, result, and release produced under that ownership.

If an ownership record expires, its owner is treated as possibly unavailable; ownership does not automatically move to another runtime. An operator must explicitly authorize recovery. 

To recover, the slice’s latest code state is reconstructed—either its immutable base or its latest published result—and verifies its digest. It then updates ownership and increments the ownership generation in one atomic operation. Events, results, and releases carrying an older generation are rejected and cannot affect workflow state.

Fresh private workspaces need no second lock. Within a worker pool, task grants continue to partition path ownership.

### D4 — Placement cannot change execution semantics

The existing scheduler remains the only definition of slice eligibility, domain readiness, composition, verification, and target-wave membership. Omnia may place eligible work on any capable node, but placement cannot infer a different hierarchy or acceptance policy.

The slice owner controls that slice's remote task calls. A worker receives only the validated operation, task grant, immutable inputs, and workspace placement. Every result produced under that ownership carries its authorization epoch, writer ID, ownership generation, and complete input identity. Emery rejects a result if its slice identity or its lead-catalog, decomposition, model-capability-profile, wave, dependency-frontier, spec, or base digest does not match the current operation.

Domain operations have no execution owner and remain content-addressed. Their operation key is the digest of the validated inputs and accepted frontier. Linearizable create-if-absent publication accepts the first byte-valid record that matches that key; an identical duplicate succeeds idempotently, while a different later record cannot replace the winner. Blob replacement alone cannot implement this rule. The atomic-state capability publishes the winning record digest without pretending that a model-assisted result is deterministic.

### D5 — Transport carries facts; it does not become their authority

Each journal writer remains the only appender to its event log. The durable-log capability carries those same events, not a second event model or lifecycle authority.

Per-writer sequence numbers make delivery idempotent and let each follower resume independently. A node stores received events under their original writer IDs and runs the existing projection over the combined logs. Delivery may be at least once; gaps remain pending until missing sequences arrive. No authority cutover, dual-write protocol, or separate reconciliation model is introduced.

An attached change's coordination state is durable independently of every participating node; resume requires no other node online. A disconnected node retains its last received state and pauses distributed work until its capabilities reconnect.

### D6 — Referenced records and values arrive before their facts become visible

Planning revisions with their embedded model-capability profiles, amendment proposals, and domain-round records travel through the coordination blob namespace. Snapshot objects reachable from build and domain records travel through the multi-node workspace value path.

An event that references one of those objects remains invisible to projection until the complete dependency set is present and digest-verified. This ordering lets another node resume a completed domain round without recomputing it. Garbage collection treats replicated live records as roots.

### D7 — Omnia multi-node workspaces preserve local behavior

The general Omnia workspace capability freezes trees into immutable snapshots and accepts a snapshot identity, access manifest, and placement requirements for preparation. It creates a fresh disk-backed workspace on the selected node, returns an opaque handle to the caller, routes the guest invocation to that placement, and supplies the worker-local `local-path` only on that node. Capture, composition, and discard execute where the workspace lives; remote paths never enter Emery artifacts.

Worker pools use the same ownership, composition, verification, and reporting sequence whether their workers run locally or remotely. Workers never share a writable tree, live handle, MCP state, or prompt state. Byte-identical snapshots round-trip between placements.

Emery provides no CRDT trees, synchronized editor buffers, or simultaneous multi-writer access to one path.

### D8 — Distribution cannot amend the plan

Runtime overlap, refinement-boundary escalation, and target decomposition may produce validated amendment proposals. RFC-92 replicates those proposals but never applies them.

Only the operator-invoked compare-and-set amendment surface may revise lead, decomposition, or plan authority. Distribution adds no hidden recovery or amendment writer.

### D9 — The execute operation attaches and resumes through CLI or HTTP

The execute operation is transport-neutral. In an authored change directory, `emery plan execute --distributed` invokes it directly and remains attached. In a hosted deployment, authenticated `POST /plan/execute` invokes the same operation against a host-managed change home and returns `202 Accepted` with opaque change and attachment identities.

Supplying a change ID reconstructs the change tree, registers a fresh writer ID, acquires ownership of eligible slices, and resumes the ordinary execution loop. The CLI names an empty host-local `--project-dir`; HTTP deployments allocate that directory and never interpret a client-local path. Resume is deterministic reconstruction, not a second recovery protocol.

The CLI process lifetime may remain the attachment lifetime for an interactive deployment. A hosted attachment is a durable host-supervised invocation: it survives request disconnection and is addressed by its attachment ID for status, event follow, and graceful detach. A graceful stop synchronizes before detaching. The retained change home remains readable through the ordinary operations, and a later invocation may reattach it.

### D10 — Missing primitives improve Omnia generally

RFC-92 does not assemble safety-critical behavior from interfaces that lack the required guarantees. A lossy publish API is not a durable log, read-then-write state is not compare-exchange, blob replacement is not create-if-absent, and in-process guest routing is not remote placement.

The required improvements land as general Omnia capabilities and backend conformance:

- durable append and cursor-based follow with per-writer ordering;
- native linearizable compare-exchange, conditional update, and expiry notification;
- remote guest resolution and invocation;
- multi-node private workspaces with opaque handles, placement-local paths, freeze, capture, composition, discard, and cancellation.
- durable asynchronous trigger supervision with authenticated submit, opaque invocation identity, status, event follow, cancellation, and disconnect-independent execution.

Emery supplies the workflow semantics layered over those primitives. Omnia supplies reusable capability contracts, local test implementations, and production backends. Exact backend configuration remains deployment documentation rather than part of this RFC.

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

The recorded decomposition places the two `payments-api` slices under one `payment-behaviour` domain. They share a recorded base but own disjoint paths, so they may run concurrently on the same node or different nodes. Their results combine at their nearest recorded frontier domain and pass the same verification gate as a desktop-only run.

The `mobile` dependency affects scheduling only. Once the required `add-refund-endpoint` result is accepted, `adopt-refund-ui` runs against the recorded `mobile` base. Emery never applies the payments patch to the mobile repository; each target produces and verifies its own candidate.

If both payments results unexpectedly touch `src/errors.rs`, their nearest shared domain stops before composition. Emery replicates the immutable results and an inert amendment proposal. Unaffected domains may continue, but only the operator may apply the proposal and start a new closed-plan execution epoch.

## Evaluation

Distributed evaluation must separate judgment time, slice-ownership wait, workspace placement, object transfer, and result-publication latency. Local and remote runs use the same source and input set, model configuration, time budget, and blind acceptance set.

The completion workload is an Omnia/Rust multi-project change using the engine-owned `build` / `repair` / `verify` / `review` loop and remote worker pools. Every remote verification and repair dispatch records the same typed phase report as its local equivalent. Distribution does not make model-assisted evidence deterministic.

Throughput alone is not success. Evaluation compares the accepted result and blind grade while reporting placement costs separately.

## Implementation requirements

### Omnia capabilities

- Add or strengthen a general durable-log capability with idempotent append by `(writer, sequence)` and follow from independent per-writer cursors. Production backends must preserve per-writer order across reconnects; local test backends must expose the same contract.
- Add native linearizable compare-exchange and conditional update to Omnia atomic state. Expiry reports loss of liveness but grants no ownership. Expiry is lease-shaped: a backend without a native per-key lease implements it as heartbeat records plus watch. Backend conformance tests must race slice-ownership acquisition, recovery, release, create-if-absent publication, and heartbeat lapse against recovery confirmation.
- Use Omnia blob storage for immutable coordination records and snapshot values under separate logical namespaces. Emery names immutable objects by digest, verifies existing and fetched bytes, and never relies on backend overwrite behavior for authority. The value namespace admits peer-to-peer backends — verified-streaming transfer between nodes (for example, iroh-blobs' BLAKE3 verified streaming) suits large snapshot closures — provided the backend's native addressing stays behind the capability: Emery verifies fetched bytes against its own recorded digests regardless of what the transfer layer verified, so a backend whose transfer hash differs from Emery's digest maps between them internally.
- Add general multi-node workspace placement and remote guest dispatch. `freeze`, `prepare`, `capture`, `compose`, `discard`, and cancellation execute on the workspace node; the invoked guest receives a placement-local path while the caller retains only an opaque handle. The dispatch carrier is owned by the Omnia cluster-transport plan (`[rfcs/wrpc-cluster.md](https://github.com/augentic/omnia/blob/main/rfcs/wrpc-cluster.md)` in `augentic/omnia`): wRPC on every leg behind the existing `LinkTransport` seam, with `Target::Remote` resolution making a guest's location a configuration decision. Workspace placement collocates with that dispatch rather than adding a second routing mechanism.
- Add general durable HTTP-trigger supervision for long-running guest operations. Submission authenticates at the native host, returns `202 Accepted` with an opaque invocation ID, and lets the invocation continue after client disconnect. Status, event follow, graceful cancellation, and result retrieval use that ID. Emery maps the invocation to an attachment but adds no second execution lifecycle or HTTP-based node-sync protocol.
- Keep backend service discovery, endpoints, credentials, resource names, transfer encoding, and chunking in the native Omnia backend. Define no Emery user directory or authentication service.
- Refuse a distributed session when a bound backend cannot prove the durable-log, atomic-state, durable-blob, remote-dispatch, or workspace contract. Development defaults that are lossy, racy, or process-local do not qualify. As of this draft, no shipped backend qualifies: the default and NATS keyvalue CAS paths are read-compare-set, the messaging backends are at-most-once pub/sub, and blobstore writes are unconditional replace — the durable-log and atomic-state requirements are new capability work, not configuration.

### Attach, resume, and detach

- On first attach, refuse an unauthored plan and append `plan.execute.started` for the current plan and artifacts.
- Accept first attach and resume through the same typed execute input over CLI or HTTP. For HTTP, allocate the change home server-side, return `202 Accepted` with change and attachment IDs, and keep execution independent of the request connection.
- Append local `plan.distributed.attach-started` with a generated UUIDv7 change ID, provision the distributed change idempotently through the capabilities, publish change records and referenced values, append `plan.distributed.attached`, begin following writer events, and atomically write `.emery/distributed.yaml`.
- Store only the change ID and last observed per-writer sequences in `.emery/distributed.yaml`. Do not treat it as part of the detached fact tree or product configuration. Never store backend endpoints or credentials in the change home or event logs.
- Retry first attach with the locally journaled ID so failure before or after capability provisioning cannot create a duplicate distributed change.
- On resume, require an empty destination, verify per-writer sequences and artifact digests, reconstruct the change tree, register a distinct writer ID, and continue through slice ownership.
- Synchronize before a graceful detach. Pause distributed work while detached. Close the attachment before `plan archive` applies its ordinary archive or delete behavior.

### Remote execution and recovery

- Treat ownership expiry only as suspected loss. Keep recovery inside `plan execute`: require explicit confirmation, verify that the slice's immutable base or most recently published result can be fully reconstructed and passes digest verification, atomically increment the ownership generation, fail conditional operations carrying the old generation, and keep stale facts invisible.
- Keep slice execution ownership with the attached engine runtime. Remote extract, build, repair, verify, review, and domain calls are subordinate Omnia dispatches under that ownership rather than independent journal writers.
- Give every remote writing worker a placement-local private workspace and require it to return a verified code patch and typed report through the multi-node workspace capability.
- Place eligible slices, source extraction, build or repair workers, verification and review workers, and ready domain operations remotely without changing readiness, ownership, composition, verification, or reporting.
- Require the authorization epoch, ownership generation, and complete input identity on every result produced under slice ownership.
- Require the validated-input and accepted-frontier digest as the operation key. Domain publication requires no slice owner and atomically accepts the first byte-valid record matching that key.
- Publish planning revisions, embedded model-capability profiles, amendment proposals, and domain-round records before exposing their referencing events. Make every reachable CID available through the multi-node workspace value path.

## Acceptance criteria

1. CLI and authenticated HTTP ingress invoke the same typed distributed execute operation and record the same `plan.execute.started` authorization. HTTP returns `202 Accepted` with change and attachment IDs and continues after client disconnect.
2. First attach creates one distributed change idempotently and stores its ID without backend endpoints, ingress credentials, or managed host paths.
3. A second node resumes that change into a server-allocated empty directory and reconstructs a byte-identical projection. An HTTP caller supplies no client-local project path.
4. Both nodes continue to author only their own logs. Graceful detach by CLI stop or attachment control leaves an ordinary, current change home that requires no Git metadata to read.
5. Two nodes cannot own the same slice. Confirmed recovery advances its ownership generation; stale conditional writes fail, and stale events or results cannot affect projection or release the recovered ownership.
6. Project and source snapshots materialize remotely to the same tree digests. Digest mismatches fail closed.
7. A remote worker pool uses Omnia multi-node private workspaces and the same ownership and composition rules as a local pool. Given the same recorded patches, both placements produce the same composed candidate.
8. Remote verification retains the same typed, model-assisted report contract as local verification without promising byte-identical findings from a fresh model call.
9. Given the same accepted facts, planning revisions, model-capability profiles, domain records, and values, desktop-only and two-node execution produce the same target-wave CID and slice statuses.
10. Publishing the same domain-operation result twice is idempotent. If independently evaluated results differ, the first byte-valid record matching the validated-input and accepted-frontier digest remains authoritative and the loser cannot alter projection.
11. An event arriving before its referenced records or CIDs remains invisible. Once its dependencies verify, resume reuses the completed domain round without recomposition or reverification.
12. Process loss at every phase boundary resumes from per-writer sequences, the ownership generation, and content-addressed values without shared filesystem state.
13. Failure before capability provisioning, after value publication, or while following events resumes without duplicate change IDs, duplicated facts, or forge or product-tree mutations.
14. General Omnia conformance tests prove durable cursor replay, native compare-exchange under races, expiry without ownership transfer, remote guest routing, placement-local workspace access, cancellation, byte-identical snapshot round trips, and disconnect-independent HTTP invocation control.
15. `cargo make ci` is green in every touched repository. Two-node Emery integration tests cover CLI/HTTP ingress equivalence, asynchronous attachment control, attach, resume, event replication, stale-work rejection, value integrity, remote materialization, worker placement, concurrent slices, and cross-target convergence.
16. Local and two-node live fixtures with the same source and input set, model configuration, time budget, and blind acceptance set report judgment, slice-ownership-wait, workspace-placement, object-transfer, and result-publication latency separately. Blind grading and runtime metrics remain outside workflow authority.

## Prior art

- **[Bazel Remote Execution API](https://github.com/bazelbuild/remote-apis)** — the same coordination/value split proven at datacenter scale: a content-addressable store for values and an action cache keyed by the digest of a fully specified action. D4's domain publication without slice ownership — the operation key as the digest of validated inputs and accepted frontier, first byte-valid record wins, idempotent duplicates — is that pattern.
- **[wasmCloud](https://wasmcloud.com/blog/wasmcloud-v2-is-here/) v1 → v2** — v1 routed every component import implicitly over a wRPC/NATS lattice; v2 reversed to in-process by default with explicit, deliberate distribution, because implicit distribution tied invocation semantics to transport failure modes. This RFC's explicit capability seams, adapter worlds importing no distribution capability, and the desktop degenerate case are the posture wasmCloud arrived at after operating the transparent mesh.
- **[iroh](https://www.iroh.computer/)** — dial-by-key peer-to-peer QUIC with content-addressed, BLAKE3-verified blob streaming. Relevant strictly on the value plane per the transport-boundary asymmetry; it offers no linearizable primitive, so it is not a coordination candidate.

## Rejected alternatives

- **An Emery-specific synchronization host** — would duplicate durable logs, atomic state, blob storage, remote dispatch, and multi-node workspaces that improve the general Omnia runtime. Emery contributes semantic requirements and uses the general capabilities.
- **Keep one HTTP request open for the attachment lifetime** — multi-day execution cannot depend on one client connection or intermediary timeout. The native host supervises a durable invocation and gives control back through its opaque attachment ID.
- **Use HTTP as the node-sync transport** — conflates operator ingress with typed guest dispatch, durable logs, slice ownership, and value transfer. HTTP starts and controls an attachment; Omnia capabilities and wRPC carry distributed execution.
- **Compose lossy publish and read-then-write state** — cannot provide cursor replay, linearizable ownership acquisition, recovery that rejects stale writes, or first-writer-wins domain publication.
- **Guest-to-guest peer synchronization** — would move cursor negotiation, anti-entropy, and reconnection state into the engine guest, duplicate the durable log's guarantees, and make resume depend on the originating node being online. Events flow only through the durable-log capability.
- **Peer-to-peer coordination state** — the host-backend version of the same temptation: gossip, CRDT replicas, or peer pubsub as the backend for slice-ownership records and writer logs cannot provide linearizable compare-exchange or first-writer-wins publication without a consensus layer, and state held only on participating peers breaks D5's durability contract. Peer transfer stays available on the value plane, where digest verification makes the path irrelevant.
- **Shared volumes or network filesystems** — couple failure domains, require distributed locking, depend on location, and break the `local-path` lending model.
- **CRDT-synchronized live trees** — solve a problem the immutable-snapshot execution model does not have while making intermediate states difficult to verify.
- **A hosted-only scheduler or convergence policy** — would give desktop and fleet execution different semantics.
- **Planning or event records in the snapshot value namespace** — would mix coordination, which needs liveness and per-writer sequencing, with content-addressed product data.
- **A second event model beside the writer logs** — would create dual-write drift. The durable-log capability carries the authoritative writer events instead.
- **Distributed authority cutover** — would make external infrastructure necessary to interpret a local change and create a one-way lifecycle boundary.

