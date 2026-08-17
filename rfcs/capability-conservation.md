# Capability Conservation Ledger

> Status: **Draft for the remediation decision gate.** This ledger preserves product capabilities, not current crates, files, commands, event kinds, or implementation mechanisms. A capability may move into the transactional store, a different artifact, or a different internal stage while remaining conserved.
>
> Authority: [product.md](product.md) defines the product, accepted records under [decisions/](decisions/) define policy, and [target-architecture.md](target-architecture.md) defines the destination. This ledger is the remediation traceability contract between the implemented RFC generation and that destination; it never overrides those authorities.
>
> Retirement: after remediation, reduce this to a completed traceability record or delete it once every conserved capability is represented in the target architecture and CI.

Legacy implementation may be deleted only when:

1. the replacement passes the capability's acceptance evidence; or
2. an accepted ADR explicitly deletes the capability and names the consequence.

A green walking skeleton is not, by itself, parity for capabilities deferred to later remediation phases.

**Closure rule:** a capability absent from both the entries below and the intentionally-not-conserved list is **not conserved**. Silence never conserves; conserving an unlisted capability means adding an entry here first.

## Classification

- **Preserve** — required in the replacement before the legacy implementation is deleted.
- **Replace** — the current mechanism is deleted, but its observable guarantee remains.
- **Deferred mandatory** — temporarily absent from the walking skeleton, but required before remediation completes.
- **Intentionally deleted** — the capability itself is removed by an accepted ADR.

An entry whose shape depends on a **proposed** ADR carries a `Conditional on` line. If that ADR resolves differently, the entry is re-shaped before any deletion relies on it.

## CC-01 — Complete source-to-evidence fidelity

**Classification:** Preserve  
**Origin:** RFC-88, RFC-104, adapter contract

Emery can survey declared sources, identify stable leads, extract focused Evidence, and retain every structured claim field required by synthesis. Source adapters remain value-in and cannot acquire lifecycle authority or inspect change state.

The replacement may change the claim DTOs and persistence format. It must not:

- silently discard claim extras such as statements, criteria, or replay digests;
- treat an inaccessible or failed source as successful empty evidence;
- let a source adapter read plan or lifecycle authority;
- lose the identity of the exact source value or CID observed.

**Acceptance evidence:**

- Structured claim extras survive the component seam and appear in persisted Evidence.
- Unreadable or malformed source output fails closed.
- An extracted claim is traceable to its source identity and lead.
- Large replay material remains available through content-addressed attachments rather than ephemeral paths.

## CC-02 — Deterministic survey-to-slice decomposition

**Classification:** Preserve  
**Origin:** RFC-88 D3, RFC-96 scheduling

Survey leads are not assumed to be buildable slices. Emery has one deterministic topology compiler that can:

- focus a broad lead into stable child leads;
- recursively partition work into coherent conflict domains;
- preserve every contributing lead at least once;
- retain cross-cutting leads wherever they inform behavior;
- bind each terminal slice to exactly one target;
- distinguish containment from execution dependencies;
- reject ambiguous ownership, non-reducing splits, cycles, and over-budget recursion;
- project byte-stable slices and dependency edges from equivalent inputs.

The implementation need not preserve `decomposition.yaml`, domain files, or the current authoring state machine. Equivalent transactional rows and a review projection are acceptable.

`spec` may perform this in one operator gesture, but topology authority remains distinct from specification synthesis. Correcting topology invalidates affected specifications.

**Acceptance evidence:**

- A broad lead is focused and decomposed into multiple stable slices.
- A multi-target case reaches at least three domain levels without losing lead coverage.
- Identical inputs produce identical topology and ordering.
- An invalid split publishes no partial topology.
- Changing one domain invalidates only its affected specification closure.
- N=1 follows the same compiler as N>1 without unnecessary ceremony.

## CC-03 — Reviewable and correctable topology

**Classification:** Preserve  
**Origin:** RFC-88 authoring and amendment semantics

The resulting slice topology is inspectable before product mutation. Model-proposed changes remain inert until accepted by deterministic engine policy or an explicit operator act.

A model cannot directly:

- add build authority;
- change a target assignment;
- broaden source scope;
- mutate an accepted slice;
- publish a partial recursive decomposition.

The four-verb surface need not expose a separate topology command. `status`, the review document, and `fix` may provide the review and correction gestures.

Abandoning a slice without merging remains an explicit operator act. Abandonment is generation-scoped scope authority recorded before any artifact movement; archival cleanup is never the scope transition itself (the S7 lesson).

**Acceptance evidence:**

