# RFC-94: Streaming Execution

> Status: Future — step 9 of the platform-migration series, scale track ([platform.md](../platform.md)). [RFC-91](../rfc-91-staged-refinement.md) owns the prerequisite phase work items, refinement records, operation claims, and authority ceilings; draft this RFC once those contracts, RFC-88's decomposition artifacts, and [RFC-92](../rfc-92-concurrent-execution.md)'s concurrent pool and multi-member waves are stable.
>
> Owns: activation of RFC-91's `streaming-discovery` grant; partial publication of closed domain branches and ready leaves while survey and decomposition continue; the scheduler's streaming readiness predicate; and the deferred-commit rule that keeps accepted-CID mutation behind a later closed-plan ceiling-`commit` grant.
>
> Does not own: phase work-item, claim, refinement-record, or authority-ceiling vocabulary ([RFC-91](../rfc-91-staged-refinement.md)); concurrent pool or convergence semantics ([RFC-92](../rfc-92-concurrent-execution.md)); distribution ([RFC-93](../rfc-93-distributed-execution.md) is orthogonal — single-node streaming does not wait on it); publication ([RFC-89](../rfc-89-publication-sets.md)).
>
> Depends on completed [RFC-88](../rfc-88-detached-changes.md), [RFC-91](../rfc-91-staged-refinement.md), and [RFC-92](../rfc-92-concurrent-execution.md).

## Intent

Run a change asynchronously end to end: refine and build ready leaves while survey and decomposition of the rest of the estate continue, awaiting dependencies through the ordinary readiness machinery, without weakening the authority model. Time-to-first-result stops being bounded by complete-tree closure; commit authority stays bounded by operator review.

## Reserved substrate

The series reserved this mode deliberately; this RFC activates it rather than inventing it:

- [RFC-86 §"Authorization epoch and coverage"](../rfc-86-change-facts.md#authorization-epoch-and-coverage) reserved streaming coverage and fixed its hard constraint: streaming work may refine and build ready leaves while survey continues, but **cannot commit a target wave**.
- [RFC-91](../rfc-91-staged-refinement.md) replaces command-shaped epoch coverage with plan-run grants carrying `refine | build | commit` ceilings. Streaming uses `streaming-discovery` with ceiling `build`; accepted-CID mutation requires a later `closed-plan` grant with ceiling `commit`.
- [RFC-88](../rfc-88-detached-changes.md) keeps lead and decomposition revisions immutable and deliberately finer-grained than its own complete-tree publication policy, so closed domain branches and ready leaves can publish early while every build stays bound to the exact revisions it saw.
- [platform.md §"Authority: grant, claim, and input fence"](../platform.md#authority-grant-claim-and-input-fence) records separate build authorization (the wave manifest) and commit authorization (the committed fact) for exactly this reason.
- RFC-91's phase-relative `depends-on` readiness supplies the refine/build distinction; [RFC-92 D11](../rfc-92-concurrent-execution.md)'s bounded antichains and shared pool supply concurrent dispatch. Streaming adds "not yet surveyed / not yet decomposed" as one more form of unreadiness.

## What the draft must decide

- **Grant payload.** The `streaming-discovery` shape within RFC-91's plan-run grant: the immutable discovery scope it binds, per-branch closure digests, and how later branch closures extend ceiling-`build` coverage without a second operator gesture.
- **Partial publication.** Which decomposition states may publish (closed branches only, profile-gated leaves), what a partially published `plan.yaml` / `decomposition.yaml` pair looks like, and how validation distinguishes "not yet surveyed" from "drifted".
- **The closing gesture.** How a streaming run converges to a reviewed closed plan whose ceiling-`commit` grant authorizes the deferred wave commits — the operator review seam streaming must preserve, not delete.
- **Gap-gate timing.** The gap gate stays per requirement before build; streaming decides only when refinement output becomes visible for review relative to speculative builds.
- **Invalidation.** A later survey or focused re-survey that changes a published branch invalidates exactly the speculative results bound to the superseded revisions — pins and digests already make that set deterministic.
- **Telemetry.** Time-to-first-accepted-result, speculative-work discard rate, and plan-staleness measurements that size the cut and validate it against the complete-tree baseline.

## Degenerate rung

`emery plan refine` is the single-writer, non-streaming rung of the same ladder: it drains every in-scope refinement work item in topological waves under a ceiling-`refine` grant and stops before generation. A later execute covers the resulting refinement-record digests and builds under a ceiling-`commit` closed-plan grant. This RFC reuses that scheduler and record substrate while changing publication timing and the grant ceiling; it does not inherit an execute stop flag.

## Evidence posture

The direction is committed — this is a named series step, not an evidence-triggered maybe. Measurements ([RFC-88](../rfc-88-detached-changes.md)'s authoring-duration and time-to-first-executable-leaf telemetry, [RFC-92 D13](../rfc-92-concurrent-execution.md)'s outcome projections) size and sequence the cut; they no longer gate whether it exists.
