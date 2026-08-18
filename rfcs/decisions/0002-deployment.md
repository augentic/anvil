# ADR-0002: Deployment — Wasm-primary, one seam, one admission mode

> Status: **Accepted** (operator decision, 2026-08-17). The spike below scopes cost and sequencing; it does not reopen the direction — only the revisit trigger does.
> Date: 2026-08-17
> Supersedes the originally proposed native-only resolution (recorded in Options below for the archaeology).

## Context

Two full providers implement the same capability traits; the production Wasm seam is the one path no automated test runs; the resolver has five modes with no durable settled identity; adapter isolation is descriptive; components have no resource or liveness budget ([architecture-review.md](../architecture-review.md) D1, D5–D8, T1, A1; [addendum](../architecture-review-addendum.md) C3, D14).

Two requirements are **foundational** and predate this codebase: (R1) adapters must be addable dynamically without rebuilding the host, and (R2) one core must run as a desktop CLI and as a web service. A previous non-Wasm attempt failed on exactly these. The serious alternatives each fail one of them: native plugins (no stable Rust ABI, no isolation, per-platform artifacts), subprocess adapters (per-platform distribution, hand-rolled wire protocol, OS-level isolation only), containers (desktop DX), embedded scripting (type-poor seam, abandons the Rust adapter ecosystem). Wasm components — one artifact, typed WIT contract, instance-per-call, a real capability boundary — are the correct answer to R1+R2. The Omnia thesis ([architecture.md](../architecture.md)) stands.

The review's actual finding was therefore never "Wasm is wrong" but "two products, and the tested one is not the shipped one." The costs are (a) the **dual native/Wasm seam** and (b) the **dynamic-distribution plumbing** that arrived before its marketplace.

## Options

- **A. Native-only** (originally proposed): compiled-in adapters, native engine. Deletes the platform — and with it R1 and R2, which are non-negotiable.
- **B. Status quo dual**: both providers, native tested, Wasm shipped. The intolerable position.
- **C. Wasm-primary, one seam, one admission mode** — accepted, below.

## Decision

1. **The WIT component seam is the sole production seam.** The native provider (`crates/native` provider + `convert.rs`) is **deleted**, not demoted — a retained shim would silently re-become the tested path, which is how the duality arose. Adapter *logic* stays wasm-free behind the operations traits, so unit tests remain in-crate and fast; integration, the walking-skeleton journey, and grading run over the component seam with a **scripted model backend** (the model is a host capability, so scripting it requires no guest change).
2. **Admission collapses to one mode.** Dynamic admission by exact identity is the foundational capability and is retained — with one permanent CI test admitting an out-of-binary component so the extension point stays real. First-party adapter components are **embedded in the binary** as default registry entries (zero-fetch desktop, reproducible `plan.yaml`). Deleted: OCI pull-on-miss during metadata dispatch, bare names with stderr-settled identity, cache seeds shadowing upgrades, pull-latest provisioning — the entire D5 matrix. Installing an out-of-binary component is one explicit verb over an exact pin.
3. **Isolation and budgets are platform features, scheduled — not deferred costs.** Per-axis least-privilege capability profiles with malicious-fixture CI tests (D7) and host-dispatch budgets — wall-clock with a real timer, fuel/epoch interruption, memory ceilings, output caps, guest-terminating cancellation (D8) — land in the platform-hardening phase before the walking skeleton widens. R2 (web service) requires D8 regardless. *(Sequencing of D7/D8 relative to the spec-generator skeleton is overridden by [ADR-0008](0008-spec-generator-programme.md): they wait with the build programme; the CI rung remains.)*
4. **The WIT is the source of truth for the seam types.** One generated Claim / Lead / PhaseReport family; the claim record opens (core fields + extras) so A8's data loss is fixed *in the contract*, and the five hand-maintained mirrors collapse toward the WIT (A1, A16).
5. **The web-service ingress is a designed future surface, not the current catch-all.** The guest's mutating HTTP routes stay disabled (addendum C3) until an ingress with auth, per-change anchoring, and a real lease is designed. The `wasi:http` export shape is the right trigger; everything around it is missing.
6. **Workspace-kernel placement (guest-side vs host-side) is decided by the D3 benchmark, with no default bias** — the operator records no preference. "Wasm is foundational" does not mean every kernel belongs in the guest; RFC-95 already demonstrated the host-capability pattern. Throughput, memory, and capability complexity decide it during platform hardening.

## Deletions

`crates/native`'s provider and conversion layer (~1.2k lines) and its shadow test path; the resolver matrix (five modes → one); `launcher::install` pull-on-miss and pull-latest; bare-name resolution and cache seeding; `adapter upgrade` surface (re-scoped to the explicit install verb). Concept-count effect: the operator-visible adapter identity model shrinks to "embedded, or installed at an exact version."

## Consequences

- Platform hardening (CI rung, profiles, budgets, one type family) was originally sequenced ahead of the walking skeleton. [ADR-0008](0008-spec-generator-programme.md) keeps the CI rung (extract across the component seam) as a skeleton gate and defers profiles/budgets with the build programme.
- Integration iteration is slower (wasm32 builds in CI); mitigated by trait-level unit tests staying native and fast.
- Release coupling for embedded first-party adapters (same team, same cadence — acceptable); out-of-binary installs cover mid-cycle adapter delivery.
- The Omnia dogfooding value is retained; Emery remains the runtime's flagship guest-hosting consumer.

## Spike (scopes cost; does not reopen the direction)

The original spike described a survey/extract round-trip, a build phase report, and an MCP hop. [ADR-0008](0008-spec-generator-programme.md) withdraws that as a gate: the spec-generator CI rung is extract across the component seam, in CI, offline, with a scripted model (embedded engine guest + mock source component); assert byte-identical results across two runs. That *is* T1 for this programme. A build-phase-report hop returns with the build programme. If the extract rung fights, the findings feed the platform plan — either way the seam gains its first automated rung.

## Revisit trigger

The spike or platform-hardening phase demonstrates the component seam cannot run the offline journey affordably in CI (build-time or complexity budget stated in the remediation plan), in which case the fallback is subprocess adapters (the only alternative satisfying R1+R2 partially) — a new ADR, not a silent drift back to a native provider.
