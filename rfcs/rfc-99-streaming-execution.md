# RFC-99: Streaming Execution

> Status: **Parked.** Reopen Phase A only when measured authoring duration or time-to-first-refinement shows that complete-plan closure is a material engagement bottleneck. Reopen Phase B only after RFC-96 Phase B is complete and a client engagement requires unattended candidate build before final closure. Phase A depends on [RFC-96](rfc-96-concurrent-execution.md) Phase A; Phase B depends on RFC-96 Phase B (`compose` and multi-member waves). [RFC-106](rfc-106-task-graphs.md) is optional unless a candidate is too large for one `target.build`. Distribution through parked [RFC-100](rfc-100-distributed-execution.md) remains optional.
>
> Patch ownership: this RFC amends RFC-86 D27, RFC-88 D1 / D3 / D7 / D8, and RFC-90's wave-only build envelope after those contracts land. Those predecessor RFC texts remain unchanged.

## Intent

Support three policies over one fact and work-item substrate:

1. author a complete plan, pause for topology review, refine it, and optionally pause again;
2. publish and refine closed branches while survey and decomposition continue elsewhere, then stop before software;
3. continue through build without human review, using exact policy admission and machine gates, while accepted-CID mutation remains behind a later closing gesture.

Time to first refinement and first build stop being bounded by complete-tree closure. Human review remains available without becoming lifecycle state, and unattended work never masquerades as reviewed.

## Relationship to staged execution

[RFC-91](rfc-91-refinement-stage.md) remains the complete-plan reference path:

```text
plan author → [optional review] → plan refine → [optional review] → plan execute
```

This RFC adds a higher-level automation runner. It reuses RFC-88 planning revisions, RFC-91 refinement manifests, RFC-96 work items, RFC-90 build records, and the existing slice lifecycle. It adds no Progressive or Approved state.

The manual stage commands remain independently callable. An operator may also invoke refinement and execute back to back; the review seam is an opportunity, not an attestation.

## Predecessor patches

RFC-99 lands as a forward patch. It does not revise the implemented RFC-86 text or the in-flight RFC-88 text.

### RFC-86 D27 and RFC-90 build envelope

The `BuildRecord` authorization anchor becomes `wave | candidate-batch`. A successful record plus its matching anchor fact may project `built`; only `wave` can satisfy merge readiness or accepted-CID projection.

### RFC-88 D1 — Change-root additions

The detached layout gains:

```text
planning/branches/<digest>.yaml
slices/<slice>/refinements/<digest>.yaml
targets/<target>/candidate-batches/<digest>.yaml
targets/<target>/candidate-frontiers/<digest>.yaml
```

All are immutable content-addressed records. In-place mode adds the same paths beneath `.emery/change/`.

### RFC-88 D3 — Progressive publication

Complete `decomposition.yaml` / `plan.yaml` publication remains the reference policy. Progressive mode may additionally publish a validated closed branch while unrelated discovery remains open. Final `plan.yaml` remains complete-only.

### RFC-88 D7 — Candidate lineage

Candidate batches and frontiers form a non-authoritative target lineage beside, never inside, `targets/<target>/waves/`. They do not advance the accepted CID. A commit-capable run revalidates candidate results into current waves.

### RFC-88 D8 — Progressive authority and amendment quiescence

`plan.run.started` and exact member admission authorize candidate build without commit. Amendment proposals created during a progressive run additionally bind affected candidate-batch and candidate-frontier digests. Application compare-and-sets and quiesces those records alongside RFC-88's planning, claim, wave, and accepted-CID frontiers.

## Terms

- A **progressive run** is one orchestration that authors, refines, and optionally builds from immutable branch publications.
- A **run bound** is `refined | built`, the furthest lifecycle result the runner may produce. This RFC does not permit an unattended `merged` bound.
- A **publication policy** is `complete | progressive`. Complete waits for final plan closure; progressive exposes validated closed branches before the full tree closes.
- A **branch publication** is an immutable record covering one closed conflict-domain branch, its retained lead and decomposition revisions, projected leaves, dependencies, source and target pins, adapter identities, and model-capability profile.
- A **run policy** is a closed deployment-owned profile that authorizes unattended candidate work through `built`. Its digest enters every admission and result. RFC-102 defines the stricter promoted autonomy-policy kind required for `merged`.
- A **member admission** binds one exact refinement manifest and gap inventory to a parent run policy before build.
- A **candidate frontier** is a non-authoritative target CID assembled from admitted build results in one run. It may base dependent builds but never advances the accepted CID.
- **Final closure** means the complete retained lead/decomposition tree and canonical `plan.yaml` have published and every surviving leaf maps to one current branch revision.

## Operator surface

The manual workflow remains:

```text
emery plan author
emery plan refine
emery plan execute
```

The progressive workflow adds:

