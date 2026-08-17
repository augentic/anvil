# ADR-0004: Conflict disposition — operator gate vs auto-defer

> Status: Proposed — pending operator acceptance
> Date: 2026-08-17

## Context

RFC-86a's build gate auto-defers every open `[unknown]` **and** `[conflict]` row with a synthesized reason carrying no requirement heading ([architecture-review.md](../architecture-review.md) P3). `[conflict]` — the strongest signal the specification is wrong — gets the same silent treatment as a missing detail. The original design had `emery plan defer` and a `strict | defer` policy; both were deleted after landing so unattended eval could finish (the R3 process failure). The [addendum](../architecture-review-addendum.md) adds S15: deferrals are immortal, unscoped, and irrevocable — the debt plane has no lifecycle regardless of policy.

## Options

- **A. Split by severity.** `[unknown]` may auto-defer (with the requirement heading in the reason); `[conflict]` requires an explicit operator disposition before the slice enters build scope. Unattended eval gets a lab flag.
- **B. Keep universal auto-defer** and stop calling specification review a gate.
- **C. Strict for both** — every open row blocks build until dispositioned.

## Decision (proposed)

**Option A.** It matches the product definition: the reviewable specification is the deliverable, and a conflict is precisely the case where the deliverable is not yet reviewable. Universal strictness (C) reintroduces the friction that motivated the deletion; universal auto-defer (B) makes the review claim false.

Regardless of option: deferral facts are scoped to the authorization (generation + token per ADR-0001's outcome), carry the requirement heading, and gain a lapse/reopen lifecycle (addendum S15). The conservation half of RFC-86a (deferred rows leave build scope and become typed debt) is retained unchanged — it was always sound.

## Deletions

The synthesized-reason string format (epoch identity encoded in prose at second precision); the fiction that `Ready` is a review gate. Concept-count effect: neutral — `disposition` already exists as a noun; it becomes honest.

## Consequences

An unattended run over a conflicted estate stops at the gate unless the lab flag is set. That is the point.

## Revisit trigger

Measured evidence (T5 telemetry) that conflict dispositions are rubber-stamped >90% of the time with no spec change — which would argue the gate is theater and B is honest.
