# RFC-92: Distributed Execution

> Status: Draft — step 7 of the platform-migration series, scale track ([platform.md](platform.md))
>
> Owns: running one change across multiple nodes without changing its execution semantics. Adds distribution for facts, planning artifacts, domain rounds, and snapshots; slice ownership generations and stale-work rejection; claim-based remote execution and worker pools; and attach, resume, and detach.
>
> Does not own: scheduling policy, result convergence, merge semantics, workflow authority, or lifecycle.
>
> Depends on completed [RFC-86](rfc-86-change-facts.md), [RFC-87](rfc-87-working-trees.md), [RFC-88](rfc-88-detached-changes.md), [RFC-90](rfc-90-build-verification.md), and [RFC-91](rfc-91-concurrent-execution.md).
>
> Related: [RFC-89](rfc-89-publication-sets.md) binds publication across repositories after this RFC's distributed execution.
>
> Runtime dependency: Omnia bindings for `wasi:documentstore`, native `wasi:keyvalue` atomics, `wasi:blobstore`, `wasi:messaging` wake-up announcements, and durable asynchronous trigger supervision. Where an interface or backend lacks a required guarantee, this RFC requires improving the general Omnia capability rather than adding an Emery-specific transport API.

## Intent

*Let one change run across peer nodes **without sharing a filesystem**.*

Emery can already run a change on a single machine. Each slice is given to a worker along with a private workspace. Progress events are logged, slice results combined, and commited as a group.

This RFC adds the transport and safeguards needed to preserve that same behavior **across nodes**.

Each node continuously shares progress and ownership updates. They exchange code only as immutable, verified snapshots after an operation completes. Emery will not synchronize keystrokes, editor buffers, or live directories.

No component selects a node for an operation. Work reaches a node because that node claimed it: the slice owner publishes a durable operation offer, eligible nodes race to claim it, and the winner executes entirely locally. There is no node registry, capability inventory, or placement scheduler anywhere in the system.

A single desktop — the degenerate case — uses the same execution model through local Omnia capability implementations and requires no distributed-runtime configuration: the local node claims every offer it publishes.

## Existing execution contract

This RFC distributes the existing contract:

- Each journal writer appends only to its own event log. Emery combines these logs to calculate status.
- An ownership record identifies which writer owns a slice and which ownership generation is current.
- Every code-writing worker starts with immutable inputs in a fresh private workspace and returns an immutable result. Verification and review receive fresh materializations of the current candidate. Their incidental writes are recorded for inspection and discarded; they do not become part of the accepted result.
- Slices may run concurrently, with results combining upward from slice leaves through the recorded domain hierarchy.
- Emery groups slices for the same target repository into a **target wave** and commits all their results together. If any result cannot be accepted, none are committed.
- Recovery proposals never amend a plan automatically. Only operator-invoked amendments change plan authority.

## Omnia runtime

Each participating node runs the native `emery` executable as an Omnia host runtime. The runtime embeds the engine guest, binds host capabilities before the guest starts, and resolves source and target adapter guests **locally**. Backend endpoints and credentials remain in the native host; guests receive only typed WIT capabilities.

This RFC composes existing storage interfaces with Omnia runtime capabilities:

- `**wasi:documentstore**` stores workflow events as immutable documents keyed by writer and sequence and queries each writer's events after a sequence cursor;
- `**wasi:keyvalue` atomics** provide native linearizable compare-exchange and conditional update for slice-ownership records, operation claims, and first-writer-wins publication;
- `**wasi:blobstore**` carries immutable coordination records — including operation offers — and snapshot values under separate logical namespaces;
- `**wasi:messaging**` announces new offers on capability-scoped topics as a pure wake-up; delivery may be lost or duplicated without affecting correctness.

These are runtime primitives, not workflow policy. Emery still decides eligibility, authorization, ownership, stale-work rejection, dependency visibility, convergence, and acceptance. If an existing Omnia interface is lossy, racy, or local-only where a guarantee is required, its general contract and backends must be strengthened before Emery relies on it.

No guest invocation crosses nodes. A node that claims an operation prepares the workspace through its own RFC-87 workspace capability and invokes its locally resolved adapter guest against it; workspace and invocation are collocated by construction, because the same node created both. A workspace ID never leaves the node that minted it, and no live WIT resource or filesystem path ever crosses the wire.

