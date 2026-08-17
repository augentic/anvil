# Emery Target Architecture

> Status: **Draft v0 — provisional until the decision gate ([decisions/](decisions/) ADR-0001…0006) is accepted.** Sections marked `[ADR-NNNN]` take their final shape from that decision; ADR-0002 (Wasm-primary) is already accepted, and this draft reflects it. This document is written forward from the operator journey in [product.md](product.md), not backward from the current implementation. It becomes v1 in remediation Phase 2 and is thereafter the document every agent task brief cites.
>
> The prior generation's failure mode was findings-driven repair with no destination ([architecture-review.md](architecture-review.md)). This document is the destination.
>
> [capability-conservation.md](capability-conservation.md) is the remediation traceability contract: target architecture v1 must map each conserved implemented capability to a section here and executable Phase 3/4 acceptance evidence, or to an accepted ADR that explicitly deletes it. The ledger does not override this document and retires after remediation closes.

## 1. The journey, restated as architecture

```text
emery spec    sources → leads → evidence → synthesis → one reviewable spec per slice
              (operator reviews; conflicts require disposition [ADR-0004])
emery build   reviewed slice → private workspace → phase machine → verified CID → baseline
              (resumable: re-run the verb; parallel later, serial until crash-proof)
emery status  one next action, from one state read
emery fix     durable, digest-bound guidance onto a stuck slice or wrong spec
```

One lifecycle `[ADR-0003]`: legacy code, documentation, contracts, intent, screenshots, captures, and designs are *sources*. Architecture modelling (as-is, target, migration) is an optional projection over the same evidence corpus, not a second product.

## 2. Shape

One process, one change home, one loop. **Wasm-primary deployment** (ADR-0002, accepted): the WIT component seam is the sole production seam — adapters are Wasm components, first-party ones embedded in the binary as default registry entries, out-of-binary components admitted by one explicit install verb at an exact version. Adapter *logic* stays wasm-free behind the `adapter::Source` / `adapter::Target` operations traits (fast native unit tests); integration and grading cross the component seam with a scripted model backend. The native provider is deleted, not demoted. Workspace-kernel placement (guest-side vs host-side) is decided by the D3 benchmark during platform hardening — no default bias.

### Kept kernels (ported, not rewritten)

| Kernel | Why it survives both reviews |
| --- | --- |
| Content-addressed snapshot store + CID identity | Store-neutral, deployment-independent digests; load-bearing for detached delivery (guest-vs-host placement per the D3 benchmark) |
| The WIT component seam + Omnia hosting | Foundational (ADR-0002): dynamic adapter admission and desktop/web-service duality; hardened (profiles D7, budgets D8, CI rung T1), never deleted |
| RFC-90 engine-owned build phase machine (`build → verify ⇄ repair → review ⇄ repair`) | Implemented as specified; extended to merge (addendum A9), never bypassed |
| Artifact parsers, collapsed to **one fail-closed spec AST** (addendum A17) | The typed serde parse as the load gate is sound; the two-parser split is not |
| Adapter operations traits + prose corpus (~27k lines) | The traits are the seam either deployment answer keeps; prose ports nearly intact |
| Refinement as a separate stage ("execute never refines") | Held in the current implementation; keep the boundary, fix the receipt (S36) |
| Ultrathin skills (invoke-and-relay) | Honest; the CLI owns lifecycle |

### Deleted planes

The shadow native provider and its conversion layer, plus the five-mode adapter resolution matrix — OCI pull-on-miss, bare names, cache seeds, pull-latest (ADR-0002; the seam survives, the duality and the distribution plumbing do not); `crates/system` as a parallel lifecycle `[ADR-0003]`; the in-place change-home mode and all five mode encodings `[ADR-0005]`; journal-as-authority reducers and the multi-writer claim protocol `[ADR-0001]`; the second topology compiler (`propose_from` in authoring, addendum P10); the guest HTTP mutating catch-all (addendum C3); `plan correct` as an unscoped constraint plane (addendum P16 — superseded by the `fix` design in §6).

## 3. State model `[ADR-0001]`

One transactional state store per change home (SQLite or single atomically swapped state document — spike decides). Properties, regardless of storage choice:

