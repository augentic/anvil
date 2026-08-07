# RFC-86: Change Facts

> Status: Draft — step 1 of the platform-migration series ([platform.md](platform.md)); the substrate every later step stands on
>
> Owns: the fact-based change substrate — the change as a self-contained tree of immutable, digest-identified artifacts and append-only per-actor event logs; workflow status as a projection over facts, never a stored field; pinned bases for every judgment leg; slice-scoped requirement identity finalized through an immutable target-wave commit; `plan.execute.started` as an explicit operator authorization epoch distinct from ownership claims and per-leaf input fences; and the single-actor desktop as the degenerate case of the same substrate, not a separate mode.
>
> Depends on: nothing in the series. Consumed by every later step: [RFC-87](rfc-87-working-trees.md) (snapshot and result identities join the fact vocabulary), [RFC-88](rfc-88-detached-changes.md) (the detached change home becomes a fact tree), [RFC-89](rfc-89-publication-sets.md) (publication verification reads execute-start and other facts), [RFC-91](rfc-91-concurrent-execution.md) (concurrent synthesis requires merge-finalized identity), [RFC-92](rfc-92-node-sync.md) (node sync becomes fact-and-value transport, with no authority cutover).

## Intent

Before distributing the work, fix the base. Today's structure is correct about its lifecycle units — the slice as the executable unit of isolation, the delta spec as the unit of intent, the plan as the umbrella — and wrong about exactly four mechanics, each of which works only because *one operator, one session, one tree* is currently true. RFC-88 may introduce persisted conflict domains above slices without creating another lifecycle unit: internal domains explain and constrain decomposition, while only terminal domains become slices and carry facts.

| Today | Where | Why it cannot distribute |
| ----- | ----- | ------------------------ |
| Workflow status is stored as mutable fields | `plan.yaml` per-entry `status` ladder (`crates/project/src/plan/model/state.rs`), `metadata.yaml` lifecycle (`crates/project/src/slice/lifecycle.rs`) | Two actors' trees cannot merge; every stored ladder is a synchronization problem RFC-92 would otherwise have to solve |
| The journal is one append-only file | `.emery/journal.jsonl` (`crates/project/src/journal.rs`) | Single files conflict under any concurrent append or tree union; the worst possible shape for a distributed log |
| Requirement identity is allocated at synthesis, relative to the baseline seen then | `IdAllocator` (`crates/slice/src/synthesis/project.rs`) | Two slices refined against the same baseline deterministically mint colliding `REQ-NNN` ids; `MODIFIED` is silent last-writer-wins |
| Execute leaves no durable start fact | "running `emery plan execute` is the approval (nothing is stamped or recorded)" | An execute invocation on one desktop cannot travel, be audited, or prove which digests authorized later work on a second actor or node |

This RFC replaces those four mechanics with one substrate: **a change is a self-contained tree of immutable facts; all status is projected, never stored; every judgment output is a persisted, pinned, digest-identified artifact; and `plan execute` records `plan.execute.started` as the operator's durable authorization epoch before privileged work.** Authorization, ownership, and input identity are orthogonal: the execute-start fact grants work, a claim assigns one leaf to one actor, and each build or merge fact pins the exact inputs it consumed. There is no plan-approval verb, status, or artifact — execute is the only operator surface, and the fact is an orchestration event like `slice.build.started`. Everything else — the WIT seam, the adapter contract, the delta-spec grammar, the evidence/model schemas, the validation registries, and the slice loop — stays load-bearing.

The outcome this purchases: **Emery works on one desktop exactly as it works in a multi-node cloud environment.** The engine guest is already location-neutral by construction — it is a Wasm component whose deployment differences live entirely in providers and the launcher. This RFC makes the *state* location-neutral to match: facts and values move through deployment-provided coordination and content-addressed-value transports; a desktop is simply the deployment with one actor and no remote. There is no hosted mode, no attach cutover, and no second lifecycle model.

It also purchases the operator story directly: a plan's specs and designs become a reviewable, portable set of pre-build artifacts. Planning can extend over days and multiple operators; a build failure loses nothing; review happens against pinned, diff-shaped delta specs rather than lead synopses; and a bug fix is the same structure with a one-slice footprint.

## Prior art

Every mechanism in this RFC is a settled pattern elsewhere; the design work is choosing which to adopt and which to reject.

