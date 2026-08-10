# RFC-91: Staged Refinement

> Status: Draft — corrective workflow substrate for specs-first operation and a prerequisite to [RFC-92](rfc-92-concurrent-execution.md) and [RFC-94](future/rfc-94-streaming-execution.md). It separates the phase scheduler and authority fences that concurrency and streaming extend.
>
> Owns: the first-class `plan refine` stage; phase-specific leaf readiness; refinement records covering complete build inputs; separation of refinement, build, and commit fences; operation-scoped claims; and the plan scheduler's move from active entries to `(slice, phase)` work items.
>
> Amends: [RFC-86](rfc-86-change-facts.md) D6, D8, D12, D14, D22, D25, and D26; [RFC-88](rfc-88-detached-changes.md) D7 and D8. It removes `refine-under-epoch`, refine-time target-base capture, and execute's implicit refine-before-build path.
>
> Depends on completed [RFC-86](rfc-86-change-facts.md), [RFC-87](rfc-87-working-trees.md), and [RFC-90](rfc-90-build-verification.md). It can land against today's flat plan and one-member waves while [RFC-88](rfc-88-detached-changes.md) supplies the later detached accepted-CID binding.
>
> Does not own: recursive discovery and decomposition ([RFC-88](rfc-88-detached-changes.md)); target task decomposition, worker-pool concurrency, domain convergence, or multi-member waves ([RFC-92](rfc-92-concurrent-execution.md)); transport and distributed coordination ([RFC-93](rfc-93-distributed-execution.md)); partial branch publication while survey continues ([RFC-94](future/rfc-94-streaming-execution.md)); or the target build phase machine ([RFC-90](rfc-90-build-verification.md)).

## Intent

Make “generate and review every specification before generating software” an ordinary Emery workflow without turning it into a separate lifecycle or preventing future asynchronous streaming.

The operator workflow becomes:

```text
plan author → plan refine → review specs and gaps → plan execute → plan archive
```

The internal workflow is not a plan-wide barrier pipeline. It is a graph of phase work items:

```text
(slice, refine) → (slice, build) → (slice, merge)
```

Each phase has its own readiness predicate and input fence. A closed branch may refine while another branch is still being authored under RFC-94; an independently ready refined leaf may build while other leaves refine; no result may commit until a commit-capable closed-plan grant covers the reviewed refinement records.

This RFC makes staged and streaming operation two scheduler policies over one substrate:

- **staged refinement** grants work through `refine`, drains the closed plan's refinement graph, and stops before product-code work;
- **closed execution** grants work through `commit`, requires existing fresh refinement records, and drains build and merge;
- **streaming execution** later grants work through `build` over partially published branches and defers commit to a later closed-plan grant.

## Problem

### The public surface lost its refinement boundary

RFC-86 preferred shift-left refinement but rejected `plan refine` to preserve a three-verb surface. Its original alternative relied on per-slice refine breakouts. The later plan-centric cut removed those breakouts and made `plan execute` the only driver of `refine → build → merge`.

The resulting workflow can stop after topology review or after generated software, but not after complete specification generation. An `execute --until refined` bound appears small, yet it preserves the couplings that prevent refinement from being a real stage.

### Execution dependencies currently block specification generation

The current scheduler admits a pending entry only when every `depends-on` entry projects `done`. `done` requires merge. A run that never builds or merges can therefore refine only the initial ready frontier; any dependent slice remains unrefined until software is generated upstream.

This contradicts the requested outcome for ordinary dependency graphs: every in-scope spec should be available for review before generation spend.

### Refinement pins a future build base too early

Current refinement freezes the product tree into `base.yaml.target-base`; build later prepares from that snapshot. This couples Evidence extraction and specification synthesis to the code base that a future build will mutate.

For a dependent slice, the correct build base is the accepted result of its dependencies. That value does not exist when the full plan is refined ahead of build. Waiting for dependency merge preserves the pin but defeats specs-first operation; refining early preserves the review seam but makes the code pin stale.