- A proposed target reassignment or boundary change is visible before build.
- Correction applies to the exact topology generation it names.
- Stale corrections cannot affect a later generation.
- Accepted slices are not silently rewritten while sibling topology changes.
- An abandoned slice durably leaves scope and the ready set even when archival cleanup fails or is interrupted.

## CC-04 — Specification-stage firewall

**Classification:** Preserve  
**Origin:** RFC-91

Specification mining and product building remain separate authorization stages.

`spec` may survey, extract, decompose, synthesize, validate, and publish review documents. It must not:

- open a target wave;
- create a product workspace;
- dispatch a target build operation;
- mutate an accepted product baseline.

`build` must not:

- survey or extract sources;
- re-decompose topology;
- synthesize a replacement specification;
- build from a missing, stale, or uncovered specification receipt.

The artifact currently called `refinement.yaml` may be replaced. Its capability survives as one exact, verifiable receipt covering every input and output consumed by build.

**Acceptance evidence:**

- `build` refuses a missing or stale specification receipt before creating a workspace.
- Source, guidance, dependency, baseline, or specification changes stale the affected receipt.
- Unrelated changes do not stale an independent slice.
- A failed synthesis cannot project a slice as specified.
- Re-running `spec` skips fresh independent work and repairs only the stale closure.

## CC-05 — One reviewable specification per slice

**Classification:** Preserve and strengthen  
**Origin:** Existing specification artifacts plus remediation P8

Each slice has one canonical, diff-friendly review document through which a human or reviewing agent can answer whether the slice is specified correctly.

It contains or directly exposes:

- behavioral requirements;
- acceptance criteria;
- inline unknowns, conflicts, and divergences;
- a provenance summary;
- target-independent behavior;
- the effect of any correction guidance.

Structured models, tasks, and internal receipts may exist but are subordinate to this review surface.

**Acceptance evidence:**

- A reviewer can approve or reject a slice without opening a second artifact.
- Re-mining produces a meaningful diff.
- Every conflict, divergence, and unknown appears inline.
- Full provenance remains one gesture away.
- Review time is measured by the eval suite.

## CC-06 — Honest conflict, divergence, and debt conservation

**Classification:** Replace  
**Origin:** RFC-86a, RFC-88 authority resolution, ADR-0004  
**Conditional on:** ADR-0004 (proposed) — the disposition policy below assumes its acceptance

When sources disagree, a closed authority precedence (`intent > documentation > behaviour`, with explicit per-slice overrides) resolves what it can. An authority-resolved disagreement is recorded as a divergence, never silently won; only unresolvable disagreement escalates to a conflict.

Unknown requirements may leave build scope as typed debt under the accepted policy. Conflicting requirements require explicit disposition before build.

Deferred requirements:

- do not become implicit build obligations;
- are not silently invented by the target;
- remain attached to the accepted baseline as typed debt;
- carry the requirement identity and reason;
- lapse or reopen when their covered generation changes.

The current `gap.deferred` fact format and prose-note parsing are not preserved.

**Acceptance evidence:**

- A conflict blocks build until explicitly dispositioned.
- An unknown may be deferred under the selected policy.
- An authority-resolved disagreement remains reviewable as a divergence rather than disappearing into the winning claim.
- An authority override that names no contributing source is rejected.
- Deferred requirements are absent from build obligations.
- Accepted baselines retain the debt without parsing magic prose.
- A stale deferral cannot authorize a changed requirement.

## CC-07 — Detached, explicitly bound delivery

**Classification:** Preserve  
**Origin:** RFC-88, ADR-0005  
**Conditional on:** ADR-0005 (proposed) — detached-only change homes

A change is portable coordination state, not a product checkout. Targets and location-backed sources are explicit bindings with immutable resolved identities.

Each delivery binding retains:

- requested locator;
- resolved locator or revision;
- exact CID;
- exact adapter identity;
- target assignment;
- authorization generation.

The replacement need not preserve `discovery.yaml`. Equivalent store authority is acceptable.

**Acceptance evidence:**

- Changing a locator cannot reuse a CID from the previous locator.
- A source or target is not reread from a mutable origin after binding.
- Rebinding creates a new generation.
- Running in an unrelated directory cannot silently create a change.
- A multi-target change carries one independently verifiable binding per target.

## CC-08 — Private workspace isolation

**Classification:** Preserve  
**Origin:** RFC-87

Every build attempt runs in a fresh, private, disposable workspace materialized from an exact base CID.

The durable result is the captured relation:

`base CID → result CID + touched paths`

Workspace paths, live handles, and mutable directories are never workflow authority.

