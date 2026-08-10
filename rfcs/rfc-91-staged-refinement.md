# RFC-91: Staged Refinement

> Status: Draft. Adds `plan refine`, reviewed refinement manifests, and wave-time target bases. Builds on [RFC-86](rfc-86-change-facts.md), [RFC-87](rfc-87-working-trees.md), and [RFC-90](rfc-90-build-verification.md); [RFC-92](rfc-92-concurrent-execution.md), [RFC-93](rfc-93-distributed-execution.md), and [RFC-94](future/rfc-94-streaming-execution.md) own concurrency, distributed claims, and streaming authority.

## Intent

*Introduce a workflow step that generates every specification before code generation begins.*

```text
plan author → plan refine → plan execute → plan archive
```

`plan refine` drains refinement for a closed plan and stops. `plan execute` accepts only reviewed, fresh refinement manifests; it never synthesizes a replacement and immediately builds it.

## Problem

### The public workflow has no specification boundary

The plan-centric cut removed per-slice refine breakouts and made `plan execute` the only driver of `refine → build → merge`. Operators can stop after topology review or after generated software, but not after complete specification generation.

An execute stop flag would expose the phase without fixing its inputs or authority. A specs-only run must neither authorize code work nor leave execute free to replace reviewed artifacts.

### Execution dependencies block complete refinement

The current scheduler admits a pending entry only after every `depends-on` entry projects `done`, which requires merge. A specs-only run can therefore refine only the initial dependency frontier.

Refinement and build need different dependency predicates:

- dependent refinement waits for predecessor refinement;
- dependent build waits for predecessor acceptance.

This lets a dependency chain produce all specifications before any software.

### Refinement captures the build base too early

Refinement currently freezes the product tree into `base.yaml.target-base`; build later prepares from that snapshot. For a dependent slice, the correct build base is the accepted result of its predecessors, which does not exist when the plan is refined.

Refinement can close source, planning, guidance, baseline-specification, dependency-refinement, and generated-artifact inputs. The target code base closes only when its wave opens. Those identities must be recorded separately.

### Execute covers too little reviewed input

`plan.execute.started` currently covers the plan and each leaf's specs tree or `refine-under-epoch`. Target generation also consumes `proposal.md`, `design.md`, `tasks.md`, and adapter-declared additional inputs.

Execute must cover the complete reviewed refinement bundle. `refine-under-epoch` must disappear: unknown future refinement cannot count as reviewed.

## Terms

- A **refinement manifest** is `.emery/slices/<slice>/refinement.yaml`, the canonical record of one successful refinement's exact inputs and complete output bundle.
- A **refinement bundle** is every slice artifact the assembled target build request may consume, with path, kind, and content digest.
- A **refinement digest** is the content digest of the canonical manifest.
- **Accepted** means the predecessor's target wave committed and advanced the target's accepted CID.

## Operator flow

### Author — topology only

`emery plan author` continues to survey sources, reconcile or recursively decompose leads, and project terminal leaves into `plan.yaml`. It does not extract Evidence or synthesize slice artifacts.

The operator reviews topology before paying for extraction and synthesis.

### Refine — complete specifications, no software

`emery plan refine [--slice <slice>...]` serially drains eligible refinement work:

1. select an in-scope leaf whose predecessor refinement manifests are fresh;
2. extract every bound source;
3. synthesize and validate the slice artifacts;
4. atomically write its refinement manifest;
5. continue until every selected leaf is fresh or a typed stop occurs.

Without selectors, the command targets every in-scope leaf. Selectors include the stale or missing predecessor closure needed to make the selected work coherent.

Successful refinement may contain `[unknown]`, `[conflict]`, or `[divergence]`. Those are review outputs, not refinement failures. The command persists them and points the operator to `emery plan gaps`.

No target build operation, product workspace preparation, target wave, `BuildRecord`, merge gate, or accepted-CID mutation may occur during `plan refine`.

### Review — human-owned

The operator reviews `proposal.md`, `design.md`, `tasks.md`, per-domain specs, provenance, and `plan gaps`. The engine adds no approval file, checklist, or projected `approved` state.

Changing a covered input or output makes the manifest stale. The operator re-runs `plan refine`; execute never silently re-refines and then builds an unreviewed replacement.

### Execute — build and commit reviewed refinements

`emery plan execute` requires a fresh refinement manifest for every in-scope leaf it may build. At start it appends `plan.execute.started` covering the exact plan and refinement digests plus any explicit unknown waivers.

It then drains the existing engine-owned build and merge phases:

```text
target build → verify ⇄ repair → review ⇄ repair → target-wave merge
```