- **One authority per question.** Topology, authorization, build identity, wave membership, debt, scope: rows/receipts in the store, transactionally coupled. No directory-presence authority, no mtime selection, no fact-union reducers.
- **Generation identity is a column.** Every authoring act, correction, deferral, wave, and archive row names its generation. Force re-authoring starts a new generation; mixed-generation reads are unrepresentable, not rejected.
- **Every irreversible external effect brackets a transaction**: intent row → effect → completion row, so resume is a table lookup (the addendum's S13/S33/S35/S38/S40 class becomes unrepresentable).
- **The journal is observability.** Append-only NDJSON for audit and telemetry; it never gates lifecycle; a failed append never changes an exit code — and nothing reads it back for authority.
- **Fail closed.** Any authority read error is `Err`, never an empty set (S13, S23).

## 4. Module map and budgets

Provisional layering (final crate names at v1). Budgets are ratchet ceilings, not goals; the current engine is ~101k lines against Omnia's ~30k for a whole runtime platform.

```text
error/diagnostics     — unchanged leaves
artifacts             — one spec AST (fail-closed), evidence, leads; no engine deps
adapter               — operations traits, seam DTOs (ONE claim family with extras, A8/A16), prose registry
store                 — the ADR-0001 state store + snapshot/CID kernel
engine                — the one loop: mine (survey/extract/synthesize), review-doc projection,
                        executor (waves as antichains S32, merge on the phase machine A9),
                        correction (fix), publication as a drain-tail stage (S38)
transport             — clap grammar, one RequestContext (C5), generated error registry (C2/C4), exit contract
host                  — engine-guest embedding, exact-pin component admission + embedded first-party
                        registry, capability profiles (D7), dispatch budgets (D8) [ADR-0002]
adapters (sibling)    — first-party source/target components over TargetContext (A10/A13/A15);
                        wasm-free cores, one export-macro guest module each
```

Ratchet ceilings (unconfirmed until v1): engine total ≤ 40k Rust; no crate > 8k; prose corpus ≤ 20k; CLI routes ≤ product.md's verb list plus an advanced namespace.

## 5. The adapter seam

- **The WIT is the source of truth; one generated type family** for Claim / Lead / PhaseReport used everywhere the seam is parsed (A1, A8, A16). The claim record opens in the contract — core fields plus per-kind extras — so extract fails when required extras are absent. No "unmodeled keys are ignored" paths, and no hand-maintained mirrors of the WIT shapes.
- **`TargetContext` on every target operation** (A10): generation, platforms (validated once, never re-discovered from YAML — A15), change-home roots, bound source identities, staged slice tree, baseline CID. Adapters never probe `.emery` (A13); a missing stage is `InvalidRequest`, not a fallback.
- **Merge on the phase machine** (A9): `phase-report`, engine-owned repair budget, read-only views, capture only after a declared mutating phase.
- Every adapter skip is a typed `not-applicable` or a blocking finding — no fail-open defaults (A14).
- **Isolation and budgets are seam properties, not prose** (ADR-0002): per-axis least-privilege capability profiles with malicious-fixture tests (D7); wall-clock, fuel/epoch, memory, and output budgets with guest-terminating cancellation on every host dispatch (D8).

## 6. The two product surfaces the old system never had

**The per-slice review document (P8).** One diff-friendly document per slice folding requirements, provenance summary, open gaps, and conflicts; approve/reject is the review act; the structured artifacts (model, tasks) derive from or subordinate to it; the parsers that conserve it are the one AST of §2. Reviewability is measured (review time in T5 telemetry).

**Conversational correction (`fix`, P9).** A stuck slice exposes its typed stop and repair brief; operator guidance is recorded as a durable, digest-bound, generation-scoped fact consumed as a hard input by the retry at the stage that stopped (spec re-synthesis or build repair). One `CorrectionTarget` type resolves what the guidance binds to (addendum S25's lesson). Corrections are consumed on honor and lapse with their generation (S16) — never immortal.

## 7. Deliberately not built

Replaces the "there is no…" negative-space ledger. Each has a reopen trigger via ADR:

| Not built | Reopen when |
| --- | --- |
| Component *distribution* machinery — registries, pull-on-miss, bare-name resolution, marketplace | third-party adapter engagement (ADR-0002; the component seam and exact-pin install are foundational and already built) |
| The web-service mutating ingress (auth, per-change anchoring, lease) | a hosted deployment is scheduled; until designed, mutating HTTP stays disabled (C3) |
| Second lifecycle / definition home | architecture-only engagement at scale (ADR-0003) |
| Multi-node, streaming, hosted fleets, unattended merge | per [platform.md](platform.md) parked RFCs — measured pull only |
| Parallel execution above cap 1 | crash-injection proves every stage idempotent |
| Intra-slice task graphs | a measured slice too large for one build |

## 8. The walking skeleton

The executable definition of this document. Scripted model, offline, temp change home; **runs across the component seam** — the embedded engine guest plus mock source/target components, so the shipped seam is the tested seam (ADR-0002, T1); runs in CI on every push; green is the definition of done for every remediation increment:

1. `emery spec` over an intent source (+ one docs source) → assert: one review document per slice, gaps typed, conflicts blocking `[ADR-0004]`.
2. Scripted operator approval (and one conflict disposition).
3. `emery build` → assert: accepted CID exists, baseline advanced, debt projected from store rows.
4. `emery status` → `drained`.
5. `emery fix` on an injected stuck slice → guidance fact recorded → retry consumes it → slice completes.
6. **Crash-injection rung:** kill at every store-write and external-effect boundary; re-run the verb; assert convergence with zero manual state surgery.

## See also

- [product.md](product.md) — the yardstick this document serves
- [capability-conservation.md](capability-conservation.md) — capability parity and deferred Phase 4 obligations
- [remediation-plan.md](remediation-plan.md) — the path from the current tree to this document
- [architecture-review.md](architecture-review.md) / [architecture-review-addendum.md](architecture-review-addendum.md) — the evidence base; finding ids cited above resolve there