Source and planning inputs are knowable at refinement. The target code base is knowable at wave open. They must not share one pin record.

### Entry status is being used as scheduler authority

The current loop projects a first active `in-progress` entry, resumes it, and advances another entry only when no active entry is selected. A local set of “already refined during this process” can skip a phase, but it cannot make the persistent scheduler understand that one entry is ready for build while another is ready for refine.

The platform contract already permits many slices in flight. Scheduling must select ready phase work, not treat one entry as the cursor for the whole loop.

### Authorization covers the wrong boundary

`plan.execute.started` currently mixes three concerns:

- the operator starting execution;
- coverage for existing specs or unknown future specs through `refine-under-epoch`;
- authority for build and wave commit.

A specs-only run would stamp code authorization even though no code work was requested. Conversely, RFC-94 needs authority to refine and build without authority to commit. The authority ceiling and the covered inputs must be explicit and independent.

Coverage also names only the specs tree, while target generation consumes the complete refinement output: `proposal.md`, `design.md`, `tasks.md`, per-domain specs, and adapter-declared additional inputs.

## Terms

- A **phase work item** is `(slice, phase, input-digest)` for one of `refine | build | merge`. The input digest fences a claim against stale work.
- A **refinement record** is the immutable, content-addressed record of one successful refinement's exact inputs and complete output bundle.
- A **refinement bundle** is every artifact the target build request may consume for a slice, with path, kind, and content digest.
- A **phase ceiling** is `refine | build | commit`, the maximum authority granted by one plan-run fact.
- A **closed-plan grant** binds one reviewed complete plan revision. A **streaming-discovery grant** is RFC-94's partial-publication coverage and may never carry a `commit` ceiling.
- An **execution dependency** is the existing `depends-on` edge interpreted at each phase: predecessor refinement before dependent refinement, predecessor acceptance before dependent build.
- **Accepted** means the predecessor's target wave committed and advanced the target's accepted CID.

## Operator flow

### Author — topology only

`emery plan author` continues to survey sources, reconcile or recursively decompose leads, and project terminal leaves into `plan.yaml`. It does not extract Evidence or synthesize slice artifacts.

The operator reviews topology before paying for full extraction and synthesis.

### Refine — complete specifications, no software

`emery plan refine [--slice <slice>...]` opens a closed-plan run with ceiling `refine` and drains eligible refinement work:

1. select a leaf whose predecessor refinement records are fresh;
2. claim its fenced refine work item;
3. extract every bound source;
4. synthesize and validate the slice artifacts;
5. write one immutable refinement record;
6. release the operation claim;
7. continue until every selected in-scope leaf has a fresh refinement record or a typed stop occurs.

Without selectors, the command targets every in-scope leaf. Selectors support focused re-refinement after input correction or a poor synthesis result; dependencies needed to make the selected work coherent are read, not implicitly re-refined unless stale.

Successful refinement may contain `[unknown]`, `[conflict]`, or `[divergence]`. As today, these are refinement outputs. The command exits successfully after persisting them and points the operator to `emery plan gaps`.

No target build operation, product workspace preparation, target wave, `BuildRecord`, merge gate, or accepted-CID mutation may occur under a refine ceiling.

### Review — human-owned

The operator reviews `proposal.md`, `design.md`, `tasks.md`, per-domain specs, provenance, and `plan gaps`. The engine does not add an approval file, checklist, or projected `approved` state.

Changing an input or refinement artifact invalidates the corresponding refinement record. The operator re-runs `plan refine`; execute never silently re-refines and then builds an unreviewed replacement.

### Execute — build and commit reviewed refinements

`emery plan execute` requires a fresh refinement record for every in-scope leaf it may build. At start it opens a closed-plan grant with ceiling `commit`, covering the exact refinement-record digests and any explicit unknown waivers.

It then drains the existing engine-owned build and merge phases:

```text
target build → verify ⇄ repair → review ⇄ repair → target-wave merge
```

