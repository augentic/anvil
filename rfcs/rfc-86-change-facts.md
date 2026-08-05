# RFC-86: Change Facts

> Status: Draft — step 1 of the platform-migration series ([platform.md](platform.md)); the substrate every later step stands on
>
> Owns: the fact-based change substrate — the change as a self-contained tree of immutable, digest-identified artifacts and append-only per-actor event logs; workflow status as a projection over facts, never a stored field; pinned bases for every judgment leg; slice-scoped requirement identity finalized at merge; approval as a recorded artifact; and the single-actor desktop as the degenerate case of the same substrate, not a separate mode.
>
> Depends on: nothing in the series. Consumed by every later step: [RFC-87](rfc-87-working-trees.md) (snapshot and result identities join the fact vocabulary), [RFC-88](rfc-88-detached-changes.md) (the detached change home becomes a fact tree), [RFC-89](rfc-89-publication-sets.md) (approval and publication verification read facts), [RFC-91](rfc-91-concurrent-execution.md) (concurrent synthesis requires merge-finalized identity), [RFC-92](rfc-92-node-sync.md) (node sync becomes fact-and-value transport, with no authority cutover).

## Intent

Before distributing the work, fix the base. Today's structure is correct about its units — the slice as the unit of isolation, the delta spec as the unit of intent, the plan as the umbrella — and wrong about exactly four mechanics, each of which works only because *one operator, one session, one tree* is currently true:

| Today | Where | Why it cannot distribute |
| ----- | ----- | ------------------------ |
| Workflow status is stored as mutable fields | `plan.yaml` per-entry `status` ladder (`crates/project/src/plan/model/state.rs`), `metadata.yaml` lifecycle (`crates/project/src/slice/lifecycle.rs`) | Two actors' trees cannot merge; every stored ladder is a synchronization problem RFC-92 would otherwise have to solve |
| The journal is one append-only file | `.emery/journal.jsonl` (`crates/project/src/journal.rs`) | Single files conflict under any concurrent append or tree union; the worst possible shape for a distributed log |
| Requirement identity is allocated at synthesis, relative to the baseline seen then | `IdAllocator` (`crates/slice/src/synthesis/project.rs`) | Two slices refined against the same baseline deterministically mint colliding `REQ-NNN` ids; `MODIFIED` is silent last-writer-wins |
| Approval is an action, not an artifact | "running `emery plan execute` is the approval (nothing is stamped or recorded)" | An action on one desktop cannot travel, be audited, or be verified by a second actor or node |

This RFC replaces those four mechanics with one substrate: **a change is a self-contained tree of immutable facts; all status is projected, never stored; every judgment output is a persisted, pinned, digest-identified artifact; and approval is itself an artifact.** Everything else — the WIT seam, the adapter contract, the delta-spec grammar, the evidence/model schemas, the validation registries, and the slice loop — stays load-bearing.

The outcome this purchases: **Emery works on one desktop exactly as it works in a multi-node cloud environment.** The engine guest is already location-neutral by construction — it is a Wasm component whose deployment differences live entirely in providers and the launcher. This RFC makes the *state* location-neutral to match: facts and values move through deployment-provided coordination and content-addressed-value transports; a desktop is simply the deployment with one actor and no remote. There is no hosted mode, no attach cutover, and no second lifecycle model.

It also purchases the operator story directly: a plan's specs and designs become a reviewable, portable set of pre-build artifacts. Planning can extend over days and multiple operators; a build failure loses nothing; review happens against pinned, diff-shaped delta specs rather than lead synopses; and a bug fix is the same structure with a one-slice footprint.

## Prior art

Every mechanism in this RFC is a settled pattern elsewhere; the design work is choosing which to adopt and which to reject.