If a manifest is missing or stale, execute fails before opening a workspace or target wave and points to `emery plan refine`.

`--waive <slice>/<req> --reason <reason>` remains execute-only because a waiver authorizes build across a reviewed unknown.

### Archive — unchanged

After publication, `emery plan archive` verifies its existing completion conditions and archives the change.

## Decisions

### D1 — `plan refine` is a first-class orchestration stage

The public workflow is `plan author → plan refine → plan execute → plan archive`. `/emery:refine` is an ultrathin invoke-and-relay wrapper over `emery plan refine`.

`plan author` remains topology-only. Refinement is not folded into author because the topology review seam remains valuable.

This supersedes RFC-86 D14 and D26. The earlier rejection preserved a three-verb surface around per-slice breakouts that no longer exist.

### D2 — Refinement adds no plan lifecycle state

Completion remains per leaf. `Ready` is a read-only projection: every in-scope leaf has a fresh refinement manifest and the clean gap policy passes.

The plan keeps no mutable lifecycle field and gains no `approved` rung. A successful `plan refine` means its requested work set reached the bound; it does not stamp a global Refined state.

### D3 — `depends-on` has phase-relative satisfaction

The existing acyclic graph remains the ordering graph:

- dependent refinement requires predecessor **refined**;
- dependent build requires predecessor **accepted**;
- dependent merge revalidates the accepted dependency frontier.

The predecessor's refinement digest and readable artifact roots enter dependent synthesis as ordered change-local context. This does not make predecessor prose Source Evidence or alter artifact authority.

No second dependency graph is introduced until retained plans demonstrate that specification ordering and execution ordering differ in practice.

### D4 — One manifest covers complete generation intent

After successful synthesis and validation, the engine atomically writes one canonical refinement manifest:

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
bundle:
  - path: proposal.md
    kind: proposal
    digest: sha256:…
  - path: design.md
    kind: design
    digest: sha256:…
  - path: tasks.md
    kind: tasks
    digest: sha256:…
  - path: specs/orders/spec.md
    kind: spec
    digest: sha256:…