If any selected refinement is missing or stale, execute fails before opening a wave and points to `emery plan refine`. It does not carry `refine-under-epoch`.

`--waive <slice>/<req> --reason <reason>` remains execute-only because a waiver authorizes build across a reviewed unknown; it has no meaning during refinement.

### Archive — unchanged

After publication, `emery plan archive` verifies its existing completion conditions and archives the change.

## Phase readiness

### Refine readiness

A leaf is ready to refine when:

- it is in scope and not dropped;
- its plan, discovery, lead-catalog, decomposition, target profile, source bindings, and resolved target-guidance identities are closed and available;
- every `depends-on` predecessor has a fresh refinement record;
- no live claim owns the same fenced refine work item.

Predecessor refinement records enter the dependent synthesis input as ordered change-local context. The input names each predecessor slice, refinement-record digest, and readable artifact roots. This permits a dependent spec and design to build on reviewed upstream intent without waiting for software or treating upstream slice artifacts as merged baseline.

Independent leaves refine concurrently. A dependency chain refines in topological waves without generating code.

### Build readiness

A leaf is ready to build when:

- it has a fresh refinement record covered by a grant with ceiling `build` or `commit`;
- its typed gap policy passes under that grant's explicit waivers;
- every `depends-on` predecessor is accepted;
- the target's current accepted CID and dependency frontier are available;
- no incompatible wave or claim owns the work.

The target wave selects its base from the current accepted CID at wave open. It binds the refinement-record digest, dependency frontier, target identity, and build-authorizing grant.

### Merge readiness

A wave is ready to commit when:

- every member has a successful `BuildRecord` bound to the frozen wave;
- all slice and domain gates pass;
- a current closed-plan grant with ceiling `commit` covers every member refinement record and waiver;
- the accepted base and dependency frontier still match.

A streaming-discovery grant can never satisfy the last condition.

## Decisions

### D1 — `plan refine` is a first-class orchestration stage

The public workflow is `plan author → plan refine → plan execute → plan archive`. `/emery:refine` is an ultrathin invoke-and-relay wrapper over `emery plan refine`.

This supersedes RFC-86 D14 and D26. The earlier rejection preserved a three-verb surface around a per-slice breakout that no longer exists. Reconstructing the same stage as `execute --until refined` would preserve the spelling of that decision while reversing its substance.

`plan author` remains topology-only. Refinement is not folded into author because the topology review seam remains valuable.

### D2 — There is no plan-wide Refined lifecycle state

Refinement authority and completion are per leaf. `Ready` remains a read-only projection: every in-scope leaf has a fresh refinement record and the clean gap policy passes.

The plan keeps no mutable lifecycle field and gains no `approved` rung. A batch `plan refine` success means the requested refinement work set reached its bound; it does not stamp a plan state.

### D3 — One dependency edge has phase-relative satisfaction

The existing acyclic `depends-on` graph remains the ordering graph:

- dependent refine requires predecessor **refined**;
- dependent build requires predecessor **accepted**;
- dependent merge revalidates the accepted dependency frontier.

The scheduler does not require predecessor merge merely to author a dependent spec. It also does not ignore dependencies during refinement: predecessor refinement records become explicit pinned context.

No second dependency graph is introduced until evidence demonstrates a topology that cannot be represented by the phase-relative interpretation.

### D4 — A refinement record covers complete generation intent

After successful synthesis and validation, the engine writes an immutable content-addressed refinement record. Conceptually:

```yaml
version: 1
slice: orders-api
inputs:
  plan: sha256:…
  discovery: sha256:…
  decomposition: sha256:…
  profile: sha256:…
  target-guidance: sha256:…
  baseline-specs: sha256:…
  sources:
    intent: sha256:…
  dependencies:
    - slice: shared-types
      refinement: sha256:…
outputs:
  proposal.md: sha256:…
  design.md: sha256:…
  tasks.md: sha256:…
  specs/orders/spec.md: sha256:…
build-inputs:
  - path: proposal.md
    digest: sha256:…
  - path: design.md
    digest: sha256:…
  - path: tasks.md
    digest: sha256:…
  - path: specs/orders/spec.md
    digest: sha256:…
```