```text
emery plan run --publication progressive --through refined
emery plan run --publication progressive --through built --policy <profile>
```

`plan run` accepts the same authoring criterion when no plan exists and resumes the retained run when planning artifacts already exist. `/emery:run` is an ultrathin invoke-and-relay wrapper.

`--through refined` creates no code-work grant and requires no policy profile. `--through built` requires a resolved run policy and may prepare private workspaces, but it cannot merge, materialize a publication worktree, publish, or mutate an accepted CID.

The concurrency cap is deployment policy with an optional operator reduction. A cap of one follows the same scheduler and remains the deterministic reference path.

## Phase A — Progressive refinement

### Branch publication

RFC-88's first implementation publishes `decomposition.yaml` and `plan.yaml` only after the complete tree passes. Progressive refinement adds immutable `planning/branches/<digest>.yaml` records without making `plan.yaml` partially valid.

A branch may publish when:

- its retained lead coverage is closed;
- its decomposition subtree passes RFC-88 validation and complexity bounds;
- every projected leaf has exact source, target, adapter, profile, and dependency identities;
- every projected leaf has RFC-91's canonical planning-entry, contributing-lead, and decomposition-scope projections;
- every dependency outside the branch names a published predecessor or remains explicitly unresolved;
- no open survey can change the branch without producing a superseding revision.

The engine appends `planning.branch.published` with the branch-record digest. `plan status` may project published leaves before final closure, but `plan.yaml` remains the canonical complete-tree projection and is written only when authoring closes.

Completion order never changes record bytes or final plan order.

### Refinement dispatch

Each published leaf becomes an RFC-96 Phase-A `refine` work item keyed by `(slice, refine, input-digest)`. Independent survey, decomposition, extract, and refinement calls share the bounded local pool.

Refinement uses RFC-91 unchanged:

- Evidence remains per terminal `(source, lead)`;
- phase-relative dependencies require predecessor refinement;
- synthesis writes the ordinary slice artifacts;
- success atomically writes the canonical refinement manifest.

The manifest binds the branch's slice-local planning projections, not a not-yet-existent whole-plan digest. If final closure projects the same leaf bytes, its manifest remains fresh. A changed local projection supersedes the branch and invalidates the manifest.

No target operation, product workspace, target wave, `BuildRecord`, build grant, or accepted-CID mutation occurs in Phase A.

### Final closure and stop

`--through refined` succeeds only after:

- authoring reaches final closure;
- canonical `plan.yaml` exactly projects the retained decomposition;
- every surviving in-scope leaf has a fresh refinement manifest;
- every superseded leaf is absent from the final projection.

The useful latency milestone is earlier: the first fresh manifest may appear while unrelated surveys remain open. Failure to close the final plan leaves retained branch and refinement records for resume but does not report successful completion.

### Invalidation

A later survey, focused resurvey, decomposition amendment, profile change, or refinement change supersedes exactly the branch and work records whose input digests reference the old revision.

Supersession:

- cancels queued or running operations when possible;
- makes late results ineligible even when cancellation was not observed;
- recursively stales dependent refinement and build records;
- never deletes or mutates historical records.

Unaffected branches and results remain reusable.

## Phase B — Progressive build

### Parent policy grant and exact member admission

Starting `--through built` appends a plan-run grant covering:

- immutable discovery scope and current published branch set;
- run-policy digest;
- source, target, adapter, model, and profile constraints;
- advisory observation-set digest, empty when none is consumed;
- run bound `built`;
- publication policy and concurrency ceiling;
- compiled repair and decomposition budgets.

The parent grant does not pretend to cover unknown future artifacts. Before each build, the engine appends a member admission that binds:

- exact branch publication and refinement-manifest digests;
- exact gap inventory;
- protected verification inputs and oracle digests;
- exact direct-predecessor result digests and target candidate base;
- closed candidate-batch digest;
- run-policy digest, compiled member-policy digest, policy class, and parent grant.

Before admission, the engine retains the exact current manifest at `slices/<slice>/refinements/<digest>.yaml`. `refinement.yaml` remains the working current view; retained records make superseded candidate input auditable and are never treated as freshness state.

The RFC-99 run-policy class is `candidate`. RFC-102 extends the closed class enum with `low | moderate | high | critical` and records the selected autonomy risk profile in the compiled member-policy digest.

The scheduler records the closed same-base candidate batch before these member admissions. Only then may it claim work and prepare a writable workspace. Progressive build does not open an RFC-86 target wave.

### Gap policy

Unattended admission is stricter than manual execution:

- `[conflict]` blocks;
- `[unknown]` blocks;
- `[divergence]` remains visible and may proceed only when deterministic authority resolution selected a winner;
- no implicit, wildcard, inherited, or policy-authored waiver exists.

Manual `plan execute --waive` remains the only unknown-waiver surface. An unattended run stops and preserves the gap inventory for operator action.