Source and target adapters import no coordination capabilities and can acquire neither slice ownership nor operation claims. They continue to receive one operation request and one private workspace. Claiming is engine-runtime behavior on the worker node; a claimed execution is subordinate to the slice owner and does not author a second workflow log.

Operator ingress is separate from inter-node execution. The engine exposes the same typed workflow operations through CLI and HTTP. An interactive deployment may invoke `emery plan execute --distributed` and remain attached to the process. A hosted deployment submits the execute operation through `POST /plan/execute`; the native host authenticates the request, allocates or selects its managed change home, and supervises the engine invocation after returning `202 Accepted` with an opaque attachment ID. Request disconnection does not stop the attachment. Status, event follow, and graceful detach address that attachment through general Omnia invocation-control surfaces rather than a second Emery workflow.

The HTTP trigger does not carry offers, claims, writer logs, slice-ownership records, or snapshot values. Once the execute operation starts, those flows use the capabilities above. HTTP is an operator control surface, not the distributed-execution protocol.

## Distributed execution

1. The operator invokes the distributed execute operation against an authored change home, either through the attached CLI or authenticated HTTP control surface.
2. Emery records `plan.execute.started`, opens the change's distributed session through the configured Omnia capabilities, and registers a writer ID for the local engine runtime.
3. The RFC-91 execution loop identifies slices whose dependencies and workflow gates are satisfied. Before offering any operation for a slice, an attached Emery runtime must atomically acquire slice execution ownership.
4. For each ready operation, the slice owner durably publishes an **operation offer** — the guest identity to invoke, the content-addressed input tree identity, the access manifest, capability requirements, the authorization epoch, the owner's writer ID, and the current ownership generation — then announces it on a capability-scoped `wasi:messaging` topic. Eligible nodes race to claim the offer through a linearizable compare-exchange; the first successful claim wins.
5. The claiming node fetches the input closure through the value plane, verifies it, prepares a fresh private workspace through its local workspace capability, and invokes its locally resolved adapter guest. When a writing operation finishes, capture freezes the resulting repository tree into an immutable snapshot and verifies every object needed to reconstruct it before the node publishes the result record under the offer's identity.
6. Other nodes follow the writer logs and fetch referenced records or snapshot values as needed. A node may use a result only after every dependency is present and digest-verified.
7. Any attached engine runtime with the required inputs may continue the existing bottom-up convergence. A target-wave commit remains the only operation that advances a target repository's accepted state.

### Transport boundaries

This RFC keeps three concerns separate even when one backend implements more than one capability:

- **Coordination** carries slice-ownership records, operation offers and claims, and durable writer events, plus the immutable planning and domain records those events reference. It uses `wasi:documentstore`, `wasi:keyvalue` atomics, and a coordination namespace in `wasi:blobstore`.
- **Values** are immutable project, source, and result snapshots. Each node's workspace capability moves their content-addressed object closure through a logically separate `wasi:blobstore` namespace.
- **Publication** remains on the forge through branches and pull requests. It is unaffected and operator-owned.

`wasi:messaging` belongs to neither plane's authority. An announcement is a wake-up that shortens the interval between offer publication and claim; a worker node may equally discover unclaimed offers by scanning the coordination plane. A lost announcement delays work, a duplicated one is absorbed by claim arbitration, and neither affects correctness.

Coordination may report that work exists before its larger snapshot values become locally available, but Emery does not expose a result to projection until its complete dependency set is present and verified. Logical separation does not require separate infrastructure: one host backend may satisfy several capabilities without making its resource names part of the guest contract.

The two planes have opposite trust and liveness profiles, and backend selection must respect the asymmetry. Values are self-certifying: a CID names its own bytes, so a snapshot object may arrive from a hosted store, a relay, or a nearby peer without weakening correctness — verification happens on read either way. Coordination is authority-bearing: slice ownership, operation claims, and per-writer cursors require a linearizable, always-on point of serialization, and the durability contract in D5 — resume requires no other node online — cannot be met by state that lives only on participating peers. Peer-to-peer transfer is therefore a legitimate value-plane backend option and never a coordination-plane substitute.

### Key terms

