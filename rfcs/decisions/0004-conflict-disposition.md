# ADR-0004: Conflict disposition — operator gate vs auto-defer

> Status: **Accepted** (operator decision, 2026-08-19) — Option D as re-scoped 2026-08-18 for the spec-generator programme ([ADR-0008](0008-spec-generator-programme.md)).
> Date: 2026-08-17

## Context

RFC-86a's build gate auto-defers every open `[unknown]` **and** `[conflict]` row with a synthesized reason carrying no requirement heading ([architecture-review.md](../architecture-review.md) P3). `[conflict]` — the strongest signal the specification is wrong — gets the same silent treatment as a missing detail. The original design had `emery plan defer` and a `strict | defer` policy; both were deleted after landing so unattended eval could finish (the R3 process failure). The [addendum](../architecture-review-addendum.md) adds S15: deferrals are immortal, unscoped, and irrevocable — the debt plane has no lifecycle regardless of policy.

[ADR-0008](0008-spec-generator-programme.md) ships the specification and does not build. There is no build gate, so the original A/B/C fight (who may enter build scope) is not a live product decision. The live question is how disagreement appears in the spec the reviewer reads.

## Options

- **A. Split by severity.** `[unknown]` may auto-defer (with the requirement heading in the reason); `[conflict]` requires an explicit operator disposition before the slice enters build scope. Unattended eval gets a lab flag.
- **B. Keep universal auto-defer** and stop calling specification review a gate.
- **C. Strict for both** — every open row blocks build until dispositioned.
- **D. Inline, no gate (this programme).** Authority precedence (`intent` > `documentation` > `behaviour`) resolves what it can and records the rest as `[divergence]` or `[conflict]` on the spec. `[unknown]` stays `[unknown]`. Nothing is auto-deferred. Nothing blocks a verb that does not exist yet. Disposition-before-build is a build-programme question (then A is the standing preference).

## Decision

**Option D for this programme.** A conflict is visible in the reviewable specification. Universal auto-defer (B) made the review claim false and is not revived. Options A and C describe a build gate this programme does not have.

Regardless: the conservation half of RFC-86a (typed debt that is not silently invented by a target) is design intent for the annex, not a generator feature.

## Deletions

The synthesized-reason string format (epoch identity encoded in prose at second precision); the fiction that `Ready` is a review gate; auto-deferral as a live policy. Concept-count effect: `disposition` is not an operator verb in this programme — conflict is a property of the spec.

## Consequences

An unattended `specify` over a conflicted estate still produces a spec, with conflicts inline. Eval grades whether conflicts are *visible*, not whether a gate stopped the run. When the build programme opens, Option A is the starting proposal and needs its own acceptance — it is not smuggled in by this ADR.

## Revisit trigger

Opening the build programme (a new ADR) reopens A vs C for entry into build scope. Independently: measured evidence that reviewers ignore inline `[conflict]` tags would argue the generator's review claim is theater and needs a stronger gate even before build.
