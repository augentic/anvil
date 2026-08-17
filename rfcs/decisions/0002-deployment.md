# ADR-0002: Deployment — native-only vs dual native/Wasm

> Status: Proposed — pending spike and operator acceptance
> Date: 2026-08-17

## Context

Two full providers implement the same capability traits; the production Wasm seam is the one path no automated test runs; adapter isolation is descriptive, not enforced; component execution has no resource or liveness budget ([architecture-review.md](../architecture-review.md) D1, D5–D8, T1, A1). The product definition ([product.md](../product.md)) contains no third-party adapter requirement: both adapter families are first-party, in one sibling repository, maintained by the same team. The component platform's bill is being paid without its benefit.

## Options

- **A. Native-only now.** Adapters compile in as crates behind the existing `adapter::Source` / `adapter::Target` traits. The WIT contract survives as the documented shape for a future component seam.
- **B. Keep dual deployment**, adding a scripted-model Wasm CI smoke rung, per-axis least-privilege capability profiles, and host-dispatch resource budgets (D1, D7, D8 recommendations).

## Decision (proposed)

**Option A.** Deletes `guest` (~1.2k lines), the launcher install/resolver matrix, `wasi-exec` and `wasi-vcs` trampolines, four of five hand-maintained DTO families (A1 mostly evaporates), D5 (resolver modes), D6 (MCP hop), D7 (isolation), D8 (budgets) — and T1, because the untested seam ceases to exist.

**Caveats the spike must confirm** (from the [addendum](../architecture-review-addendum.md)):

- **A8 is not fixed by native-only.** The claim-extras drop lives in `crates/native/src/convert.rs` too. The one-Claim-family fix (A8/A16) is required on either option.
- **D14 remains.** The unscoped MCP reference grant is a prompt-control-plane problem on the native shelf as well.
- The isolation *requirement* does not vanish; it is deferred until a third-party adapter exists, at which point the trait seam is the reintroduction point.

## Deletions

`crates/guest`, `crates/wasi-exec`, `crates/wasi-vcs` (kernels re-homed in-process), `launcher::install` and the resolver matrix, the guest HTTP catch-all (addendum C3), the store/cache/pull-on-miss surface, `adapter add/upgrade` component plumbing. Concept-count effect: removes adapter identity/versioning/store nouns from the operator surface.

## Consequences

Adapter releases couple to engine releases (acceptable: same team, same cadence). Isolation of a defective adapter is process-level only. Omnia (the runtime) loses its flagship dynamic-component consumer for now — an honest reflection of current need.

## Spike (before acceptance)

Compile the omnia target adapter (~700 lines) in natively behind `adapter::Target`; run one build+merge through it in the native suite. Enumerate exactly what deletes and what breaks. Budget: 1–2 days.

## Revisit trigger

A signed third-party adapter engagement, or a client isolation/audit requirement that process boundaries cannot satisfy.