- A **journal writer** is a stable identity with exclusive append authority over one `.emery/events/<writer-id>.jsonl` log and its sequence namespace. In distributed execution, an attached Emery runtime becomes a slice owner under its writer ID; nodes executing claimed operations are not journal writers.
- A **CID** is the content digest of an immutable snapshot.
- **Slice execution ownership**, shortened below to **slice ownership**, is the exclusive responsibility of one journal writer to progress one slice. RFC-86 records its acquisition as `slice.claimed`; distributed execution adds an ownership generation to reject stale work. Ownership never grants workflow authority.
- An **ownership generation** is a number that increases whenever slice ownership is recovered. Every event, result, and release carries the current generation. After it increases, Emery rejects anything carrying an older generation.
- An **operation offer** is the durable, domain-neutral record by which a slice owner exposes one ready operation for execution: operation ID, guest identity, input tree CID, access manifest, capability requirements, authorization epoch, writer ID, and ownership generation. It never carries a node endpoint, filesystem path, or workflow lifecycle state.
- A **claim** is the lease-bound record, acquired by linearizable compare-exchange, that binds one node to one offer for the lease's duration. Workspace placement is the emergent result of claiming, not a scheduling decision.
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

Node B's host allocates an empty managed change home; an HTTP caller never supplies a path meaningful only on the caller's filesystem. The equivalent interactive CLI remains `emery plan execute --distributed --change-id 0198a40f-… --project-dir ./checkout-v2`. Node B verifies the received sequences and artifact digests, reconstructs the change tree, registers its own writer ID, and acquires execution ownership of any eligible unowned slices with fresh ownership generations. It also begins subscribing to the offer topics its capabilities match, so operations offered by node A may execute on node B. Each writer continues to append only to its own event log. Replicated events retain their original writer IDs.

Suppose node B disappears while owning `mobile-shell` at generation 18. An expiry notification reports suspected loss but does not transfer ownership. During a later `plan execute`, the operator may explicitly confirm recovery after Emery verifies that the slice's immutable base or most recently published result can be fully reconstructed and passes digest verification. Emery then atomically advances the ownership record to generation 19. Any event, result, or release carrying generation 18 is rejected before it can affect projection. Any operation claim node B held simply expires; the owning runtime re-offers the operation, and a late result under the expired claim is rejected by its stale claim identity.

A graceful stop first brings the local change home current and then detaches from the distributed session. Reattaching reconstructs the same projected state. `plan archive` closes the attachment before performing its ordinary archive or delete behavior.

## Decisions

### D1 — Omnia capabilities preserve the transport boundaries

Events use `wasi:documentstore`. Slice-ownership records, operation claims, and first-writer-wins records use native `wasi:keyvalue` atomics. Immutable planning records and operation offers use a coordination namespace in `wasi:blobstore`. Product and source snapshots move through each node's workspace value path over a separate `wasi:blobstore` namespace. `wasi:messaging` carries only capability-scoped offer announcements. Publication remains on the forge.

This lets each concern use the consistency model it needs and prevents large code objects from blocking coordination updates.

The engine guest imports capability contracts, never backend identities, endpoints, subjects, bucket names, or credentials. Source and target adapter worlds import none of the distribution capabilities. A deployment profile may bind several contracts to one service, but that mapping remains native host policy. A future blob backend may fetch snapshot values peer-to-peer without changing the contract: verify-on-read makes the transfer path irrelevant to correctness. Per the transport-boundary asymmetry above, that peer-to-peer option exists only on the value plane; the coordination capabilities always bind to a linearizable, durably hosted backend.

### D2 — Remote code results cross nodes only as verified snapshots

A claiming node performs each code-writing operation entirely in its private workspace. No shared volume, network filesystem, patch blob, persistent source copy, live directory handle, or intermediate edit crosses the wire. After the operation finishes, the workspace freezes the resulting repository tree into an immutable snapshot, stores and verifies every object needed to reconstruct it, and derives the touched paths by comparing it with the operation's base snapshot. This is the capability's `capture` operation.

Only after this process succeeds may the result be published for another node to use.

Dependants create fresh private workspaces from that immutable result. Domain gates publish immutable domain-round records instead of code snapshots. Result-availability latency is therefore the operation's runtime plus snapshot storage, verification, and transfer latency.

