# Emery Remediation Plan

> Status: **The plan of record.** This document sequences the recovery from the state documented in [architecture-review.md](architecture-review.md) and [architecture-review-addendum.md](architecture-review-addendum.md) toward [product.md](product.md) via [target-architecture.md](target-architecture.md). It also carries the insights ledger — context from the review sessions that must survive them.
>
> Rule of the plan: feature work is frozen until Phase 3 is green. The walking skeleton (target-architecture §8) is the only definition of progress from Phase 2 onward.

## Artefact map

| Artefact | Role | Owner | State |
| --- | --- | --- | --- |
| [product.md](product.md) | Product yardstick: journey, verbs, concept budget, measured qualities, non-goals | Operator | Written — confirm targets |
| [decisions/](decisions/) | ADR log; ADR-0001…0006 are the decision gate | Operator decides; agents pre-fill evidence | Six proposed |
| [target-architecture.md](target-architecture.md) | The destination; agents implement against it | Human-owned, agent-drafted | Draft v0 → v1 in Phase 2 |
| [capability-conservation.md](capability-conservation.md) | Traceability contract: implemented capability → target section → acceptance evidence or explicit deletion | Human-owned through remediation | Draft — close in Phase 5 |
| [CONSTITUTION.md](../CONSTITUTION.md) | Standing invariants + mechanical enforcement | Human | Written |
| [architecture-review.md](architecture-review.md) | Findings + corrective cuts + acceptance scorecard | Review of record | Amended, absorbing the addendum's programme items |
| [architecture-review-addendum.md](architecture-review-addendum.md) | Second-pass findings (P10…, S12…, D12…, A8…, T6, C3…) | Authoritative finding list until folded | Standing; fold per its "How to merge" |
| This file | Sequence, anti-reversion strategy, insights ledger | Human | Written |

## Phase 0 — Containment (now; days)

Cut 0 of the standing review, as **widened by the addendum** (items 9–17 there). These need no gate decisions. Highest-urgency subset:

1. The original Cut 0 items 1–8 (P7 force-rebind guard, S7 drop tombstone, A4 fail-closed contracts validation, wasm example repair, pool cap 1 + timer wake, D11 fetch limits, T4 read-only vet, D2/D5/D6 containment).
2. ~~Park the uncommitted `plan correct` work~~ **Done (2026-08-17): removed.** The work had been committed (in `a7e3335f`, bundled with the programme artifacts) before the decision landed; it was then removed in a dedicated commit citing addendum P16/S25/S26 — a new durable constraint plane on an unscoped journal, constraint keyed to the wrong node, force-then-park cannot resume. Its design intent (durable, digest-bound operator guidance) survives properly as the `fix` surface in target-architecture §6.
3. Fail-closed I/O: gap inventory and merge-resume journal reads return `Err`, never empty (S13, S23 — the accepted-CID chain poisoning is permanent and has no repair verb).
4. Stop the claim-extras loss at extract time (A8): fail or persist extras rather than silently dropping `statement` / `criterion` / `replay-digest`. The core spec-mining function is degraded today; do not wait for Cut 4.
5. Disable the guest HTTP mutating catch-all (C3); job-scoped workspace discard + startup sweep (S37, D12); no fallback platform sets (A14/A15); probe runner accepts exit 2 as a typed stop (T6); regenerate the always-applied plugin rule from the router (P14).

In parallel (operator, ~half a day): confirm the targets and concept list in [product.md](product.md).

## Phase 1 — Decision gate (~1 week)

ADR-0002 is already **accepted** (Wasm-primary — operator decision, 2026-08-17); its spike scopes cost, not direction. Two spikes, then accept the remaining ADRs:

- **Store spike** (ADR-0001, 2–3 days): the transactional store behind `plan status` + one merge commit, crash-injected.
- **Component-seam spike** (ADR-0002, ~1 week): the scripted-model journey across the WIT seam in CI, offline — embedded engine guest + mock source/target components; metadata dispatch, one survey/extract round-trip, one build phase report, one `read_doc` MCP hop; byte-identical across two runs. This is the seam's first automated rung (T1) either way.

Then decide 0003 (one lifecycle), 0004 (conflict gate), 0005 (detached-only) on the recorded evidence — no spikes needed — and 0006 (rebuild shape) informed by the two spike measurements. Cuts 1–5 of the standing review are then **re-derived** against the accepted ADRs, not executed as written.

## Phase 2 — Target architecture v1 + enforcement (~1–2 weeks)