### Candidate dependency frontiers

Closed execution requires accepted predecessors before dependent build. A progressive candidate run may instead satisfy a dependency with an exact successful predecessor `BuildRecord` under the same parent grant. A cross-target dependency is satisfied by that record plus its `slice.build.succeeded` fact; both digests enter the dependant work item, but one target's tree never composes into another.

The engine advances one candidate frontier per target through immutable **candidate batches**:

1. start from the recorded accepted CID;
2. choose one canonical bounded antichain of ready same-target leaves;
3. record the closed batch, then admit and build every member against that exact frontier CID;
4. compose only disjoint same-base member patches through RFC-96's deterministic composition kernel;
5. persist the resulting candidate CID, member records, and prior frontier digest;
6. use that CID as the exact base of the next batch.

A serial chain therefore advances `accepted → A result → B result → C result`; it never reapplies B or C to A's original base. Independent leaves in one batch share a base and compose only when their captured paths are disjoint. Overlap fails and parks the batch with the ordinary inert ownership/fan-in amendment proposal. RFC-99 never auto-applies it; unaffected branches may finish before the run reports the typed stop.

The progressive `BuildRecord` authorization anchor is a closed union:

```text
wave <wave-digest> | candidate-batch <batch-digest>
```

RFC-86/88 closed execution writes `wave`; progressive build writes `candidate-batch`. A later commit-capable run may reuse the result only after placing it in a current wave and revalidating its exact base, planning, policy, protected-input, dependency, and report digests.

RFC-99 amends RFC-86 D27 and RFC-90's wave-only envelope: `built` projects from a successful `BuildRecord` plus its matching `target.wave.opened | target.candidate-batch.recorded` fact. Merge readiness still requires a current target wave and accepted dependency frontier.

Candidate frontiers are inert execution values. They do not project `merged`, satisfy an external run's dependency, or mutate target state.

If an upstream branch, refinement, build, protected input, or policy changes, every descendant candidate frontier and build becomes stale through its digest chain.

### Build and verification

RFC-96 Phase B supplies code-patch composition, slice verification, domain rounds, protected-input closure, and the scheduler. RFC-90 supplies the engine-owned `build → verify ⇄ repair → review ⇄ repair` machine. RFC-106 may supply task decomposition and isolated writers inside a candidate when a slice is too large for one build call.

Candidate batches do not write `target.wave.opened`, project RFC-96 `frontier` or accepted `complete` domain rounds, or participate in target drain. Their batch and frontier records are the complete candidate-lineage substrate. Candidate-frontier verification may persist a non-authoritative candidate report; a commit-capable run creates current domain rounds and multi-member waves only after final closure.

The run policy may require stronger checks than the target's baseline but cannot:

- select commands or lower host sandbox policy;
- increase engine repair budgets;
- add or remove protected inputs after member admission;
- accept a blocking report;
- treat candidate-authored tests as protected assurance.

[RFC-97](rfc-97-native-verification.md) may provide host-attested profiles. Until required profiles exist, policy reports the actual model-assisted assurance and may stop rather than claim stronger evidence.

### Deferred commit

A progressive build result is never commit authority. Early batches may finish before authoring. After final closure recompiles the leaf and domain protected-input closures, the run reaches a terminal projection only when every surviving reachable leaf is either:

- built against its exact accepted or candidate frontier; or
- parked with a typed blocking reason.

It projects `succeeded` only when every surviving leaf is built; any parked leaf yields `stopped` after unaffected work drains.

If final closure changes a leaf planning projection, protected set, dependency identity, or required domain closure, its member admission and descendant candidate lineage become stale. Final candidate reports may be recomputed over the closed frontier without rerunning unchanged slice builds when their exact inputs remain valid.

After final closure, an operator may review the plan, refinement manifests, gaps, and candidate results, then invoke `plan execute`. Closed-plan authorization revalidates every candidate against the final plan, accepted target, dependency frontier, protected inputs, and current policy. Valid candidates may be reused; stale candidates rebuild.

Unattended accepted-CID mutation is deferred to [RFC-102](rfc-102-policy-gated-autonomy.md). Plan closure, completed builds, claims, elapsed time, or a passing model review never imply merge authority.

## Plan-run facts

RFC-99 replaces neither per-writer journals nor operation facts. It adds:

- `plan.run.started` — parent scope, policy, bound, publication mode, and budgets;
- `planning.branch.published` — immutable closed-branch identity;
- `planning.branch.superseded` — old and replacement branch identities;
- `plan.run.member-admitted` — exact build admission under a parent run;
- `target.candidate-batch.recorded` — closed same-base member set and authorization anchor;
- `target.candidate-frontier.recorded` — non-authoritative target lineage;
- `plan.run.stopped | succeeded` — terminal projection.