### D3 — Ownership generations reject stale work

An attached Emery runtime offers operations for a slice only after acquiring slice ownership through a linearizable compare-exchange. The ownership record stores that runtime's writer ID, and its ownership generation accompanies every offer, event, result, and release produced under that ownership.

If an ownership record expires, its owner is treated as possibly unavailable. However, ownership does not automatically move to another runtime, rather, an operator must explicitly authorize recovery.

To recover, the slice’s latest code state is reconstructed—either its immutable base or its latest published result—and digest verified. Slice ownership is then incremented in one atomic operation. Events, results, and releases carrying an older generation are rejected and cannot affect workflow state.

An operation claim is subordinate to slice ownership and carries the offer's generation; recovery invalidates outstanding claims along with everything else minted under the old generation. Fresh private workspaces need no second lock. Within a worker pool, task grants continue to partition path ownership.

### D4 — Claiming cannot change execution semantics

RFC-91's execution rules remain the only definition of slice eligibility, domain readiness, composition, verification, and target-wave membership. Claiming decides only where already-eligible work runs; a claim cannot infer a different hierarchy or acceptance policy.

The slice owner controls what is offered and remains the sole acceptor of results. A claiming node receives a validated operation, task grant, and immutable inputs through the offer — nothing else. Every result produced under that ownership carries its authorization epoch, writer ID, ownership generation, and complete input identity. Emery rejects a result if its slice identity or its lead-catalog, decomposition, model-capability-profile, wave, dependency-frontier, spec, or base digest does not match the current operation.

Domain convergence checks do not belong to a slice owner. Once their child results are ready, any attached runtime may perform the check from the same immutable inputs. Emery hashes those inputs and the current accepted target state to identify that exact check.

Two runtimes may finish the same check concurrently, and model-assisted verification may produce different results. Emery therefore records the first structurally valid result atomically. Repeating that result is harmless; a later different result is ignored. `wasi:blobstore` holds the result, while a `wasi:keyvalue` compare-exchange records which result digest won. This selects one authoritative result without claiming that the model’s output is deterministic.

### D5 — Transport carries facts; it does not become their authority

Each journal writer remains the only author of its workflow events. Events are persisted (`wasi:documentstore`) keyed by writer ID and sequence. Other nodes query each writer's events in sequence order and resume after their last received sequence when reconnecting.

Per-writer sequence numbers make delivery idempotent and let each follower resume independently. A node stores received events under their original writer IDs and runs the existing projection over the combined logs. Delivery may be at least once; gaps remain pending until missing sequences arrive. No authority cutover, dual-write protocol, or separate reconciliation model is introduced.

An attached change's coordination state is durable independently of every participating node; resume requires no other node online. A disconnected node retains its last received state and pauses distributed work until its capabilities reconnect.

### D6 — Referenced records and values arrive before their facts become visible

Immutable workflow records—planning revisions, model profiles, amendment proposals, and domain-round records—are stored centrally in a **coordination container** in `wasi:blobstore`.

Project, source, and result snapshot objects are stored in a separate `wasi:blobstore` container. Each node's workspace implementation reads those objects when creating a private workspace and writes them when freezing or capturing a workspace.

An event that references one of those objects remains invisible to projection until the complete dependency set is present and digest-verified. This ordering lets another node resume a completed domain round without recomputing it. Garbage collection treats replicated live records as roots.

### D7 — Placement is claim-based: eligible nodes pull work

No component selects a node for an operation. The slice owner publishes a durable operation offer on the coordination plane; nodes able to satisfy it claim it; the first successful claim wins. Placement is the emergent result of claiming, not a scheduling decision, and no node registry, capability inventory, or placement algorithm exists anywhere in the system.

An offer is domain-neutral at the transport: it carries an operation ID, the guest identity to invoke, the content-addressed input tree identity, the access manifest, capability requirements, the authorization epoch, the slice owner's writer ID, and the current ownership generation. Emery decides which operation is ready and translates its task grants into the access manifest; the offer never carries a node endpoint, filesystem path, or workflow lifecycle state.