The concrete schema is generated from a Rust DTO and rejects unknown fields. `build-inputs` covers the canonical build request, including present adapter-declared additional inputs. A required declared input missing at refinement completion prevents the record from becoming fresh.

The current artifact files remain the human-readable working projection. Their digest set must match the record before build.

### D5 — Target code base is selected at wave open, not refinement

Refinement records source, planning, baseline-spec, guidance, dependency-refinement, and output identities. It does not freeze the product tree and carries no `target-base`.

The wave owns the exact target base. Closed in-place execution selects the current product snapshot at wave open; RFC-88 detached execution selects the target's current accepted CID. Dependent waves therefore naturally start from accepted predecessor results.

This supersedes RFC-86 D25's refine-time target-base rule. Build still never uses an unrecorded ambient base: wave open records the selected value before any writable workspace is prepared.

### D6 — `plan.run.started` grants carry an explicit phase ceiling

The command-shaped `plan.execute.started` fact is replaced by `plan.run.started`, a plan-run grant with:

- coverage kind: `closed-plan | streaming-discovery`;
- phase ceiling: `refine | build | commit`;
- exact planning revision coverage;
- per-leaf refinement coverage when the ceiling permits build;
- optional unknown waivers only when the ceiling permits build;
- operator/writer identity and sequence.

Command mapping is fixed:

- `plan refine` → `closed-plan`, ceiling `refine`;
- `plan execute` → `closed-plan`, ceiling `commit`;
- RFC-94 streaming start → `streaming-discovery`, ceiling `build`.

A lower ceiling can never be inferred upward from completed work. A build result produced under a streaming grant remains inert until a later commit grant covers and revalidates it.

`refine-under-epoch` is removed. A build-capable grant covers only existing refinement-record digests; unknown future refinement cannot be treated as reviewed.

There is one grant vocabulary rather than separate incompatible authorization models. `plan.execute.started` is retired in the pre-1.0 cut; no compatibility alias or dual authority projection survives.

### D7 — The scheduler selects fenced phase work items

Scheduler authority moves from “first in-progress entry” to a deterministic set of ready `(slice, phase, input-digest)` work items.

Selection order is canonical by target, topological layer, plan order, slice, and phase where a cap requires truncation. RFC-92 may dispatch a bounded antichain concurrently without changing readiness or identity.

A claim names the phase and input digest. It prevents duplicate execution of the same operation, not all future work on the slice. The owner releases it when the operation completes, fails terminally, or is retracted. A later phase or changed input uses a different claim identity.

Per-entry `pending | in-progress | done` remains a coarse projection:

- `pending` — no durable phase output and no live work;
- `in-progress` — a refinement/build record exists short of accepted completion, or a live claim exists;
- `done` — the existing absorbing wave/archive facts.

Multiple entries may project `in-progress`; status never treats the first one as the scheduler cursor.

### D8 — Status projects work frontiers and review seams

`plan status` remains read-only. It projects:

- counts and coarse entry status;
- the number of ready, running, blocked, failed, and complete work items per phase;
- `Ready` from fresh refinement records plus clean gaps;
- the next operator action.

`Authorized` projects only from a fresh grant with ceiling `build` or `commit`; a refine-only grant never presents code work as authorized.

After author, resume points to `/emery:refine`. After successful refinement:

- conflicts point to input correction and focused re-refinement;
- unknowns point to correction or an explicit execute waiver;
- clean refinement points to `/emery:execute`.

Missing or stale refinement always points back to refine. Execute does not repair the review boundary by silently synthesizing new artifacts.

### D9 — Streaming pipelines the same work items

RFC-94 adds partial publication and the `streaming-discovery` grant; it does not add another scheduler.

Under streaming:

1. a closed discovery/decomposition branch publishes immutable revisions;
2. ready leaves refine against those revisions while survey continues elsewhere;
3. independent refined leaves may build under a ceiling-`build` grant;
4. build results persist as immutable snapshots and records but cannot commit;
5. later branch changes invalidate exactly the refinement and build records whose input digests reference superseded revisions;
6. a reviewed closed plan opens a ceiling-`commit` grant and revalidates waiting waves before commit.

Execution dependencies still require accepted predecessors for build. Streaming may therefore build independent ready antichains while dependent build layers wait for closed-plan commit. It does not create a speculative accepted-CID chain.

### D10 — This is a hard workflow cut

Pre-1.0, staged refinement replaces:

- `execute --until refined`;
- execute-time implicit refinement;
- `refine-under-epoch`;
- refine-time `target-base`;
- spec-only authorization coverage;
- singular active-entry scheduling;
- lifecycle-long slice claims.

No compatibility aliases, dual event projections, or fallback reads of the old pin shape survive. Existing projects re-init or recreate active changes across the cut.

## Persistence and authority

### Refinement records

Immutable records live at `.emery/slices/<slice>/refinements/<digest>.yaml`. The latest successful body is also projected at `.emery/slices/<slice>/refinement.yaml` for direct inspection. The projection is byte-identical to its immutable record; freshness is always recomputed from the immutable record and live covered artifacts.

An interrupted refinement may leave Evidence or staging files but writes no successful refinement record. Re-entry starts a new operation attempt; it does not infer completion from partial files.

### Build and wave records

Wave member inputs replace the current spec-only digest with the refinement-record digest. The spec digest remains available inside the record for requirement-identity and merge checks.

`BuildRecord` continues to bind base/result snapshots, touched paths, wave digest, and terminal build report. RFC-90 attempt and phase reports remain audit evidence rather than lifecycle authority.

Successful target phases may promote target-owned changes such as task progress after build. They do not rewrite the pre-build refinement record. Freshness against the live refinement bundle gates a leaf awaiting build; once built, merge revalidates the immutable refinement input through the wave and `BuildRecord`.

### Commit authorization

The committed wave fact names the commit-capable grant separately from the wave's build grant. Closed execution may use one grant for both. Streaming necessarily uses different grants.

The current serial implementation's reuse of build authorization as commit authorization is retired.

## Failure and restart

- Refine dispatch or validation failure parks only that fenced refine work item. Independent ready refinement continues when the scheduler policy permits; the batch command returns the typed stop summary after quiescing its local work.
- Build and merge failures retain RFC-90/RFC-92 stop and retry semantics.
- A stale input invalidates a queued or completed work item by digest; it does not mutate or delete historical records.
- Re-running `plan refine` schedules only missing, stale, failed, or explicitly selected refinement work.
- Re-running `plan execute` reuses valid successful build and domain records, but refuses missing/stale refinement.
- Claims from a crashed writer expire or retract under RFC-93's fenced-claim policy; desktop re-entry uses the same semantics through its local provider.

## Implementation requirements

- Add `emery plan refine [--slice <slice>...]` and the ultrathin `/emery:refine` wrapper. Remove any planned `execute --until refined` surface.
- Replace the execute loop's active-entry cursor with reusable phase work-item projection and deterministic selection. Both `plan refine` and `plan execute` use that scheduler with different ceilings.
- Replace lifecycle-long slice claims with fenced operation claims carrying slice, phase, and input digest.
- Add the typed content-addressed refinement-record schema, atomic persistence, freshness projection, and complete build-input manifest.
- Pass ordered predecessor refinement records into dependent synthesis.
- Remove `target-base` from refine-time pin assembly and remove product-tree `freeze` from refinement. Select and persist the target base when opening a wave.
- Replace wave `MemberInputs.spec` authority with a refinement-record reference while preserving the nested spec identity needed by merge.
- Replace `plan.execute.started` with the typed `plan.run.started` grant, explicit phase ceilings, and separate build/commit anchors. Remove `refine-under-epoch`.
- Make execute reject missing or stale refinement before any build workspace or target wave is created.
- Preserve the RFC-90 `build → verify ⇄ repair → review ⇄ repair` machine unchanged.
- Update plan status, gaps hints, CLI output shapes, skills, workflow standards, RFC-86/88/92/94 relationships, and platform-series documentation in the same change.
- Keep the native and Wasm providers on one typed transport contract; do not add a WIT source or target operation.