Every fact references schema-validated content. Claims select who performs admitted work; they never create admission or authority.

## Failure and restart

- Re-entry reconstructs ready work from branch records, refinement manifests, build records, candidate batches, candidate frontiers, and terminal facts.
- A partially written record is invisible; publication is atomic.
- A failed survey or decomposition blocks only its open domain and dependants.
- A failed refinement or build follows its ordinary bounded retry and stop behavior.
- A lost operation claim may duplicate computation but cannot duplicate an accepted result.
- Changing the run policy starts a new parent grant and invalidates its member admissions; it never upgrades the old run in place.

## Delivery

### Phase A requirements

- Add immutable branch publication and supersession DTOs and facts.
- Persist RFC-91's canonical slice-local planning projections in each branch.
- Keep `plan.yaml` complete-only; project progressive status from branch facts.
- Land RFC-96 Phase-A work items, bounded pool, cancellation, and local claims.
- Route published leaves through RFC-91 refinement without adding a second synthesis path.
- Add `emery plan run --publication progressive --through refined` and `/emery:run`.
- Persist time-to-first-branch and time-to-first-refinement telemetry.

### Phase B requirements

- Add closed run-policy resolution and digest recording.
- Add parent plan-run grants, retained refinement-manifest revisions, and exact member admissions.
- Extend the RFC-96 scheduler with candidate batches and candidate dependency frontiers.
- Add `--through built --policy <profile>` and reject policy-free unattended build.
- Extend the `BuildRecord` authorization anchor to `wave | candidate-batch`; reuse RFC-90 reports and RFC-96 same-base composition without writing accepted domain rounds or waves.
- Make every progressive result ineligible for merge until closed-plan revalidation.
- Persist speculative-work discard, reuse, and invalidation-cascade telemetry.

## Acceptance criteria

1. **Reviewed topology path.** Ordinary `plan author` publishes no refinement, and ordinary `plan refine` remains the explicit complete-plan next step.
2. **Progressive refinement.** While one scripted survey remains blocked, an independent closed branch publishes and reaches a fresh refinement manifest. No target event, workspace, wave, build record, or code grant occurs.
3. **Final equivalence.** Cap-one complete-plan refinement and cap-four progressive refinement produce the same canonical final plan and refinement inputs from the same scripted judgments.
4. **Supersession.** Replacing one branch invalidates exactly its refinement and dependent descendants; an independent branch remains fresh. A late result from the superseded branch cannot affect projection.
5. **Resume.** Process loss after branch publication, extraction, refinement, candidate-batch recording, member admission, and candidate-frontier recording resumes without duplicate authoritative facts.
6. **Unattended candidate build.** One progressive run reaches `built` without a human pause when every manifest is exact, the gap inventory is clean, policy admission succeeds, and required verification passes.
7. **Gap refusal.** A conflict or unknown stops unattended build before workspace preparation; no policy field or model answer can waive it.
8. **Candidate dependency chain.** A three-leaf serial chain advances three exact candidate bases, while two independent same-base leaves compose as one disjoint batch. Neither opens a target wave or advances the accepted CID. A cross-target dependency contributes identity and readiness without tree composition.
9. **No implicit commit.** A completed progressive run cannot merge, materialize a publication worktree, publish, or project accepted results. Closed `plan execute` reuses only candidates that pass full revalidation.
10. **Assurance truthfulness.** Operator output and facts distinguish model-assisted, protected, and host-attested checks and never label candidate-authored tests as protected.
11. **Quality and economics.** Repository gates pass, cap-one/cap-four candidate snapshots remain equivalent, and the live fixture reports first-refinement latency, first-build latency, discarded speculative work, and model cost when available.

## Rejected alternatives

- **Put progressive refinement inside `plan author`.** Manual author must retain its topology review boundary. `plan run` orchestrates the same operations without changing their individual contracts.
- **Wait for all of RFC-96 before refining progressively.** Refinement needs the work-item scheduler and read-heavy pool, not composition, domain rounds, or multi-member commit. Task graphs are RFC-106 and are not a refinement prerequisite.
- **Write a partially valid `plan.yaml`.** Immutable branch records project progressive readiness; `plan.yaml` remains the canonical complete-tree leaf projection.
- **Authorize unknown future manifests directly.** Parent policy grants constrain the run; member admission binds exact manifests before build.
- **Auto-waive unknowns.** Absence of information is not policy authority. Unattended work stops.
- **Require accepted predecessors for every progressive build.** That prevents a complete unattended candidate from being assembled without commit. Candidate frontiers preserve exact lineage without accepted-CID mutation.
- **Let passing builds imply merge authority.** Build evidence and accepted-state authority are separate. RFC-102 owns any future policy-gated merge.
- **Use a second progressive lifecycle.** Branches, work items, candidates, and grants are facts; slices retain the ordinary lifecycle.