The offer record is authoritative and lives in the coordination plane. `wasi:messaging` announces it on a capability-scoped topic — for example, one topic per guest identity and platform requirement — so only suitable nodes wake. Messaging is strictly a wake-up: delivery may be lost or duplicated without affecting correctness, because a worker node may also discover unclaimed offers by scanning the coordination plane, and every claim is arbitrated by a linearizable `wasi:keyvalue` compare-exchange. The claim record carries the claiming node's identity, a liveness lease, and the offer's ownership generation. A node self-assesses eligibility before claiming; a misconfigured node that claims work it cannot complete degrades progress through lease expiry and re-offer, never correctness.

The claiming node then executes entirely locally. It fetches the input closure through the value plane, verifies it, prepares a fresh private workspace through its own RFC-87 workspace capability, and invokes its locally resolved adapter guest against it. `freeze`, `prepare`, `capture`, `compose`, `discard`, and cancellation are node-local calls on the claiming node; a workspace ID never leaves the node that minted it, and no live WIT resource or local path ever crosses the wire. After the operation finishes, capture stores and verifies every result object (D2), and the node publishes the result record — carrying the operation ID, epoch, writer ID, ownership generation, and complete input identity — for the slice owner to accept under D4's rejection rules.

A claim is disposable, exactly as its workspace is. If the claiming node or its lease is lost, the claim expires as suspected loss; the slice owner re-offers the operation under the same fencing rules that govern slice ownership, and a late result from the expired claim is rejected by its stale claim identity. A completed snapshot remains available by digest; a lost claim and its workspace do not become recoverable state. Explicit cancellation is a coordination-plane record the claiming node observes; lease expiry bounds the window in which an unobserved cancellation can consume resources. Duplicate execution after a lost claim is safe: results are immutable, digest-named, and accepted at most once by the slice owner.

First-claim-wins is deliberately policy-free: there is no load balancing, data-affinity preference, or cost-based selection beyond what topic scoping encodes. If a smarter policy is ever needed, it layers on as claim back-off heuristics in workers without changing this contract; no scheduler is reintroduced for it.

Worker pools use the same ownership, composition, verification, and reporting sequence whether their workers claimed locally or remotely. Workers never share a writable tree, workspace ID, MCP state, or prompt state, and a claiming node is never a journal writer — its result is subordinate to the slice owner. The host scopes claims to their lease and generation and rejects unknown or stale claim identities. Byte-identical snapshots round-trip between claiming nodes.

Emery provides no CRDT trees, synchronized editor buffers, or simultaneous multi-writer access to one path.

### D8 — Distribution cannot amend the plan

Runtime overlap, refinement-boundary escalation, and target decomposition may produce validated amendment proposals. This RFC replicates those proposals but never applies them.

Only the operator-invoked compare-and-set amendment API may revise lead, decomposition, or plan authority. Distribution adds no hidden recovery or amendment writer.

### D9 — The execute operation attaches and resumes through CLI or HTTP

The execute operation is transport-neutral. In an authored change directory, `emery plan execute --distributed` invokes it directly and remains attached. In a hosted deployment, authenticated `POST /plan/execute` invokes the same operation against a host-managed change home and returns `202 Accepted` with opaque change and attachment identities.

Supplying a change ID reconstructs the change tree, registers a fresh writer ID, acquires ownership of eligible slices, and resumes the ordinary execution loop. The CLI names an empty host-local `--project-dir`; HTTP deployments allocate that directory and never interpret a client-local path. Resume is deterministic reconstruction, not a second recovery protocol.

The CLI process lifetime may remain the attachment lifetime for an interactive deployment. A hosted attachment is a durable host-supervised invocation: it survives request disconnection and is addressed by its attachment ID for status, event follow, and graceful detach. A graceful stop synchronizes before detaching. The retained change home remains readable through the ordinary operations, and a later invocation may reattach it.

### D10 — Missing primitives improve Omnia generally

This RFC does not assemble safety-critical behavior from interfaces that lack the required guarantees. An unconditional document upsert is not append-only event persistence, read-then-write key-value state is not compare-exchange, blob replacement is not create-if-absent publication, and a broadcast wake-up is not a claim.

The required improvements land in the existing Omnia capability bindings and backend conformance:

- create-only document insertion plus stable writer-and-sequence queries after a cursor;
- native `wasi:keyvalue` compare-exchange, conditional update, and lease-liveness observation for ownership records and operation claims;
- topic-scoped `wasi:messaging` announcement conformance — deliberately weak: at-most-once delivery and duplicate delivery are both acceptable, because announcements are wake-ups over durable coordination state;
- durable asynchronous trigger supervision with authenticated submit, opaque invocation identity, status, event follow, cancellation, and disconnect-independent execution.

Emery supplies the workflow semantics layered over those primitives. Omnia supplies reusable capability contracts, local test implementations, and production backends. This RFC requires no remote guest dispatch, no placement-aware guest resolution, and no cross-node workspace routing: adapters always run on the node that claimed the work, through the local dispatch path that already exists. Exact backend configuration remains deployment documentation rather than part of this RFC.

Omnia additions remain domain-neutral: their contracts may name content identities, access manifests, guest identities, capability requirements, claim leases, and invocation IDs, but never Emery changes, slices, tasks, writers, waves, or lifecycle states. This RFC owns only the workflow requirements that consume them.

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

The `mobile` dependency affects scheduling only. Once the required `add-refund-endpoint` result is accepted, `adopt-refund-ui` runs against the recorded `mobile` base — offered on the mobile capability topic and claimed by whichever suitable node responds first. Emery never applies the payments patch to the mobile repository; each target produces and verifies its own candidate.

If both payments results unexpectedly touch `src/errors.rs`, their nearest shared domain stops before composition. Emery replicates the immutable results and an inert amendment proposal. Unaffected domains may continue, but only the operator may apply the proposal and start a new closed-plan execution epoch.

## Evaluation

Distributed evaluation must separate judgment time, slice-ownership wait, offer-to-claim latency, object transfer, and result-publication latency. Local and remote runs use the same source and input set, model configuration, time budget, and blind acceptance set.

The completion workload is an Omnia/Rust multi-project change using the engine-owned `build` / `repair` / `verify` / `review` loop and remote worker pools. Every remote verification and repair dispatch records the same typed phase report as its local equivalent. Distribution does not make model-assisted evidence deterministic.

Throughput alone is not success. Evaluation compares the accepted result and blind grade while reporting claim and transfer costs separately.

## Implementation requirements

### Omnia capabilities