The workspace kernel may move between guest and host if the benchmark justifies it. Placement must not change semantics.

**Acceptance evidence:**

- Preparing a workspace never modifies the operator checkout.
- Capture round-trips files, deletes, binaries, modes, and symlinks.
- Change artifacts are granted separately and excluded from product capture.
- Losing or discarding a workspace loses no completed result.
- Retry starts from the recorded base and needs no workspace repair.
- Cancellation and timeout discard or sweep orphaned workspaces.

## CC-09 — Accepted CID and living product baseline

**Classification:** Preserve  
**Origin:** RFC-87, RFC-88

Product acceptance advances through content-addressed CIDs, not ambient checkout mutation or Git commit identity.

A successful merge:

- captures the complete accepted product tree;
- includes the updated behavioral baseline;
- advances the target's accepted CID exactly once;
- never applies a patch into the operator checkout;
- never aliases a CID to a Git SHA.

**Acceptance evidence:**

- Failure before commit leaves the accepted CID unchanged.
- Retry of an already committed transition is idempotent.
- The accepted tree contains product code and its living baseline.
- No `apply` or checkout write-back path exists.
- Every irreversible transition revalidates complete authorization immediately before commit.

## CC-10 — Engine-owned phase protocol

**Classification:** Preserve and extend  
**Origin:** RFC-90

The engine owns operation order, repair routing, budgets, terminal success, and terminal failure.

The target performs one pass per dispatch. It cannot:

- select its next phase;
- retry internally;
- reset a repair budget;
- suppress blocking findings;
- write the terminal report;
- claim success while a gate remains blocking.

The conserved protocol is:

`build → verify ⇄ repair → review ⇄ repair`

Merge verification and remediation move under the same engine-owned policy rather than remaining an adapter-private loop.

**Acceptance evidence:**

- Repair always returns through verification.
- Verification and review budgets cannot be exceeded or reset.
- Malformed reports terminate without lifecycle advancement.
- A target cannot self-certify terminal success.
- Merge repair follows the same one-pass and budget rules.
- Deterministic merge validation classifies every supported document exactly once; traversal and parse failures block rather than fail open.
- Complete phase history remains inspectable while only the latest authoritative reports gate success.

## CC-11 — Resumability from one authoritative state read

**Classification:** Replace  
**Origin:** RFC-86 computed status; architecture-review S1–S3

Re-running the stopped verb is always sufficient recovery. Status and dispatch consume the same validated state snapshot and cannot disagree about the next action.

The fact-union journal, wall-clock reducers, multi-writer claims, and manual lock recovery are not conserved. They are replaced by one transactional authority per change home.

**Acceptance evidence:**

- Crash injection at every state-write and external-effect boundary converges after retry.
- No recovery requires deleting locks, records, or workspace directories by hand.
- Status and dispatch identify the same next work item.
- Corrupt authority fails closed rather than projecting empty state.
- Historical generations cannot satisfy current lifecycle gates.
- Observability-log failure cannot alter lifecycle correctness.

## CC-12 — Deterministic bounded concurrency

**Classification:** Deferred mandatory  
**Origin:** RFC-96

Independent survey, extraction, decomposition, specification, and build work may execute concurrently on one node without changing results.

Concurrency changes dispatch and latency, not semantics.

The replacement must retain:

- deterministic ready-set ordering;
- bounded admission;
- one private workspace per build attempt;
- reusable successful sibling work after another item fails;
- cancellation without partial authoritative publication;
- cap-one and cap-N outcome equivalence.

The initial replacement runs at cap one until crash safety is proven. Multi-writer journal claims are not conserved.

**Acceptance evidence:**

- Cap one and the production cap produce equivalent topology, specifications, outcomes, and accepted CIDs.
- Independent targets build concurrently.
- Concurrent result persistence follows canonical order, not completion order.
- A failed item does not erase reusable successful siblings.
- No worker shares writable product, artifact, prompt, or continuation state.

## CC-13 — Atomic same-target waves

**Classification:** Preserve, initially exercised at cap one  
**Origin:** RFC-96

A same-target wave is a frozen antichain over one accepted base.

Its membership:

- is fixed before member builds;
- contains no dependency relationship between members;
- cannot silently shrink after failure;
- commits only after every member passes;
- advances the accepted CID once;
- exposes no authoritative prefix.

Composition accepts only same-base, non-overlapping results and is deterministic.

A failed member's successful siblings remain reusable as work items on retry (CC-12); they are never committed as a prefix of the failed wave. This deliberately resolves the addendum's acceptance criterion 19 to the no-prefix-commit branch.