```

The schema is generated from a Rust DTO and rejects unknown fields. `bundle` is assembled from the same canonical input declaration used by the target build request, including present adapter-declared additional inputs. A missing required input prevents successful refinement.

The artifact files remain the human-readable working view. Freshness recomputes the manifest inputs and bundle against those files; no duplicate `refinements/<digest>.yaml` history or latest-record projection is introduced in this cut. The execute fact and build attempt retain the digest identities they consumed.

`baseline-specs` identifies the immutable baseline read by synthesis. Target-wave commits from the same covered plan advance the accepted target but do not stale unbuilt sibling manifests merely because the live baseline moved. Merge continues to use the recorded three-way baseline. Drift outside the covered plan remains a typed validation failure.

### D5 — Execute retains the closed-plan authorization event

`plan refine` writes planning artifacts and creates no code-work grant. `plan.execute.started` remains the privileged-start fact.

Its closed-plan coverage changes from per-leaf `existing | refine-under-epoch` spec coverage to exact per-leaf refinement digests. Optional unknown waivers remain nested on that fact.

`refine-under-epoch` is removed. Execute cannot authorize a refinement that does not yet exist, because doing so would erase the human review boundary.

Generic `plan.run.started` grants, build-only authority, and deferred commit authorization belong to RFC-94, where a second authority mode requires them.

### D6 — The target base is selected at wave open

The refinement manifest carries no `target-base`.

Closed in-place execution selects the current product snapshot when the wave opens. RFC-88 detached execution selects the target's current accepted CID. The wave persists that value before any writable workspace is prepared, and `BuildRecord.base` continues to bind it.

Dependent waves therefore start from accepted predecessor results without re-refining reviewed specifications.

This supersedes RFC-86 D25's refine-time target-base rule and preserves RFC-87's invariant that no build consumes an unrecorded ambient base.

### D7 — This cut uses a serial refinement drain

`plan refine` has a refinement-specific deterministic selector over the closed plan. It does not call `advance_next`, append `plan.entry.advanced`, or convert refined leaves into the execute loop's active cursor.

The first implementation runs under the existing guest marker and stops on the first failed refinement. Re-entry skips fresh manifests and resumes missing or stale work.

RFC-92 owns the generic phase-work-item scheduler, concurrent work frontiers, and local operation-scoped claims. RFC-93 adds distributed offers, leases, ownership generations, and stale-claim rejection.

### D8 — Status exposes the review seam

`plan status` remains read-only:

- missing or stale manifests point to `/emery:refine`;
- fresh manifests with conflicts point to input correction and re-refinement;
- fresh manifests with unknowns point to correction or explicit execute waivers;
- clean refinement points to `/emery:execute`;
- executed entries retain the existing build, merge, stop, and drained projections.

`Authorized` continues to project from a fresh covering `plan.execute.started`. Refinement alone never presents code work as authorized.

### D9 — This is a hard workflow cut

Pre-1.0, staged refinement removes:

- execute-time implicit refinement;
- `refine-under-epoch`;
- refine-time `target-base`;
- spec-only execute coverage.

No compatibility aliases, dual coverage projections, or fallback reads of the old pin shape survive. Existing projects re-init or recreate active changes across the cut.

This amends RFC-86 D6, D8, D12, D14, D22, D25, and D26, plus RFC-88 D7 and D8. RFC-90's target build phase machine is unchanged.

## Failure and restart

- An interrupted or failed refinement writes no successful manifest for that attempt.
- Re-running `plan refine` schedules only missing, stale, or explicitly selected work; fresh siblings are not repeated.
- Re-running `plan execute` reuses valid build records but refuses missing or stale refinement.
- A changed predecessor refinement invalidates dependent manifests through the recorded dependency digest.
- A build or merge failure retains RFC-90's stop and retry behavior.

## Implementation requirements

- Add `emery plan refine [--slice <slice>...]` and the ultrathin `/emery:refine` wrapper.
- Add a serial refinement-drain orchestration with phase-relative dependency selection and selector closure.
- Add the typed `refinement.yaml` schema, atomic persistence, freshness projection, and canonical complete bundle manifest.
- Pass ordered predecessor refinement digests and artifact roots into dependent synthesis.
- Remove `target-base` from refine-time pin assembly and product-tree freeze from refinement.
- Select and persist the target base when opening a wave.
- Replace wave `MemberInputs.spec` with the refinement digest while preserving nested spec identity needed by merge.
- Change `plan.execute.started` coverage to exact refinement digests and remove `refine-under-epoch`.
- Make execute reject missing or stale refinement before any build workspace or target wave is created.
- Update plan status, gaps hints, CLI output shapes, skills, workflow standards, and platform-series documentation.
- Keep native and Wasm providers on one typed transport contract; add no source or target WIT operation.

## Acceptance criteria

1. A three-leaf dependency chain reaches fresh manifests for all leaves through one serial `plan refine`, in topological order, with no target operation, wave, `BuildRecord`, product workspace freeze, or product-code change.
2. `plan refine --slice <leaf>` includes only the stale or missing predecessor closure needed by that leaf and skips fresh siblings.
3. A dependent manifest binds its predecessor's refinement digest; changing the predecessor invalidates the dependent.
4. Every path consumed by the assembled target build request appears in the refinement bundle. Changing any covered path makes execute refuse before wave open.
5. Execute over an unrefined or stale leaf returns a typed refinement-required result and never auto-refines.
6. A dependent build opens only after predecessor acceptance and records the then-current target CID as its wave base.
7. Refinement succeeds with typed gaps. Conflicts remain non-waiveable; unknown waivers remain execute-only and bind to the covered refinement digest.
8. An interrupted refinement writes no successful manifest; restart neither treats partial artifacts as fresh nor repeats valid sibling refinements.
9. Existing RFC-90 success, repair, review-remediation, failure, and attempt-abandonment fixtures pass without target WIT changes.
10. `cargo make ci`, the wasm32 engine compile-check, and native integration suites pass.

## Rejected alternatives

- `**plan execute --until refined**` — preserves code authorization and implicit-refinement semantics for a specs-only request.
- **Fold refinement into author** — removes topology review and pays extraction and synthesis cost before the operator accepts decomposition.
- **Keep implicit refine inside execute** — permits newly generated specifications to enter build without human review.
- **Keep refine-time target-base** — makes dependent builds consume a pre-predecessor base or forces ordinary re-refinement after code generation.
- **Compare refinement baselines to every plan-local merge** — would stale reviewed dependent manifests as their predecessors commit.
- **Persist immutable refinement-record history now** — the canonical manifest plus digest-bound execute and build records provide the closed-plan review fence; RFC-94 may add retained revisions when partial publication requires them.
- **Replace the scheduler in this RFC** — serial staged refinement needs a bounded refinement selector, not concurrent phase work items or distributed claim semantics.
- **Replace `plan.execute.started` with a generic grant now** — closed execution has one authority mode; RFC-94 owns the second mode that justifies generalization.
- **Treat the plan as globally Refined** — creates mutable barrier state instead of projecting freshness from per-leaf manifests.