- Use `wasi:documentstore` for workflow events. Store one immutable document per `(writer, sequence)` using create-only insertion. On an existing ID, accept byte-identical content idempotently and reject different content. Query one writer at a time after its last received sequence in ascending sequence order. Production backends must retain acknowledged inserts and preserve stable ordered queries across reconnects; local test backends must expose the same contract. Cassandra may be a candidate backend for this.
- Use native `wasi:keyvalue` atomics for linearizable compare-exchange and conditional update. A read followed by an unconditional write does not qualify. Ownership and claim liveness may use native expiry or heartbeat values plus observation, but expiry reports only suspected loss and never transfers ownership. Backend conformance tests must race slice-ownership acquisition, recovery, release, first-result publication, claim acquisition, claim-lease lapse against late result publication, and heartbeat lapse against recovery confirmation.
- Use `wasi:blobstore` for immutable coordination records — including operation offers — and snapshot values under separate logical namespaces. Emery names immutable objects by digest, verifies existing and fetched bytes, and never relies on backend overwrite behavior for authority. The value namespace admits peer-to-peer backends — verified-streaming transfer between nodes (for example, iroh-blobs' BLAKE3 verified streaming) suits large snapshot closures — provided the backend's native addressing stays behind the capability: Emery verifies fetched bytes against its own recorded digests regardless of what the transfer layer verified, so a backend whose transfer hash differs from Emery's digest maps between them internally.
- Use `wasi:messaging` only to announce offers on capability-scoped topics. The contract is deliberately weak: at-most-once delivery, duplicate delivery, and reordering are all acceptable. No claim, result, event, or recovery decision may depend on an announcement arriving; a worker that misses an announcement must be able to find the same offer by scanning the coordination plane. Backends need prove nothing beyond topic-scoped delivery when connected.
- Add general durable HTTP-trigger supervision for long-running guest operations. Submission authenticates at the native host, returns `202 Accepted` with an opaque invocation ID, and lets the invocation continue after client disconnect. Status, event follow, graceful cancellation, and result retrieval use that ID. Emery maps the invocation to an attachment but adds no second execution lifecycle or HTTP-based distributed-execution protocol.
- Keep backend service discovery, endpoints, credentials, resource names, transfer encoding, and chunking in the native Omnia backend. Define no Emery user directory or authentication service.
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
- Keep slice execution ownership with the attached engine runtime. Claimed extract, build, repair, verify, review, and domain executions are subordinate to that ownership rather than independent journal writers.
- Publish every operation offer durably before announcing it, and make the offer domain-neutral on the wire: operation ID, guest identity, input tree CID, access manifest, capability requirements, epoch, writer ID, and ownership generation.
- Arbitrate every claim by compare-exchange with a liveness lease and the offer's generation. On lease expiry, re-offer under the same fencing rules; reject a late result by its stale claim identity. Record explicit cancellation on the coordination plane for the claiming node to observe.
- Require the claiming node to execute locally: fetch and verify the input closure, prepare a fresh node-local private workspace, invoke the locally resolved adapter, and return a verified code patch and typed report through the workspace capability before publishing the result.
- Require the authorization epoch, ownership generation, claim identity, and complete input identity on every result produced under slice ownership.
- Require the validated-input and accepted-frontier digest as the operation key. Domain publication requires no slice owner and atomically accepts the first byte-valid record matching that key.
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

- **An Omnia placement control plane** — a cluster node registry, per-node capability inventory, health model, and placement scheduler would centralize a decision that claim-based dispatch makes locally, require Omnia to track fleet state no other capability needs, and put a live workspace-to-node binding on the critical path of every remote call. Claiming needs only the compare-exchange primitive this RFC already requires for slice ownership.
- **Push placement over remote guest dispatch** — routing an adapter invocation to the node holding its workspace requires collocation bookkeeping, workspace-ID-to-node routing, and placement-aware remote guest resolution. Pulling the work to the node makes collocation structural: the claiming node prepares the workspace and invokes the guest locally, so nothing needs to route.
- **An Emery-specific synchronization host** — would duplicate document storage, key-value atomics, blob storage, and messaging that improve the general Omnia runtime. Emery contributes semantic requirements and uses the general capabilities.
- **Messaging as claim or delivery authority** — claims, results, and recovery decisions that depend on broker delivery inherit its loss, duplication, and ordering semantics. Announcements stay wake-ups; durable offers plus compare-exchange claims carry the authority.
- **Keep one HTTP request open for the attachment lifetime** — multi-day execution cannot depend on one client connection or intermediary timeout. The native host supervises a durable invocation and gives control back through its opaque attachment ID.
- **Use HTTP as the distributed-execution transport** — conflates operator ingress with document-backed writer events, offers, claims, and value transfer. HTTP starts and controls an attachment; Omnia capabilities carry distributed execution.
- **Compose lossy publish and read-then-write state** — cannot provide cursor replay, linearizable ownership or claim acquisition, recovery that rejects stale writes, or first-writer-wins domain publication.
- **Guest-to-guest peer synchronization** — would move cursor negotiation, anti-entropy, and reconnection state into the engine guest, duplicate the document store's guarantees, and make resume depend on the originating node being online. Events flow only through `wasi:documentstore`.
- **Peer-to-peer coordination state** — the host-backend version of the same temptation: gossip, CRDT replicas, or peer pubsub as the backend for slice-ownership records, claims, and writer logs cannot provide linearizable compare-exchange or first-writer-wins publication without a consensus layer, and state held only on participating peers breaks D5's durability contract. Peer transfer stays available on the value plane, where digest verification makes the path irrelevant.
- **Shared volumes or network filesystems** — couple failure domains, require distributed locking, depend on location, and break the node-local workspace model.
- **CRDT-synchronized live trees** — solve a problem the immutable-snapshot execution model does not have while making intermediate states difficult to verify.
- **A hosted-only execution or convergence policy** — would give desktop and fleet execution different semantics.
- **Planning or event records in the snapshot value namespace** — would mix coordination, which needs liveness and per-writer sequencing, with content-addressed product data.
- **A second event model beside the writer logs** — would create dual-write drift. `wasi:documentstore` carries the authoritative writer events instead.
- **Distributed authority cutover** — would make external infrastructure necessary to interpret a local change and create a one-way lifecycle boundary.

