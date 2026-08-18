# Emery Remediation Plan

> Status: **The plan of record.** Supersedes the prior sequencing (retrievable at tag `v1`). The programme is now scoped to one shippable product: **the specification generator**. Build planning, building, and merging are conserved as design intent (the deferred annex of [target-architecture.md](target-architecture.md)) and stay frozen until the generator is reliable. This document also carries the insights ledger — context from the review sessions that must survive them.
>
> Rule of the plan: feature work is frozen until the spec walking skeleton (Phase 3) is green.

## The product, restated

Mine sources (documentation, code, intent, contracts, screenshots) into a coherent, reviewable set of specifications describing the current system and how it could be rebuilt. This is shippable on its own — the specification is the product ([product.md](product.md)), whether or not a build ever follows. Pipeline:

```text
sources → extract     (one operation; survey collapsed into extract)
        → spec IR
        → synthesise  (merge per-source specs under authority precedence)
        → slice specs (spec.md / design.md / tasks.md per slice;
                       slice = one buildable requirement)
        → refine loop (cross-cutting concerns inferred from slices spawn new slices)
        → collation   (one unified, ordered, human-readable specification)
```

Operator surface for this programme: `emery init`, `emery specify`. The four-verb product budget (`spec` / `build` / `status` / `fix`) remains the destination; only the first verb ships now. Nobody reads the two-verb CLI as the yardstick — it is phasing, recorded here.

## Artefact map

| Artefact | Role | State |
| --- | --- | --- |
| [product.md](product.md) | Product yardstick; the spec generator is its `spec` verb | Written; programme scope note added |
| [decisions/](decisions/) | ADR log | Re-scoped in Phase 1 below |
| [target-architecture.md](target-architecture.md) | The destination; build/fix sections move to a deferred annex | Draft v0 → v1 in Phase 2 |
| [capability-conservation.md](capability-conservation.md) | Traceability: capability → new spine, deferred annex, or deleting ADR | Re-scope in Phase 2 |
| [CONSTITUTION.md](../CONSTITUTION.md) | Standing invariants + mechanical enforcement | Written |
| [architecture-review.md](architecture-review.md) | The evidence base (addendum folded in); finding ids cited here resolve there | Standing |
| This file | Sequence, conservation strategy, insights ledger | Written |

## Phase 0 — Conserve and cut (days)

The prior containment phase repaired planes now being frozen rather than shipped; it collapses to freeze-and-quarantine. No patches land in code destined for `crates-v1/`.

