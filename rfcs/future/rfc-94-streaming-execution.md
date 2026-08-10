# RFC-94: Streaming Execution

> Status: Future — step 9 of the platform-migration series, scale track ([platform.md](../platform.md)). Owns partial branch publication, streaming readiness and invalidation, build-without-commit authority, generalized plan-run grants, and deferred commit. Depends on completed [RFC-88](../rfc-88-detached-changes.md), [RFC-91](../rfc-91-staged-refinement.md), and [RFC-92](../rfc-92-concurrent-execution.md); distribution through [RFC-93](../rfc-93-distributed-execution.md) is optional.

## Intent

Run ready branches while survey and decomposition continue elsewhere, without allowing accepted-CID mutation before the resulting closed plan and refinement set receive operator review.

Time to first build stops being bounded by complete-tree closure. Commit authority remains bounded by a later closed-plan gesture.

## Inherited substrate

This RFC generalizes settled contracts rather than moving streaming policy into them:

- [RFC-88](../rfc-88-detached-changes.md) retains immutable lead and decomposition revisions fine-grained enough for closed branches to publish independently.
- [RFC-91](../rfc-91-staged-refinement.md) defines the serial closed-plan baseline: reviewed refinement manifests, exact bundle digests, execute-time coverage, and wave-time target bases.
- [RFC-92](../rfc-92-concurrent-execution.md) defines `(slice, phase, input-digest)` work items, phase readiness, local operation claims, bounded antichains, and multi-member waves.
- [RFC-93](../rfc-93-distributed-execution.md) may transport the same work and values across nodes without changing streaming authority.

`emery plan refine` remains the non-streaming specs-first stage. It creates no code-work grant. Streaming introduces the second privileged authority mode that justifies generalizing RFC-91's closed `plan.execute.started` event.

## Decisions the draft must close

### Generalized plan-run authority

Define the replacement or extension for `plan.execute.started` only after the second authority mode exists. The grant contract must distinguish:

- reviewed closed-plan build and commit authority;
- streaming build authority over published branches;
- a later closed-plan commit gesture covering exact waiting results.

The draft owns the event name, exact ceiling vocabulary, coverage schema, and compatibility cut. It must not infer commit authority from completed builds, plan closure, claims, or elapsed time.

### Streaming-discovery coverage

Define the immutable discovery scope each streaming grant binds, including:

- published branch and leaf revision digests;
- source, target, adapter, profile, and refinement-manifest identities;
- how later branch closures extend build coverage;
- whether extension requires another operator gesture.

Unknown future refinement cannot be treated as reviewed. A branch becomes buildable only when its exact refinement manifest and gap policy are covered.

### Partial publication

Define which decomposition states may publish, how partial lead/decomposition/plan projections represent unsurveyed and open branches, and how validation distinguishes:

- not yet surveyed;
- published and fresh;
- superseded;
- structurally invalid.

Only closed branches and profile-gated leaves may enter the RFC-92 scheduler.

### Deferred commit

A streaming-built wave remains inert until a later closed-plan commit-capable grant covers:

- the final reviewed planning revisions;
- every member refinement digest and waiver;
- the wave's build authorization and result records;
- the current accepted target and dependency frontier.

The later gesture revalidates; it never upgrades the earlier streaming grant retroactively.

### Gap timing

The requirement-level gap policy remains unchanged: conflicts block, unknowns require explicit waivers, and divergences are informational.

The draft must decide when speculative build becomes visible relative to human specification review. It may not weaken RFC-91's rule for ordinary closed execution or turn a streaming result into commit authority.

### Invalidation

A later survey, focused resurvey, decomposition amendment, or refinement change invalidates exactly the queued or completed work whose input digest references the superseded revision.

Historical records remain immutable. Invalidation makes results ineligible; it does not delete or mutate them.

### Telemetry

Record time to first refined leaf, first completed build, and first accepted result; speculative-work discard rate; branch-revision churn; and plan-staleness cost. These measurements size policy and budgets but never alter lifecycle or authority.

## Acceptance criteria

1. A fixture surveys one branch while another refines and an independent third leaf builds through RFC-92 work items.
2. Streaming build authority cannot commit a target wave. A later reviewed closed-plan gesture may commit only covered, revalidated results.
3. A superseded branch revision invalidates exactly its digest-bound refinement, build, and dependent descendants; unaffected work remains reusable.
4. Partial projections distinguish unsurveyed, open, published, stale, and invalid branches without treating absence as success.
5. Streaming preserves the requirement gap policy and never creates implicit waivers.
6. Cap-one and concurrent local execution produce the same accepted results from the same surviving records.
7. The operator-invoked streaming evaluation fixture reports time-to-first-result and discarded speculative work, and all repository quality gates pass.

## Evidence posture

Streaming is a named series step, but its grant and publication schemas remain deliberately uncommitted until RFC-88's revisions, RFC-91's serial review boundary, and RFC-92's local scheduler are stable. Designing those wire shapes here avoids forcing speculative streaming vocabulary into earlier independently deliverable RFCs.