| System | Pattern adopted | Pattern rejected |
| ------ | --------------- | ---------------- |
| [git-bug](https://github.com/git-bug/git-bug/blob/master/doc/design/data-model.md) / [Radicle COBs](https://radicle.dev/guides/protocol) | Mutable state represented as append-only operations; the snapshot is always derived by deterministic replay, never persisted as authority | Git object storage and Lamport-clock CRDT merge of concurrent edits to one entity — Emery's fact tree is version-control-neutral, and its exclusive per-slice claims make cross-actor conflicts a detected violation rather than a merge to resolve |
| [Jujutsu](https://jj-vcs.dev) | The stable-identity / content-identity split: a *change id* survives rewrites while the commit id tracks content — mirrored here as slice-scoped requirement identity vs the baseline number finalized at merge | Treating identity as local-only state; Emery's identity mapping is a recorded merge fact |
| [Bazel Remote Execution](https://github.com/bazelbuild/remote-apis/blob/main/build/bazel/remote/execution/v2/remote_execution.proto) | Work as a content-addressed pure function: an action is the digest of its command plus input tree; any compatible worker can execute it; outputs return by digest | The action *cache* — judgment legs are not deterministic, so Emery records outcomes as facts rather than deduplicating executions |
| [in-toto attestations](https://github.com/in-toto/attestation/blob/main/spec/README.md) / SLSA | Approval and provenance as statements binding a predicate to immutable digest-identified subjects, consumable by policy engines and other actors | Signature/envelope machinery in this cut — actor identity rides the fact log; cryptographic attestation is a later hardening, not a substrate change |
| Event sourcing (CQRS projections) | State as a fold over an immutable event log; projections rebuildable; snapshots never authority | A global totally-ordered stream — per-actor logs with per-slice ownership need only per-actor order plus causality at claim boundaries |

## The model

Three kinds of thing exist. Nothing else carries workflow state.

- **Artifacts** — immutable once referenced: `change.md`, `plan.yaml` (topology only), per-slice `spec.md` / `design.md` / `model.yaml` / `evidence/`, `base.yaml` pins, build records. Identified by content digest. Ordinary files.
- **Facts** — append-only events in per-actor logs: claims, validations, approvals, build outcomes, merges, retractions. The existing closed `EventKind` taxonomy, re-homed and extended.
- **Values** — content-addressed snapshot objects. RFC-87 code patches relate a base snapshot to a result snapshot; no separate patch blob exists. Artifacts and facts reference the snapshot identities.

The change tree:

```text
<change>/                        # `.emery/change/` in-place, an ordinary directory detached (RFC-88)
  change.md
  plan.yaml                      # topology only: slices, sources, projects — no status field
  approvals/<digest>.yaml        # approval facts: plan digest + per-slice spec digests in scope
  events/<actor>.jsonl           # per-actor append-only logs; union-merged, never conflicting
  slices/<slice>/
    base.yaml                    # pinned inputs: baseline spec revision, per-source snapshot revisions
    evidence/<source>.yaml
    model.yaml
    spec.md                      # delta spec with slice-scoped requirement ids
    design.md
    tasks.md
    builds/<digest>.yaml         # build record: base/result snapshots + touched paths + report digest
```

The baseline (`.emery/specs/`) remains the durable system of record in the product repository, exactly as today — durable project state beside the change home, never inside it. Changes merge into it and archive away. [RFC-88](rfc-88-detached-changes.md) carries that state inside each project's snapshot, so the fold can run without an ambient checkout.

Status is one deterministic fold over the union of artifacts and facts:

```text
authored   ⇐ plan.yaml has slices
approved   ⇐ an approval fact covers the current plan digest (and whichever spec digests existed)
refined(s) ⇐ validated artifacts exist for s, pinned by base.yaml, with a validation fact
built(s)   ⇐ a build record references s's current spec digest and base/result snapshots exist
merged(s)  ⇐ a merge fact records s's delta folded into a named baseline revision
dropped(s) ⇐ a drop fact
```

`plan status` already projects next actions from entries, metadata, and the journal tail; this completes that move — the projection becomes the *only* status surface, and two actors' change trees combine by fact union with no mutable status to reconcile.

Every phase becomes a pure function over pinned values:

```text
survey     : (source snapshot id)                      → leads
extract    : (source snapshot id, lead)                → evidence
synthesize : (evidence digests, baseline snapshot)     → spec delta + model
build      : (spec digest, base tree snapshot)         → result snapshot + touched paths + report
merge      : (base/result snapshots, spec delta, baseline) → new baseline + tree snapshots + identity map
```

Each is relocatable to any node that can prepare its inputs, and auditable by anyone who can read the change tree. This is the planning layer speaking the same value language RFC-87 gives the tree layer.

## Decisions

| # | Decision | Consequence |
| - | -------- | ----------- |
| D1 | **The change is a self-contained fact tree with no version-control requirement.** In-place mode it is `.emery/change/` in the product repository, sitting beside but never containing durable project state (`project.yaml`, `specs/`, `decisions/`); detached mode ([RFC-88](rfc-88-detached-changes.md)) it is an ordinary directory. The change-tree contract requires no Git metadata or other VCS backing; a containing or participating project has its separate repository contract. | Extended planning survives because artifacts and facts are portable files with digest-defined identity. Versioning, backup, review, and transport are deployment or operator policy rather than workflow prerequisites. |
| D2 | **Status is projected, never stored.** `plan.yaml` carries no per-entry `status`; `metadata.yaml` carries no lifecycle field. The `LifecycleStatus` and plan `Status` ladders survive as projection *outputs* with their exact current semantics and writer discipline expressed as fact-emission rules. `plan undo` appends a retraction fact; projections honor retractions; no fact is ever rewritten. | The single hardest distributed-state problem — synchronizing mutable ladders — is deleted rather than solved. Re-entry, resumption, and `plan status` read one code path. The lifecycle vocabulary (`refining → refined → built → merged`, `pending → in-progress → done`) is unchanged for operators. |
| D3 | **Events live in per-actor append-only logs.** `events/<actor>.jsonl` replaces the single `journal.jsonl`; an actor id is a stable workflow identity (operator or node), captured once per composition root like `Locations`; each event carries actor and per-actor sequence. `emery journal show` projects the received union. Lossless tree exchange unions actor-owned files without concurrent append conflicts. | The journal keeps its closed `EventKind` taxonomy and its role as the audit trail. What changes is only the storage shape — from the worst concurrent-append structure to one that any coordination transport can union without rewriting another actor's log. |
| D4 | **Every judgment leg runs against pinned, recorded inputs.** Per-source snapshot revisions close at plan authoring (in-place) or discovery (detached) — when the source set is closed — and are recorded at plan scope first. Refine writes each slice's `base.yaml` by copying those source pins and adding the baseline spec revision before the first extract; build records pin the spec digest and base tree revision; evidence is valid only against its recorded snapshot. Staleness is *detected* (typed diagnostics at validate and merge), never silently absorbed. | Refinement against the committed baseline is reproducible and safe over time — the precondition for shift-left planning, multi-operator refinement, and RFC-91's concurrent synthesis. A reviewed spec provably describes what it was synthesized from. Pins close when their inputs are knowable; `base.yaml` is the refine-time assembly, not the first writer of source snapshot ids. |
| D5 | **Requirement identity is slice-scoped at synthesis and finalized at merge.** Synthesis mints ids in the slice's namespace; the delta spec carries them; each `MODIFIED` entry records the digest of the baseline requirement body it modified. Merge assigns final baseline `REQ-NNN` numbers, records the slice-id → baseline-id map as a merge fact, and rejects a drifted `MODIFIED` base digest with a typed conflict instead of last-writer-wins. | Two slices refined against the same baseline can never collide on identity — the defect that today forces serial, interleaved refinement. Provenance survives renumbering through the recorded map. This is the same reconciliation RFC-91's concurrency needs; it lands once, here. |
| D6 | **Execution records approval as an artifact.** `emery plan execute` records an approval fact naming the current plan digest, the per-slice spec digests then in scope, and the invoking actor before it proceeds. Execution after shift-left refinement covers specs; execution before refinement covers topology only. There is no standalone approval verb. | "What exactly authorized execution, by whom, against which artifacts" becomes answerable — by archive verification, a second operator, or an audit months later — without adding an operator step to the default loop. |
| D7 | **Work is claimed by fact, not by lock.** Claiming a slice (or the plan-author / merge role) is an event; the projection treats a slice with a live claim as owned; conflicting concurrent claims are a *detected violation* surfaced by the projection, not a merge to resolve. RFC-87 workspaces are private by construction and carry no workflow lock. | Coordination is visible in the fact union with no status authority. Claims coordinate actors across machines once a coordination transport exchanges them; RFC-92 supplies that transport and hardens claims with fencing without changing their shape. |
| D8 | **Phases are pure functions over values.** Every phase's inputs and outputs are digest-identified (the table above); orchestrations record the binding as facts. No phase reads ambient directory state as an implicit input. | Any node that can prepare the inputs can run the work — the Bazel property, without pretending judgment is cacheable. Failure loses nothing: inputs and every completed output persist; retry is re-dispatch. |
| D9 | **The desktop is the degenerate deployment, not a mode.** One actor, one node, no remote: the same fact tree, projections, claims, and pins, with coordination and value transports simply absent. No behavior, verb, or artifact differs between single-node and multi-node operation except the transports configured at the composition root. | There is exactly one lifecycle model to implement, test, and document. The Omnia/Wasmtime investment pays off as intended: the guest is location-neutral, providers carry deployment, and now state does too. RFC-92 shrinks to transport. |
| D10 | **Hard cut, no shims.** Stored status fields, the single journal file, synthesis-time global id allocation, and unrecorded approval are removed, not aliased. Pre-1.0 posture applies: re-init over migration. | One model in the codebase. The projection is the only status reader; the fact logs are the only status writers. |

## Surface

This RFC adds no workflow subcommand. `plan author` and `plan archive` retain their roles; the existing execution and breakout surfaces change only as follows:

- `emery plan execute` — appends an approval fact over the current plan and spec digests before running; otherwise unchanged as the drained loop.
- `emery plan advance` — becomes the claim-writing verb (its `in-progress` semantic, re-expressed); `emery plan undo` appends retraction facts walking the same rungs.
- `emery plan status` / `emery journal show` — unchanged operator surface, now reading the projection over the fact union.
- `emery slice refine` — writes `base.yaml` before extraction (plan-scope source snapshot pins plus baseline digest); refuses when the pinned baseline revision cannot be resolved.
- `emery slice validate` — gains staleness checks: `slice-base-drifted` (baseline moved since `base.yaml`), `slice-evidence-stale` (source snapshot superseded) — review signals until merge, where drift on a `MODIFIED` target becomes blocking.
- `emery slice merge` — assigns final requirement numbers, records the identity map in the merge fact, and rejects `merge-base-drifted` on a stale `MODIFIED` digest.
- New closed diagnostics: `slice-claim-conflict`, `slice-base-drifted`, `slice-evidence-stale`, `merge-base-drifted` (exit 2).
- New `EventKind` variants: `plan.approved`, `slice.claimed`, `slice.claim-released`, `fact.retracted`, `slice.merge.identity-mapped` — extending the closed taxonomy.

## Fixed implementation cut

- The projection is one pure kernel in `crates/project` (facts + artifact index in, status out), property-tested for determinism: same fact set, any interleaving of per-actor files, same projection.
- Actor identity is a stable string captured once at the composition root (config or environment via the launcher, generated and persisted on first use); kernels never read `std::env`.
- Per-actor event files append with the same atomic-write discipline as today's journal; an actor writes only its own file.
- Plan authoring (in-place) or discovery (detached) records per-source snapshot ids at plan scope. `base.yaml` is written by refine before the first extract: it copies those source pins and adds the baseline snapshot. Build records reference RFC-87 base/result snapshot identities; no code-patch bytes live in the change-files tree.
- Slice-scoped requirement ids use the existing grammar under a slice namespace; the merge-recorded identity map is the only translation surface, and `emery slice provenance` resolves through it.
- The `MODIFIED` base digest is SHA-256 over the parsed baseline requirement block, computed by the same parser the merge engine uses.
- Approval facts live as both an event and a content-addressed file under `approvals/` — the file is the reviewable artifact, the event is the ordered record; they carry the same digest.
- `crates/mock` and the eval cases grow multi-actor fixtures: two actors, disjoint slices, unioned change trees, claim-conflict and base-drift injections.

## Rejected alternatives

- **A coordination service or hosted database as status authority** — recreates the single-writer problem as an availability problem, and breaks D9: the desktop would need a server or a mode split.
- **Keeping stored status and synchronizing it** (RFC-92's original journal-authority cutover) — synchronizing mutable state is strictly harder than projecting immutable facts, and the cutover created two lifecycle models with a one-way door between them.
- **CRDT merge of concurrent edits to one slice** (full git-bug/Radicle machinery) — solves a conflict Emery's exclusive claims are designed to prevent; detection plus refusal is simpler and matches RFC-91's ownership posture.
- **Global requirement numbering via a coordination step at synthesis time** — reintroduces cross-slice coupling at the exact point independence matters most; merge is the natural serialization point and already owns the baseline write.
- **A single journal file with transport-specific merge logic** — bespoke reconciliation in every transport, versus a storage shape that unions actor-owned logs natively.
- **Signed attestations in this cut** — actor identity in the fact log is sufficient for the trust domain (an operator's own repos and nodes); DSSE-style envelopes are a compatible later hardening.

## Phased delivery

- **Phase A — Facts and projections.** Per-actor event logs, the projection kernel, removal of stored `status` / lifecycle fields, `plan advance` / `undo` as claim / retraction facts, `journal show` over the union. Single-actor behavior is observably unchanged (`plan status` output parity is the gate).
- **Phase B — Pins and identity.** Plan-scope source snapshot pins from authoring or discovery assembled into `base.yaml` at refine with the baseline digest, staleness diagnostics in validate, slice-scoped requirement ids, `MODIFIED` base digests, merge-time number finalization with the recorded identity map, provenance through the map.
- **Phase C — Execution authorization and multi-actor.** Approval facts emitted by `plan execute`, claim conflicts as typed violations, multi-actor change-tree union coverage, shift-left flow proven end to end: author → refine all slices → review the pinned spec set → execute (build + merge only).

## Acceptance criteria

1. `plan.yaml` and `metadata.yaml` carry no status fields; every status the CLI reports is the projection kernel's output, and the projection is deterministic under any interleaving of per-actor event files.
2. Two actors refine disjoint slices in copies of one change tree; a lossless union of the two trees is conflict-free; the combined projection reports both slices `refined`; a conflicting claim injected on one slice surfaces `slice-claim-conflict`.
3. Two slices refined against the same pinned baseline, each adding requirements to the same domain, merge serially without identity collision; the recorded identity maps resolve every provenance reference; a `MODIFIED` entry whose baseline target drifted fails `merge-base-drifted` instead of overwriting.
4. A plan refined fully before execution runs as a build-and-merge-only loop; a build failure leaves every artifact and fact intact and re-entry resumes from the projection; `plan execute` records topology-scope or spec-scope approval according to the artifacts then present, and a changed plan or spec requires a fresh execution fact.
5. Evidence and specs record their pinned inputs; moving the baseline or a source tree after pinning surfaces the typed staleness diagnostics at validate and blocks at merge only where a `MODIFIED` target drifted.
6. The complete flow runs identically in one ordinary directory (desktop) and with the change tree losslessly exchanged between two ordinary directories (two-actor); neither directory needs Git metadata, and no verb, artifact, or diagnostic differs.
7. `cargo make ci` is green; projection determinism, fact-union merge, claim conflicts, identity finalization, staleness, approval scope, and retraction walk-back are covered as crate-level integration tests over local fixtures.