The current wave manifests and events are not preserved as mechanisms.

**Acceptance evidence:**

- A ready batch containing dependent slices cannot become one wave.
- One member failure does not allow a successful prefix commit.
- Retry preserves membership or performs an explicit whole-wave retraction.
- Base mismatch or overlapping writes fail before candidate verification.
- Replaying a completed commit does not advance the CID twice.

## CC-14 — Verification of the exact composed result

**Classification:** Preserve via replacement  
**Origin:** RFC-96 domain convergence; architecture-review S4

The exact candidate CID committed by a wave is the candidate that verification evaluated.

For multiple slices on one target:

- child results are composed first;
- verification runs over the composed candidate;
- commit records that same candidate CID;
- completion verification evaluates the current accepted tree after required children and dependencies land;
- failed convergence blocks dependants, drain, and publication without rolling back already accepted work.

`DomainRound` files and the existing secondary verification protocol are not necessarily preserved. The guarantee should move onto the engine-owned phase protocol.

**Acceptance evidence:**

- The verification input CID equals the committed CID.
- A combined candidate cannot inherit independent passing reports as proof of combined correctness.
- Failed combined verification commits nothing.
- Failed completion verification blocks drain and publication with a typed stop.
- Retry can repair or reverify without incidental input changes.

## CC-15 — Coordinated multi-target publication

**Classification:** Deferred mandatory for multi-target delivery  
**Origin:** RFC-95

A completed change can be published as one coordinated set derived from its in-scope targets and dependency topology.

For each member:

- only the final accepted CID is materialized;
- the operator receives a normal reviewable Git worktree;
- Emery does not author commits, push branches, open pull requests, merge, or revert;
- landing order follows the contracted target dependency graph;
- archive observes forge state and reports incomplete or out-of-order publication.

Publication is not execution state and does not replace private workspaces or accepted CIDs.

**Acceptance evidence:**

- Multiple slices on one target produce one final publication member.
- Intermediate CIDs receive no publication branch.
- Multi-target order is derived deterministically from dependencies.
- The operator remains the author of every Git and forge mutation.
- Archive fails closed on forge transport errors.
- Publication failure does not erase or falsify an already accepted CID.
- N=1 may complete without an unnecessary publication clone unless publication is requested.

## CC-16 — Coverage-accounted archaeology and architecture projection

**Classification:** Preserve via re-homing  
**Origin:** RFC-104, ADR-0003  
**Conditional on:** ADR-0003 (proposed) — one lifecycle; if two lifecycles are kept instead, this entry is re-shaped around closing the seam findings (P2, P5, P6, P11, P12, S41–S45) rather than re-homing

The mandatory second lifecycle is not conserved. Its valuable deliverable capabilities are.

When architecture recovery is requested, Emery can project from the same source and Evidence corpus:

- a declared system boundary;
- explicit coverage dispositions for included, excluded, inaccessible, unsupported, and unresolved material;
- the exact observed source identities;
- an evidence-linked as-is architecture;
- target and transition architectures;
- modernization dispositions;
- state movement, coexistence, cutover, rollback, and acceptance concerns;
- bounded migration waves or equivalent delivery topology.

This projection must not require hand-authored configuration before first useful output.

**Acceptance evidence:**

- Every declared source or system element receives an explicit coverage disposition.
- Failed sources remain visible and cannot silently count as current evidence.
- Observed CIDs remain distinct from delivery-binding CIDs.
- Architecture diagrams are projections, never authority.
- An architecture-only engagement can finish with a reviewable deliverable.
- If delivery follows, accepted results can advance the living architecture projection without creating a second lifecycle authority.

## CC-17 — Dynamic typed adapter boundary

**Classification:** Preserve and harden  
**Origin:** Existing WIT seam, ADR-0002

A source or target adapter can be added at an exact version without rebuilding the host. The same engine core remains embeddable behind a desktop CLI and a service ingress; the conserved duality is architectural embeddability, not a live web service — the mutating HTTP surface stays disabled until an ingress design exists (C3, target-architecture §7).

The native shadow provider, bare-name resolution, pull-on-miss, cache seeding, and marketplace machinery are not conserved.

The conserved capability requires:

- one WIT-defined type family;
- exact adapter identity;
- explicit admission;
- per-axis least-privilege capabilities;
- enforced wall-clock, CPU/fuel, memory, and output budgets;
- adapter-embedded prose and references served lazily to judgment legs under a scoped, per-dispatch grant (D9, addendum D14);
- automated execution across the production component seam.

**Acceptance evidence:**