## Acceptance criteria

1. A three-leaf dependency chain reaches fresh refinement records for all three through one `plan refine` run, in topological order, with no target build/merge event, target wave, `BuildRecord`, product workspace freeze, or product-code change.
2. Two independent leaves refine concurrently under the RFC-92 pool and produce the same canonically ordered records and status projection as cap one.
3. A dependent refinement input binds its predecessor's refinement-record digest. Re-refining the predecessor invalidates the dependent and schedules it again.
4. A successful refinement record covers every path the assembled target build request consumes. Changing `proposal.md`, `design.md`, `tasks.md`, a spec, or an adapter-declared additional input makes execute refuse before wave open.
5. A dependent build opens only after predecessor acceptance and records the target's then-current accepted CID as its wave base. No refine-time target-base exists.
6. A refine-ceiling grant cannot dispatch build or merge. A build-ceiling streaming grant cannot commit. A commit-ceiling closed-plan grant may commit only covered, revalidated refinement and build results.
7. Execute over an unrefined or stale leaf returns a typed refinement-required result and never auto-refines. Re-running refine repairs the condition; re-running execute resumes normally.
8. Refinement succeeds with typed gaps. Conflicts remain non-waiveable; unknown waivers remain execute-only and are bound to the covered refinement record.
9. Multiple entries and phases may be in progress concurrently. Status and scheduling do not depend on a singular active entry, and duplicate claims on the same fenced work item fail.
10. An interrupted refinement writes no successful record; restart neither treats partial artifacts as fresh nor repeats already valid sibling refinements.
11. A streaming fixture surveys one branch while another refines and an independent third leaf builds. No wave commits until a later closed-plan commit grant; a superseded branch revision invalidates only its digest-bound descendants.
12. Existing RFC-90 success, repair, review-remediation, failure, and attempt-abandonment fixtures pass without target WIT changes.
13. `cargo make ci`, the wasm32 engine compile-check, native integration suites, and the operator-invoked streaming evaluation fixture pass.

## Rejected alternatives

- **`plan execute --until refined` over the current loop** — refines only the merge-ready frontier, stamps code authorization for a specs-only request, retains refine-time code pins, and requires process-local skip state around an entry scheduler that does not understand phase work.
- **Rename execute to build** — target `build` means generation only. The operator stage also owns verification, repair, review, wave merge, and accepted-CID progression; `execute` remains the accurate umbrella.
- **Fold refinement into author** — removes topology review and pays extraction/synthesis cost before the operator accepts decomposition.
- **Keep implicit refine inside execute** — allows newly generated or re-generated specs to enter build without the human review seam this RFC exists to create.
- **Ignore `depends-on` during refinement** — permits dependent intent to be synthesized without pinning the upstream slice it relies on. Phase-relative satisfaction retains one graph and makes the dependency explicit.
- **Add a second refinement dependency graph immediately** — duplicates topology before evidence shows that execution and specification ordering differ in practice. A later amendment may split the relation if retained plans demonstrate the need.
- **Keep refine-time target-base and re-refine after dependencies merge** — repeats nondeterministic extraction and synthesis, invalidates reviewed specs as an ordinary path, and serializes refinement behind code generation.
- **Treat a plan as globally Refined** — creates a mutable barrier lifecycle that conflicts with partial publication, focused re-refinement, and streaming invalidation. Freshness remains per leaf.
- **Let a streaming build grant commit when its plan later closes** — upgrades authority retroactively. The later closed-plan gesture must mint an explicit commit-capable grant and revalidate the waiting result.