1. Rewrite [target-architecture.md](target-architecture.md) from draft v0 to v1 against the accepted ADRs. Resolve every `[ADR-NNNN]` marker; fix the module map, budgets, and the closed noun list.
2. **Land the fitness functions** (CONSTITUTION.md table) *before* the build starts: journey-test harness (red is fine initially — it is the Phase 3 exit criterion), route budget, LOC ratchet (`scripts/ratchet.toml` baseline from today's counts), layering test, seam-copy counter, gate tripwires for ADR-0004, prose budgets, ADR-required-paths check.
3. Specify the walking-skeleton script (target-architecture §8) as an executable test, scripted model, offline, across the component seam.
4. Close every entry in the [capability conservation ledger](capability-conservation.md) onto a target-architecture section and Phase 3/4 acceptance test, or an accepted ADR that explicitly deletes the capability. A green walking skeleton is not parity for a deferred capability.

## Phase 2b — Platform hardening (ADR-0002; ~4–8 weeks, may overlap Phase 3's store work)

The cost of Wasm-primary, paid up front rather than deferred as prose:

1. **The component-seam CI rung** — the Phase 1 spike promoted to a per-push gate (T1); the shipped seam is thereafter the tested seam.
2. **One admission mode**: embedded first-party registry + explicit exact-pin install; delete the native provider, `convert.rs`, the five-mode resolver, pull-on-miss/pull-latest, bare names, cache seeding (D5, D9-adjacent surface).
3. **Capability profiles** (D7): per-axis least privilege with malicious-fixture CI tests; scope the MCP shelf grant (addendum D14).
4. **Dispatch budgets** (D8): wall-clock via a real timer, fuel/epoch interruption, memory ceilings, output caps, guest-terminating cancellation.
5. **WIT as source of truth** (A1/A8/A16): the open claim record (core + extras) in the contract; one generated type family; delete the hand-maintained mirrors.
6. **D3 benchmark**: guest-side vs host-side workspace kernel under a representative build tree — decides placement (no default; the operator recorded no preference).

## Phase 3 — The walking skeleton (the build begins)

Per ADR-0006's shape (proposed: rebuild the spine, refactor the periphery). Milestone: **the §8 journey passes in CI**, including the crash-injection rung, across the component seam, over the intent source and the mock/omnia target. Nothing else counts as progress. Order within the phase: store → one-loop mining (`spec`) with the review document (P8) → executor (`build`) with waves-as-antichains and merge on the phase machine → `status` → `fix` (P9).

## Phase 4 — Widen, one axis at a time

Each increment lands only with the skeleton still green:

1. Sources: documentation, code, contracts, screenshots, captures (porting prose; extras honored per A8).
2. Second target (Vectis) behind `TargetContext`; the honest mock profile (A12) in the engine suite.
3. Architecture projection (the ADR-0003 replacement for the definition deliverable).
4. Publication as a drain-tail stage; archive.
5. Parallelism: raise the cap only after crash injection at cap 1 proves every stage idempotent (S32/S34 made membership bugs, not just races — they reproduce at cap one).

## Phase 5 — Reliability gate live + bookkeeping

1. Fix the eval runner to be a public-contract client (T6) — it cannot gate anything while `exit == 0` is its pass condition and the build back door is graded as a workflow.
2. Graded eval suite + telemetry against product.md's numbers; wire as a release gate (T5).
3. Fold the addendum into the standing review per its "How to merge" and delete it; update AGENTS.md to the shrunken navigational form (Invariant 2); revisit [platform.md](platform.md) against product.md and retire or re-scope the RFC programme accordingly.

## Working in place

The remediation happens in this repository — decided over a new repo for the git archaeology, the existing CI/release tooling, and because a lingering old repo is the zombie failure mode this project is specifically prone to: the two trees drift and you have a second product again. The real risk of in-place is different — agents read the existing ~101k lines as exemplars and faithfully reproduce the patterns being deleted. The discipline that contains it:

1. **New spine, new crates.** The target-architecture §4 modules (`store`, `engine`, …) land as *new* workspace members. The skeleton never lives inside `crates/change` or `crates/project`; it borrows from them by explicit port, reviewed against the target document.
2. **Quarantine the legacy tree.** Superseded crates are frozen: ratchet ceilings pinned at current LOC with only decreases allowed, a `LEGACY` note per crate, and an AGENTS.md line that agents must not use them as pattern references (the "map, not spec" header already establishes the posture).
3. **The journey test runs only against the new spine.** Legacy code can never silently become load-bearing for the acceptance path (the T3 pattern).
4. **Delete as parity lands.** Parity means the corresponding [capability ledger](capability-conservation.md) evidence is green, not merely that the walking skeleton passes. Deletion — not relocation — is how the old architecture leaves. A legacy crate surviving Phase 4 is a monthly-scorecard finding.

The one scenario where a new repo would have won — folding `emery-adapters` in as compiled-in crates — was conditioned on native-only and dissolved with ADR-0002's acceptance: under Wasm-primary the adapters repo stays a sibling producing components. Do not re-derive it.

## Anti-reversion strategy

Prose rules did not hold — agents faithfully extend whatever exists, and lab pressure deleted a designed gate (R3). Enforcement is therefore mechanical (the CONSTITUTION.md fitness functions) plus a ratchet, with prose only as explanation:

- **The ratchet converts aggregate drift into individual red builds.** Reversion was gradual; no single change was wrong. Per-crate LOC ceilings, route budgets, seam-copy counts, and prose budgets in a committed baseline file make each increment of drift a CI failure someone must justify with an ADR reference.
- **Gate tripwires make policy deletion loud.** One integration test per operator gate, named `adr_NNNN_*`. Deleting the conflict gate means deleting a test that names its decision record — impossible to mistake for a bug fix.
- **The journey test makes composition failure immediate.** T3 (the wasm example silently became an illegal workflow) and the S2/S32 divergence classes are exactly what a permanently-green end-to-end journey prevents.
- **The monthly scorecard** (CONSTITUTION.md ritual) walks the review's acceptance list and ratchet deltas — 30 minutes, recorded as a dated note.

## Insights ledger

Context from the review sessions (2026-08-17) that must not be lost:

1. **Scale datum.** Engine ~101k lines of Rust (project 33.9k, change 17.8k, slice 15k, system 6.7k) + ~27k adapter prose + ~12k adapter Rust. Omnia — the entire runtime platform, including twelve WASI host-capability crates, guest SDK, macros, conformance suite — is ~29.8k. A workflow CLI 3.4× its runtime is a scope symptom before an engineering one. Omnia's coherence came from being built to a settled architecture; repair does not produce that property.
2. **The yardstick error.** The first review audited the implementation against platform.md and never audited platform.md against the product. Findings-driven repair without a destination converges on the same system, hardened. Hence: product.md first, ADRs second, architecture third, cuts re-derived last.
3. **The dissolution logic.** One transactional store per change home dissolves (not fixes) S1–S3, S6–S8, S10, S11, D9, D10 and the addendum's reducer-class findings — but *not* the missing types and seams (authoring generation, CorrectionTarget, SurveyReceipt, claim family, wave antichain), which must be designed regardless. Do not let the store decision masquerade as the whole fix.
4. **Native-only is the highest-leverage subtraction** (~a third of the blocker list) — but A8 (claim-extras drop) lives in the native converter too, D14 (unscoped MCP grant) survives on the native shelf, and the isolation requirement is deferred, not deleted. *(Superseded in part by insight 11: the subtraction actually taken was the native provider and the resolution matrix, not the platform.)*
5. **A8 is the quiet product killer.** The seam silently drops the structured claim fields (`statement`, `criterion`, `replay-digest`) that first-party extract prompts require and synthesis prefers. Eval "worked" via the `synopsis` fallback, so the degradation of the core spec-mining function was invisible. Lesson: eval greenness is not evidence the designed data path is exercised.
6. **The lab shaped the product** (R3/P3/T6). Auto-deferral replaced a designed operator gate so unattended eval could finish; the probe runner cannot represent a typed stop (exit 2 fails the case) and grades a build back door as a workflow. The measurement instrument must be a client of the public contract or it will keep selecting product shortcuts.
7. **`plan correct` was the R-pattern happening live**: a new durable authority plane, unscoped, with the constraint keyed to the wrong node (S25) and a resume path force can break (S26) — landed while the review warning against new planes stood. The process rules exist for exactly this. *(Resolved 2026-08-17: removed in a dedicated commit; the design intent returns as `fix` in target-architecture §6.)*
8. **Cap-one is not a soundness proof.** S32 (wave = ready batch, not antichain) and S34 (refine retracts frozen waves) are membership/isolation design bugs that reproduce at cap one.
9. **Development-loop causes** (R1–R4): RFC-at-a-time with no walking skeleton; AGENTS.md as load-bearing spec compounding prose and code sprawl; policy changes without decision records; addition-only scope because agents don't push back. The countermeasures are the constitution's invariants — mechanically enforced, because prose did not hold.
10. **Second-pass yield stayed high** (~65 new findings after a ~45-finding first pass), which is itself evidence for rebuilding the spine over refactoring it (ADR-0006): finding density did not fall with a second look at the same regions.
11. **Wasm is foundational — an operator decision, not an engineering preference** (ADR-0002, accepted 2026-08-17). Two requirements predate this codebase and killed a prior non-Wasm generation: adapters must be addable dynamically without rebuilding the host, and one core must run as a desktop CLI and a web service. Native plugins, subprocess adapters, containers, and embedded scripting each fail one of them. The review's real finding was the *duality* (the tested seam was not the shipped seam) and premature *distribution* plumbing — so the subtraction taken deletes the native provider and the five-mode resolver, keeps and hardens the component seam, and re-prices D7/D8/T1 as scheduled platform features rather than deferred costs. The operator recorded **no preference on guest-side vs host-side workspace kernel** — the D3 benchmark decides placement on evidence.