- CI admits and dispatches one out-of-binary exact-pin component.
- The complete journey crosses the component seam.
- Malicious source and target fixtures cannot access undeclared capabilities.
- Infinite-loop, memory-growth, output-flood, and silent-pending fixtures terminate within budgets.
- A judgment leg can read a served adapter reference under a grant scoped to its dispatch, not a global shelf.
- Native-only integration cannot become the authoritative test path.

## CC-18 — Deterministic and traceable authorization

**Classification:** Preserve via replacement  
**Origin:** RFC-86, RFC-88, RFC-91

Every irreversible effect is authorized by one complete, current identity covering:

- change generation;
- topology;
- slice;
- source and target bindings;
- specification receipt;
- target adapter identity;
- accepted base CID;
- wave membership;
- relevant policy and dispositions.

The current epoch events and digest-carrying YAML files may be replaced by transactional preconditions.

**Acceptance evidence:**

- Mutation of any covered input revokes outstanding authorization.
- Rebinding or force-authoring creates a new generation.
- An old build, correction, deferral, or publication result cannot satisfy a new generation.
- Candidate capture and commit recheck authorization immediately before acting.
- Status can explain which covered identity became stale.

## CC-19 — Durable conversational correction

**Classification:** Replace  
**Origin:** RFC-88 inert amendment proposals; `plan correct` design intent (removed, remediation Phase 0 item 2); remediation P9; target-architecture §6

A stuck slice or wrong specification can be corrected by operator guidance without hand-editing state or abandoning the slice. Conversation stays at the call site; the recorded guidance fact — not the conversation — gains authority.

Correction guidance:

- is durable, digest-bound, and generation-scoped;
- names exactly one resolvable correction target (the S25 lesson);
- is consumed as a hard input by the retry at the stage that stopped;
- is consumed on honor and lapses with its generation — never immortal (S16);
- never applies itself: model-proposed corrections remain inert until an explicit operator act.

The `plan amend --proposal` surface and the removed `plan correct` verb are not preserved as mechanisms.

**Acceptance evidence:**

- A stuck slice exposes its typed stop and repair brief, and accepts guidance.
- The retry consumes the guidance as a hard input; an unhonored guidance fact is visible, not silently dropped.
- Guidance bound to a stale generation or an unresolvable target is refused.
- An honored correction is traceable from the resulting specification or build.

## Intentionally not conserved

Unless a later ADR reopens them, remediation does not preserve:

- the journal as lifecycle authority (ADR-0001);
- the multi-writer claim protocol (architecture-review S4);
- stored mutable progress labels;
- a mandatory definition-home lifecycle (ADR-0003, proposed);
- hand-authored `scope.yaml` or `coverage.yaml` before first output (ADR-0003, proposed);
- the in-place/detached mode distinction (ADR-0005, proposed);
- merge-time checkout `apply` (deleted by implemented RFC-88);
- the native shadow provider (ADR-0002, accepted);
- bare adapter names, pull-on-miss, cache seeding, and pull-latest (ADR-0002, accepted);
- adapter-private retry loops (superseded by CC-10);
- universal conflict auto-deferral (ADR-0004, proposed);
- the `plan amend --proposal` and removed `plan correct` verbs as mechanisms (design intent conserved by CC-19);
- exact legacy artifact names or crate boundaries;
- default cap-four execution before crash safety is demonstrated (CC-12).

Rows citing a proposed ADR are conditional on its acceptance: if the ADR resolves differently, the row moves back above as a capability entry before any deletion proceeds.

## Remediation gates

### Phase 3 exit

The walking skeleton must prove at least:

- CC-01 source fidelity;
- CC-02 decomposition, including one forced focused split;
- CC-03 topology review, correction, and abandonment authority;
- CC-04 specification firewall;
- CC-05 review document;
- CC-06 conflict/divergence/debt behavior;
- CC-07 detached explicit binding;
- CC-08 private workspaces;
- CC-09 accepted CID and baseline;
- CC-10 phase protocol;
- CC-11 crash recovery;
- CC-13 wave atomicity at cap one;
- CC-14 exact-candidate verification;
- CC-17 production component seam;
- CC-18 authorization freshness;
- CC-19 durable correction guidance (skeleton step 5 exercises it directly).

### Phase 4 exit

Remediation is not capability-complete until it additionally proves:

- CC-12 cap-N concurrency and cap equivalence;
- CC-15 multi-target publication;
- CC-16 coverage-accounted architecture projection;
- all first-party source and target adapters over the conserved seam.

Legacy implementations of deferred capabilities remain quarantined until their corresponding Phase 4 evidence is green or an ADR explicitly deletes the capability.