| System | Pattern adopted | Pattern rejected |
| ------ | --------------- | ---------------- |
| [git-bug](https://github.com/git-bug/git-bug/blob/master/doc/design/data-model.md) / [Radicle COBs](https://radicle.dev/guides/protocol) | Mutable state represented as append-only operations; the snapshot is always derived by deterministic replay, never persisted as authority | Git object storage and Lamport-clock CRDT merge of concurrent edits to one entity — Emery's fact tree is version-control-neutral, and its exclusive per-slice claims make cross-actor conflicts a detected violation rather than a merge to resolve |
| [Jujutsu](https://jj-vcs.dev) | The stable-identity / content-identity split: a *change id* survives rewrites while the commit id tracks content — mirrored here as slice-scoped requirement identity vs the baseline number finalized at merge | Treating identity as local-only state; Emery's identity mapping is a recorded merge fact |
| [Bazel Remote Execution](https://github.com/bazelbuild/remote-apis/blob/main/build/bazel/remote/execution/v2/remote_execution.proto) | Work as a content-addressed pure function: an action is the digest of its command plus input tree; any compatible worker can execute it; outputs return by digest | The action *cache* — judgment legs are not deterministic, so Emery records outcomes as facts rather than deduplicating executions |
| [in-toto attestations](https://github.com/in-toto/attestation/blob/main/spec/README.md) / SLSA | Authorization and provenance as statements binding a predicate to immutable digest-identified subjects, consumable by policy engines and other actors | Signature/envelope machinery in this cut — actor identity rides the fact log; cryptographic attestation is a later hardening, not a substrate change |
| Event sourcing (CQRS projections) | State as a fold over an immutable event log; projections rebuildable; snapshots never authority | A global totally-ordered stream — per-actor logs with per-slice ownership need only per-actor order plus causality at claim boundaries |

## The model

Three kinds of thing exist. Nothing else carries workflow state.

- **Artifacts** — immutable once referenced: `change.md`, `plan.yaml` (topology and executable leaves only), per-slice `spec.md` / `design.md` / `model.yaml` / `evidence/`, `base.yaml` pins, build records, closed target-wave manifests under `targets/<target>/waves/<digest>.yaml`, RFC-88's lead catalog and decomposition revisions, and RFC-91 domain-round records. Identified by content digest. Ordinary files.
- **Facts** — append-only events in per-actor logs: authorization epochs, claims, validations, build outcomes, domain convergence, target-wave merges, retractions. The existing closed `EventKind` taxonomy, re-homed and extended.
- **Values** — immutable phase values. RFC-87 later supplies content-addressed snapshot objects and code patches relating base to result; no separate patch blob exists. Artifacts and facts reference those identities once available.

The change tree:

```text
<change>/                        # `.emery/change/` in-place, an ordinary directory detached (RFC-88)
  change.md
  plan.yaml                      # topology + executable leaf projection — no status field
  decomposition.yaml             # RFC-88 conflict-domain hierarchy; no lifecycle state
  events/<actor>.jsonl           # per-actor append-only logs; union-merged, never conflicting
  slices/<slice>/
    base.yaml                    # pinned inputs: baseline spec revision, per-source revisions
    evidence/<source>.yaml
    model.yaml
    spec.md                      # delta spec with slice-scoped requirement ids
    design.md
    tasks.md
    builds/<digest>.yaml         # build record: base/result values + touched paths + report digest
```

The baseline (`.emery/specs/`) remains the durable system of record in the product repository, exactly as today — durable project state beside the change home, never inside it. Changes merge into it and archive away. [RFC-88](rfc-88-detached-changes.md) carries that state inside each project's snapshot, so the fold can run without an ambient checkout.

Status is one deterministic fold over the union of artifacts and facts:

```text
authored   ⇐ plan.yaml has slices
refined(s) ⇐ validated artifacts exist for s, pinned by base.yaml, with a validation fact
built(s)   ⇐ a build record references s's current spec digest and its declared base/result values exist
merged(s)  ⇐ a target-wave committed fact names s and records its finalized delta and identity map
dropped(s) ⇐ a drop fact
```

Privileged work (build, merge) may proceed only under a `plan.execute.started` authorization epoch. The first implementation accepts only `closed-plan` coverage over one reviewed plan digest. Its sorted per-leaf spec coverage is either `existing { digest }` or `refine-under-epoch`: the latter authorizes only the spec produced by that leaf's refinement fact under this epoch, preserving today's refine→build loop without pretending an unknown future digest was reviewed. A changed or externally replaced spec requires a new epoch. A claim or projected `in-progress` status never implies authorization: claims are worker-owned, recoverable liveness records and may also cover pre-execution refinement.

Every `slice.build.started` and merge-wave member binds its authorization epoch plus the exact leaf, spec, dependency frontier, and pinned base it consumed. RFC-88 extends that fence with lead-catalog, decomposition-revision, model-capability-profile, and target-CID digests. Any changed input makes the result stale independently of whether the epoch remains open. This separation leaves a future `streaming-discovery` coverage variant free to authorize refinement and build of ready leaves from one immutable discovery scope while survey and decomposition continue. Streaming coverage cannot commit a target wave: accepted-CID mutation still requires a later `closed-plan` epoch over the reviewed projection and exact built spec digests. There is no `approved` rung.

`plan status` already projects next actions from entries, metadata, and the journal tail; this completes that move — the projection becomes the *only* status surface, and two actors' change trees combine by fact union with no mutable status to reconcile.

Every phase becomes a pure function over pinned values:

```text
survey     : (source revision id)                      → leads
decompose  : (lead catalog, topology, model profiles)  → domains + leaf plan
extract    : (source revision id, lead)                → evidence
synthesize : (evidence digests, baseline revision)     → spec delta + model
build      : (spec digest, base tree value)            → result value + touched paths + report
merge      : (base/result values, spec delta, baseline) → new baseline + result values + identity map
```

Each is relocatable to any node that can prepare its inputs, and auditable by anyone who can read the change tree. This is the planning layer speaking the same value language RFC-87 gives the tree layer.

## Decisions

| # | Decision | Consequence |
| - | -------- | ----------- |
| D1 | **The change is a self-contained fact tree with no version-control requirement.** In-place mode it is `.emery/change/` in the product repository, sitting beside but never containing durable project state (`project.yaml`, `specs/`, `decisions/`); detached mode ([RFC-88](rfc-88-detached-changes.md)) it is an ordinary directory. The change-tree contract requires no Git metadata or other VCS backing; a containing or participating project has its separate repository contract. | Extended planning survives because artifacts and facts are portable files with digest-defined identity. Versioning, backup, review, and transport are deployment or operator policy rather than workflow prerequisites. |
| D2 | **Status is projected, never stored.** `plan.yaml` carries no per-entry `status`; `metadata.yaml` carries no lifecycle field. The `LifecycleStatus` and plan `Status` ladders survive as projection *outputs* with their exact current semantics and writer discipline expressed as fact-emission rules. `plan undo` appends a retraction fact; projections honor retractions; no fact is ever rewritten. | The single hardest distributed-state problem — synchronizing mutable ladders — is deleted rather than solved. Re-entry, resumption, and `plan status` read one code path. The lifecycle vocabulary (`refining → refined → built → merged`, `pending → in-progress → done`) is unchanged for operators. |
| D3 | **Events live in per-actor append-only logs.** `events/<actor>.jsonl` replaces the single `journal.jsonl`; an actor id is a stable workflow identity (operator or node), captured once per composition root like `Locations`; each event carries actor and per-actor sequence. `emery journal show` projects the received union. Lossless tree exchange unions actor-owned files without concurrent append conflicts. | The journal keeps its closed `EventKind` taxonomy and its role as the audit trail. What changes is only the storage shape — from the worst concurrent-append structure to one that any coordination transport can union without rewriting another actor's log. |
| D4 | **Every judgment leg runs against pinned, recorded inputs.** Per-source revisions close at plan authoring (in-place) or discovery (detached) — when the source set is closed — and are recorded at plan scope first. Refine writes each slice's `base.yaml` by copying those source pins and adding the baseline spec revision before the first extract; build records pin the spec digest and base tree revision; evidence is valid only against its recorded source revision. Staleness is *detected* (typed diagnostics at validate and merge), never silently absorbed. | Refinement against the committed baseline is reproducible and safe over time — the precondition for shift-left planning, multi-operator refinement, and RFC-91's concurrent synthesis. A reviewed spec provably describes what it was synthesized from. Pins close when their inputs are knowable; `base.yaml` is the refine-time assembly, not the first writer of source ids. |
| D5 | **Requirement identity is slice-scoped at synthesis and finalized at merge.** Synthesis mints ids in the slice's namespace; the delta spec carries them; each `MODIFIED` entry records the digest of the baseline requirement body it modified. Merge assigns final baseline `REQ-NNN` numbers, records the slice-id → baseline-id map as a merge fact, and rejects a drifted `MODIFIED` base digest with a typed conflict instead of last-writer-wins. | Two slices refined against the same baseline can never collide on identity — the defect that today forces serial, interleaved refinement. Provenance survives renumbering through the recorded map. This is the same reconciliation RFC-91's concurrency needs; it lands once, here. |
| D6 | **`plan execute` opens an explicit authorization epoch.** Before privileged work, `emery plan execute` appends `plan.execute.started`; the event envelope identifies the operator actor and its payload carries a closed `coverage` variant. This RFC implements `closed-plan { plan-digest, specs }`, where each leaf is `existing { digest }` or `refine-under-epoch`; RFC-88 makes the plan transitively bind its lead catalog, decomposition, and model-capability profiles. The event is durable operator authority, not evidence that a worker happened to move. A future `streaming-discovery` variant may cover refinement and build under one immutable discovery scope, but not accepted-CID mutation; target-wave commit still requires closed-plan coverage. There is no plan-approval verb, status, file, or projected `approved` rung. | "Who authorized which bounded scope" is answerable independently of claims. Closed-plan changes require a fresh epoch; normal unrefined leaves remain executable; future streaming build can admit later leaf revisions without pretending their first claim was operator approval. |
| D7 | **Work is claimed by fact, not by lock.** Claiming a slice (or the plan-author / merge role) is an event; the projection treats a slice with a live claim as owned; conflicting concurrent claims are a *detected violation* surfaced by the projection, not a merge to resolve. A claim references its authorization epoch when it covers privileged work but never creates that authority. Internal conflict domains carry no claim or lifecycle status: the plan-author claim covers recursive decomposition, and execution claims only its terminal slice projection. RFC-87 workspaces are private by construction and carry no workflow lock. | Coordination is visible in the fact union with no status authority. Claims coordinate actors across machines once a coordination transport exchanges them; RFC-92 supplies that transport and hardens claims with fencing without changing their shape. |
| D8 | **Phases are pure functions over values.** Every phase's inputs and outputs are digest-identified (the table above); orchestrations record the binding as facts. Build bindings include build authorization, pinned base identity, leaf, dependency frontier, and spec; commit adds its closed-plan authorization. RFC-88 later adds lead, decomposition, model-capability-profile, and base-CID bindings, while RFC-91 domain convergence binds complete child results. No phase reads ambient directory state as an implicit input. | Any node that can prepare the inputs can run the work — the Bazel property, without pretending judgment is cacheable. Failure loses nothing: inputs and every completed output persist; retry is re-dispatch. |
| D9 | **Merge transitions use one immutable target wave.** Before build, the engine writes `targets/<target>/waves/<digest>.yaml` naming the target (the current project in the in-place cut), pinned base, ordered member set, exact member inputs, dependency frontier, and build-authorization epoch, then appends `target.wave.opened`. This RFC admits one member. Merge revalidates it, names a `closed-plan` commit-authorization epoch (which may differ from build authorization), performs the existing deterministic merge, and appends one `target.merge.wave-committed` fact carrying every identity map. That fact projects all members merged. Postflight then appends succeeded or postflight-failed; failure is non-rollback and uses the existing acknowledgement stop. RFC-88 later uses RFC-87 snapshot values to add base/result CIDs and make the same fact the accepted-CID transition; RFC-91 admits bounded independent membership. | RFC-86 is deployable before snapshots exist and establishes the stable fact and manifest shape. No fact prefix can merge only some members, worker ownership cannot authorize a merge, and future streaming builds can bind a build-only epoch while commit waits for later closed-plan review. |
| D10 | **The desktop is the degenerate deployment, not a mode.** One actor, one node, no remote: the same fact tree, projections, claims, and pins, with coordination and value transports simply absent. No behavior, verb, or artifact differs between single-node and multi-node operation except the transports configured at the composition root. | There is exactly one lifecycle model to implement, test, and document. The Omnia/Wasmtime investment pays off as intended: the guest is location-neutral, providers carry deployment, and now state does too. RFC-92 shrinks to transport. |
| D11 | **Hard cut, no shims.** Stored status fields, the single journal file, synthesis-time global id allocation, and unrecorded execute starts are removed, not aliased. Pre-1.0 posture applies: re-init over migration. | One model in the codebase. The projection is the only status reader; the fact logs are the only status writers. |

## Surface

This RFC adds no workflow subcommand. `plan author` and `plan archive` retain their roles; the existing execution and breakout surfaces change only as follows:

- `emery plan execute` — appends `plan.execute.started` with `closed-plan` coverage over the current plan and per-leaf existing/refine spec coverage before privileged work; RFC-88's plan digest later includes its lead-catalog and decomposition digests. The drained-loop behavior is otherwise unchanged.
- `emery plan advance` — becomes the claim-writing verb (its `in-progress` semantic, re-expressed); `emery plan undo` appends retraction facts walking the same rungs.
- `emery plan status` / `emery journal show` — unchanged operator surface, now reading the projection over the fact union.
- `emery slice refine` — writes `base.yaml` before extraction (plan-scope source snapshot pins plus baseline digest); refuses when the pinned baseline revision cannot be resolved.
- `emery slice validate` — gains staleness checks: `slice-base-drifted` (baseline moved since `base.yaml`), `slice-evidence-stale` (source snapshot superseded) — review signals until merge, where drift on a `MODIFIED` target becomes blocking.
- `emery slice merge` — invokes D9's one-member target wave; target-wave commit assigns final requirement numbers, records every member identity map in its atomic fact, and rejects `merge-base-drifted` on a stale `MODIFIED` digest. Postflight runs after commit; failure records the existing non-rollback stop condition while the merge remains accepted.
- New closed diagnostics: `slice-claim-conflict`, `slice-base-drifted`, `slice-evidence-stale`, `merge-base-drifted` (exit 2).
- New `EventKind` variants: `plan.execute.started`, `slice.claimed`, `slice.claim-released`, `fact.retracted`, `target.wave.opened`, `target.merge.wave-committed`, `target.merge.wave-succeeded`, and `target.merge.wave-postflight-failed` — extending the closed taxonomy. RFC-91 later adds `domain.convergence.recorded`. The committed fact carries every member's finalized identity map, so a separate slice identity-mapped event is unnecessary; succeeded or postflight-failed records the post-commit gate outcome.

## Fixed implementation cut

- The projection is one pure kernel in `crates/project` (facts + artifact index in, status out), property-tested for determinism: same fact set, any interleaving of per-actor files, same projection.
- Actor identity is a stable string captured once at the composition root (config or environment via the launcher, generated and persisted on first use); kernels never read `std::env`.
- Per-actor event files append with the same atomic-write discipline as today's journal; an actor writes only its own file.
- Plan authoring (in-place) or discovery (detached) records per-source revision ids at plan scope. `base.yaml` is written by refine before the first extract: it copies those source pins and adds the baseline revision. Build records use the available base/result value identity; RFC-87 later makes those identities content-addressed snapshots, with no code-patch bytes in the change-files tree.
- Slice-scoped requirement ids use the existing grammar under a slice namespace; the merge-recorded identity map is the only translation surface, and `emery slice provenance` resolves through it.
- The `MODIFIED` base digest is SHA-256 over the parsed baseline requirement block, computed by the same parser the merge engine uses.
- `plan.execute.started` has a closed `coverage` payload. This cut supports `closed-plan { plan-digest, specs }` plus optional `discovery-digest` (absent in-place and required for detached changes by RFC-88). Each sorted `specs` entry is `existing { digest }` or `refine-under-epoch`. The plan transitively covers its lead-catalog and decomposition digests; the event does not duplicate them. The common event envelope supplies actor and per-actor sequence and is the stable epoch anchor referenced by later privileged facts. Reserve build-only `streaming-discovery` as a future coverage variant rather than overloading a claim. There is no `approvals/` directory and no plan-approval vocabulary in status or CLI.
- A target-wave manifest has one `build-authorization` epoch anchor. `target.merge.wave-committed` separately carries `commit-authorization`, which must identify a live `closed-plan` epoch covering the exact current leaf and spec digests; current serial execution normally uses the same epoch for both fields. A future streaming-built result can therefore survive review without gaining merge authority.
- `crates/mock` and the eval cases grow multi-actor fixtures: two actors, disjoint slices, unioned change trees, claim-conflict and base-drift injections.

A detached execute start is one line in `events/<actor>.jsonl`:

```json
{"timestamp":"2026-08-05T04:30:00Z","actor":"operator-a","sequence":7,"event":"plan.execute.started","payload":{"coverage":{"kind":"closed-plan","plan-digest":"sha256:…","specs":{"orders-api":{"kind":"existing","digest":"sha256:…"},"orders-ui":{"kind":"refine-under-epoch"}}},"discovery-digest":"sha256:…"}}
```

## Rejected alternatives

- **A coordination service or hosted database as status authority** — recreates the single-writer problem as an availability problem, and breaks D10: the desktop would need a server or a mode split.
- **Keeping stored status and synchronizing it** (RFC-92's original journal-authority cutover) — synchronizing mutable state is strictly harder than projecting immutable facts, and the cutover created two lifecycle models with a one-way door between them.
- **CRDT merge of concurrent edits to one slice** (full git-bug/Radicle machinery) — solves a conflict Emery's exclusive claims are designed to prevent; detection plus refusal is simpler and matches RFC-91's ownership posture.
- **Global requirement numbering via a coordination step at synthesis time** — reintroduces cross-slice coupling at the exact point independence matters most; merge is the natural serialization point and already owns the baseline write.
- **A single journal file with transport-specific merge logic** — bespoke reconciliation in every transport, versus a storage shape that unions actor-owned logs natively.
- **A separate authorization artifact beside `plan.execute.started`** — duplicates one statement across two authorities without adding information; the event already carries the covered digests and actor.
- **Inferring authorization from the first `in-progress` slice** — confuses a recoverable worker claim with an operator grant, loses the reviewed input scope, and makes hosted workers capable of authorizing themselves. Per-leaf digest fields would recreate the input fence but still not the operator authority.
- **Naming the fact `plan.approved` or projecting an `approved` plan status** — reintroduces plan-approval vocabulary for an orchestration start event; execute remains the only operator surface, and coverage is a gate over facts, not a lifecycle rung.
- **Signed attestations in this cut** — actor identity in the fact log is sufficient for the trust domain (an operator's own repos and nodes); DSSE-style envelopes are a compatible later hardening.

## Phased delivery

- **Phase A — Facts and projections.** Per-actor event logs, the projection kernel, removal of stored `status` / lifecycle fields, `plan advance` / `undo` as claim / retraction facts, `journal show` over the union. Single-actor behavior is observably unchanged (`plan status` output parity is the gate).
- **Phase B — Pins, identity, and serial waves.** Plan-scope source snapshot pins from authoring or discovery assembled into `base.yaml` at refine with the baseline digest, staleness diagnostics in validate, slice-scoped requirement ids, `MODIFIED` base digests, one-member target-wave manifests, merge-time number finalization with the atomic committed identity map, provenance through the map, and commit/postflight crash recovery.
- **Phase C — Execute start and multi-actor.** `plan.execute.started` facts emitted by `plan execute`, claim conflicts as typed violations, multi-actor change-tree union coverage, shift-left flow proven end to end: author → refine all slices → review the pinned spec set → execute (build + merge only).

## Acceptance criteria

1. `plan.yaml` and `metadata.yaml` carry no status fields; every status the CLI reports is the projection kernel's output, and the projection is deterministic under any interleaving of per-actor event files.
2. Two actors refine disjoint slices in copies of one change tree; a lossless union of the two trees is conflict-free; the combined projection reports both slices `refined`; a conflicting claim injected on one slice surfaces `slice-claim-conflict`.
3. Two slices refined against the same pinned baseline, each adding requirements to the same domain, merge serially without identity collision; the recorded identity maps resolve every provenance reference; a `MODIFIED` entry whose baseline target drifted fails `merge-base-drifted` instead of overwriting.
4. A plan refined fully before execution runs as a build-and-merge-only loop; an unrefined leaf covered by `refine-under-epoch` may refine and build under that epoch; a build failure leaves every artifact and fact intact and re-entry resumes from the projection. A changed lead catalog, decomposition, plan, existing spec, or refinement output requires a fresh closed-plan epoch before affected privileged work continues.
5. Evidence and specs record their pinned inputs; moving the baseline or a source tree after pinning surfaces the typed staleness diagnostics at validate and blocks at merge only where a `MODIFIED` target drifted.
6. The complete flow runs identically in one ordinary directory (desktop) and with the change tree losslessly exchanged between two ordinary directories (two-actor); neither directory needs Git metadata, and no verb, artifact, or diagnostic differs.
7. Serial execution opens one immutable one-member target wave before build. Merge names both build and closed-plan commit authorization, projects the member merged with one fact, and survives postflight failure without rollback; failures before that fact leave no merged projection. The test requires no RFC-87 snapshot provider.
8. `cargo make ci` is green; projection determinism, fact-union merge, claim conflicts, identity finalization, staleness, target-wave crash boundaries, execute-start coverage, and retraction walk-back are covered as crate-level integration tests over local fixtures.