1. **Tag the current state `v1`** (annotated tag + branch at the pre-trim commit). This is the archive; see [Conserving v1](#conserving-v1).
2. **Quarantine:** move superseded crates to `crates-v1/`, excluded from the Cargo workspace — frozen reference text, never compiled, never a pattern exemplar. Kept-kernel crates (`error`, `diagnostics`, the surviving parts of `artifacts` and the snapshot/CID kernel) stay live per target-architecture's kept-kernels table.
3. **Prune the CLI** to `init` + `specify` (a stub is fine); delete the remaining routes from the grammar rather than hiding them.
4. **Simplify `wit/emery.wit`:** one source world exporting `extract` (+ `metadata`); no survey operation, no target worlds yet. The claim record is open in the contract — core fields plus per-kind extras, fail-closed when required extras are absent (A8/A16 as a design input to the new seam, not a patch to the old one). Mine earlier, simpler WIT revisions from git history (`git log -- wit/emery.wit crates/guest/wit/engine.wit`) as a starting point.
5. **Keep the guest HTTP mutating catch-all disabled** (C3) — the only prior containment item that stays live, because the runtime shell survives.

## Phase 1 — Decisions (days, not weeks)

- **ADR-0002 (Wasm-primary):** stands as accepted. The component seam survives; the WIT it carries shrinks.
- **ADR-0003 (one lifecycle):** accept — `crates/system` freezes into `crates-v1/`; architecture modelling returns, if ever, as a projection over the evidence corpus.
- **ADR-0006 (rebuild shape):** accept — new spine crates, legacy quarantined.
- **ADR-0001 (state store):** **re-scope.** A spec generator's state is source bindings, extract receipts, the spec IR, the slice set, and the refinement generation — no waves, epochs, merges, or claims. Decide atomically-swapped documents vs a store on that scope; the crash-injected merge matrix spike is not run as written.
- **ADR-0004 (conflict disposition):** now the central product decision — how conflicts surface in the IR and the review document under authority precedence. Decide before synthesis lands.
- **ADR-0005 (change home):** decide on paper; the new spine has one change-home mode.

The prior platform-hardening phase (capability profiles D7, dispatch budgets D8, admission-mode collapse) was priced for a system that builds and merges; for a read-sources/write-specs product it defers to the build programme. Only the component-seam CI rung (T1) survives, narrowed to the source axis.

## Phase 2 — Target architecture v1 + enforcement (~1 week)

1. Rewrite [target-architecture.md](target-architecture.md) around the pipeline above; the executor, phase machine, and `fix` sections move to an explicit **conserved-deferred annex** — design intent the build programme re-derives from, not current scope.
2. **Land the fitness functions before the build starts:** journey test (red first — it is the Phase 3 exit criterion), route budget (2 verbs), LOC ratchet with `crates-v1/` excluded and pinned, layering test, prose budgets, ADR-required-paths check.
3. Specify the walking skeleton as an executable test: scripted model, offline, across the component seam — embedded engine guest + mock source component. `emery specify` over intent + one docs source → assert per-slice `spec.md`/`design.md`, gaps typed `[unknown]`, conflicts surfaced per ADR-0004, byte-stable re-run diff.
4. Re-scope [capability-conservation.md](capability-conservation.md): every ledger entry maps to (a) the new spine, (b) the deferred-build annex, or (c) an ADR that deletes it.

## Phase 3 — The walking skeleton (the build begins)

New crates only, per target-architecture v1's module map; borrow from `crates-v1/` by explicit reviewed port, never by reference. Order within the phase: state substrate → extract (one operation, typed claims with extras) → synthesis under authority precedence → slice projection → review document. Milestone: **the skeleton passes in CI across the component seam.** Nothing else counts as progress.

## Phase 4 — Widen the generator

Each increment lands only with the skeleton still green:

1. **Sources:** documentation, code (typescript), intent first; then contracts, screenshots, captures — porting prose from `emery-adapters`, extras honored per A8.
2. **The refine loop:** staleness, re-mining diffs, cross-cutting slice inference.
3. **Collation:** the single ordered, human-readable specification.
4. **Graded eval** against product.md's "time to first reviewable specification" and per-operation success numbers; the eval runner as a public-contract client (T6); wired as the release gate.

Ship to Propellerhead. Only then open the next programme gate: **build planning, build, merge** — re-derived from the conserved annex and `crates-v1/` by explicit port, behind its own decision record.

## Conserving v1

Decided: **no second repository.** Git history in this repo is the archive; a clone re-creates the zombie failure mode this project is specifically prone to — two trees drift and you have a second product again.

1. **`v1` annotated tag + branch** at the pre-trim commit. A buildable old tree on demand is `git worktree add ../emery-v1 v1` — disposable, no second remote, no drift.
2. **`crates-v1/` in-tree** for greppability during ports: listed under `[workspace] exclude`, never compiled, never linted, ratchet-exempt but frozen, one `LEGACY` note per crate, and an AGENTS.md line: reference for porting only, never a pattern exemplar. Crates move wholesale — no `zz_` renames; the directory boundary is the segregation, and renames would break the frozen text's internal references and `git log --follow`.
3. **`emery-adapters` stays a sibling repo:** the source-adapter prose (extract prompts) is directly load-bearing for the new product; target adapters freeze in place there under the same tag discipline.

The discipline against legacy patterns leaking into the new spine (agents read ~101k lines as exemplars) is unchanged from the prior plan: new spine in new crates, the journey test runs only against the new spine, and legacy code is deleted — not relocated back — as ported capability lands with green ledger evidence.

## Anti-reversion strategy

Prose rules did not hold — agents faithfully extend whatever exists, and lab pressure deleted a designed gate (R3). Enforcement is therefore mechanical (the CONSTITUTION.md fitness functions) plus a ratchet, with prose only as explanation:

- **The ratchet converts aggregate drift into individual red builds.** Per-crate LOC ceilings, route budgets, seam-copy counts, and prose budgets in a committed baseline file make each increment of drift a CI failure someone must justify with an ADR reference.
- **Gate tripwires make policy deletion loud.** One integration test per operator gate, named `adr_NNNN_*`. Deleting the conflict gate means deleting a test that names its decision record.
- **The journey test makes composition failure immediate.** T3 (the wasm example silently became an illegal workflow) and the S2/S32 divergence classes are exactly what a permanently-green end-to-end journey prevents.
- **The monthly scorecard** (CONSTITUTION.md ritual) walks the review's acceptance list and ratchet deltas — 30 minutes, recorded as a dated note.

## Insights ledger

Context from the review sessions (2026-08-17) that must not be lost:

1. **Scale datum.** Engine ~101k lines of Rust (project 33.9k, change 17.8k, slice 15k, system 6.7k) + ~27k adapter prose + ~12k adapter Rust. Omnia — the entire runtime platform, including twelve WASI host-capability crates, guest SDK, macros, conformance suite — is ~29.8k. A workflow CLI 3.4× its runtime is a scope symptom before an engineering one. Omnia's coherence came from being built to a settled architecture; repair does not produce that property.
2. **The yardstick error.** The first review audited the implementation against platform.md and never audited platform.md against the product. Findings-driven repair without a destination converges on the same system, hardened. Hence: product.md first, ADRs second, architecture third, cuts re-derived last.
3. **The dissolution logic.** One transactional store per change home dissolves (not fixes) S1–S3, S6–S8, S10, S11, D9, D10 and the addendum's reducer-class findings — but *not* the missing types and seams (authoring generation, CorrectionTarget, SurveyReceipt, claim family, wave antichain), which must be designed regardless. Do not let the store decision masquerade as the whole fix. *(Re-scoped 2026-08-18: with the programme narrowed to the spec generator, the store question shrinks with it — see Phase 1.)*
4. **Native-only is the highest-leverage subtraction** (~a third of the blocker list) — but A8 (claim-extras drop) lives in the native converter too, D14 (unscoped MCP grant) survives on the native shelf, and the isolation requirement is deferred, not deleted. *(Superseded in part by insight 11: the subtraction actually taken was the native provider and the resolution matrix, not the platform.)*
5. **A8 is the quiet product killer.** The seam silently drops the structured claim fields (`statement`, `criterion`, `replay-digest`) that first-party extract prompts require and synthesis prefers. Eval "worked" via the `synopsis` fallback, so the degradation of the core spec-mining function was invisible. Lesson: eval greenness is not evidence the designed data path is exercised.
6. **The lab shaped the product** (R3/P3/T6). Auto-deferral replaced a designed operator gate so unattended eval could finish; the probe runner cannot represent a typed stop (exit 2 fails the case) and grades a build back door as a workflow. The measurement instrument must be a client of the public contract or it will keep selecting product shortcuts.
7. **`plan correct` was the R-pattern happening live**: a new durable authority plane, unscoped, with the constraint keyed to the wrong node (S25) and a resume path force can break (S26) — landed while the review warning against new planes stood. The process rules exist for exactly this. *(Resolved 2026-08-17: removed in a dedicated commit; the design intent returns as `fix` in the deferred annex.)*
8. **Cap-one is not a soundness proof.** S32 (wave = ready batch, not antichain) and S34 (refine retracts frozen waves) are membership/isolation design bugs that reproduce at cap one. They live in the deferred build programme now, but the lesson generalizes: serial greenness is not a concurrency design.
9. **Development-loop causes** (R1–R4): RFC-at-a-time with no walking skeleton; AGENTS.md as load-bearing spec compounding prose and code sprawl; policy changes without decision records; addition-only scope because agents don't push back. The countermeasures are the constitution's invariants — mechanically enforced, because prose did not hold.
10. **Second-pass yield stayed high** (~65 new findings after a ~45-finding first pass), which is itself evidence for rebuilding the spine over refactoring it (ADR-0006): finding density did not fall with a second look at the same regions.
11. **Wasm is foundational — an operator decision, not an engineering preference** (ADR-0002, accepted 2026-08-17). Two requirements predate this codebase and killed a prior non-Wasm generation: adapters must be addable dynamically without rebuilding the host, and one core must run as a desktop CLI and a web service. Native plugins, subprocess adapters, containers, and embedded scripting each fail one of them. The review's real finding was the *duality* (the tested seam was not the shipped seam) and premature *distribution* plumbing — so the subtraction taken deletes the native provider and the five-mode resolver, keeps and hardens the component seam, and re-prices D7/D8/T1 as scheduled platform features rather than deferred costs. The operator recorded **no preference on guest-side vs host-side workspace kernel** — the D3 benchmark decides placement on evidence (now a build-programme question).
12. **The programme narrowed to the specification generator** (operator decision, 2026-08-18, this document). Survey collapses into extract; the WIT shrinks to the source axis; the CLI prunes to `init` + `specify`; superseded crates freeze into `crates-v1/` under tag `v1` rather than being repaired; build planning, building, and merging return only after the generator ships and proves reliable. The specification set — per-slice `spec.md`/`design.md`/`tasks.md` plus one collated ordered document — is the shippable product for Propellerhead.
