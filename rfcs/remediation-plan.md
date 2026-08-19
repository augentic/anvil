# Emery Remediation Plan

> Status: **The plan of record.** Supersedes the prior sequencing (retrievable at tag `v1`). The programme ships one product: **the specification generator**. Build planning, building, and merging are conserved as design intent (the deferred annex of [target-architecture.md](target-architecture.md)) and stay frozen until the generator is reliable. This document also carries the insights ledger — context from the review sessions that must survive them.
>
> Rule of the plan: feature work is frozen until the spec walking skeleton (Phase 3) is green.

## The product, restated

Mine sources (documentation, code, intent) into a coherent, reviewable specification of the current system and how it could be rebuilt. That specification set is the shippable product for Propellerhead — whether or not a build ever follows. Pipeline:

```text
sources  →  extract     (one source operation; survey is gone)
         →  per-source specifications
         →  synthesise  (merge under authority precedence)
         →  one coherent spec set
```

First artifacts: `spec.md` (what the system must do) and `design.md` (how a rebuild would do it). `composition.yaml` and `tasks.md` wait — composition is a Vectis *build* output, and tasks are a build-order artifact. Putting them in the first generator pulls target shape into a product that is not building yet.

Operator surface: `emery init`, `emery specify`. Two verbs is phasing, not a new yardstick. The four-verb budget in [product.md](product.md) (`spec` / `build` / `status` / `fix`) remains the destination; only the first verb ships now.

**Not in this programme:** leads catalog, focused survey, decomposition, plan topology, waves, workspaces, target WIT, build/verify/repair/review/merge, `status`/`fix`, publication, `crates/system`. Those stay design intent in the deferred annex. They are not kept kernels for this build.

## Artefact map

| Artefact | Role | State |
| --- | --- | --- |
| [product.md](product.md) | Product yardstick; the spec generator is its `spec` verb | Written; programme scope note |
| [decisions/](decisions/) | ADR log | All accepted; 0001/0004/0005 as re-scoped (2026-08-19) |
| [target-architecture.md](target-architecture.md) | Destination for this programme; build/fix in the deferred annex | Stamped v1 (2026-08-19) |
| [capability-conservation.md](capability-conservation.md) | Live entries for the generator; the rest deferred with the build programme | Re-scoped |
| [CONSTITUTION.md](../CONSTITUTION.md) | Standing invariants + mechanical enforcement | Invariant 1 re-scoped to the spec journey |
| [architecture-review.md](architecture-review.md) | The evidence base (addendum folded in); finding ids cited here resolve there | Standing |
| This file | Sequence, conservation strategy, insights ledger | Written |

## Phase 0 — Freeze, don't repair (days)

The prior containment phase repaired planes now being frozen rather than shipped. No patches land in code destined for the archive.

1. **Tag `v1` — done.** Annotated tag on this repo (`Emery v1 — pre-remediation archive`) and on `augentic/emery-adapters`. Retrieve with `git worktree add ../emery-v1 v1`. There is no `archive/v1` branch yet; the tag is sufficient.
2. **Delete superseded crates from the live workspace** when the spine starts (Phase 3). Do not leave them on the branch agents work on. Do not move them to `crates-v1/` — see [Conserving v1](#conserving-v1).
3. **Prune the CLI** to `init` + `specify` (a stub is fine); delete the remaining routes from the grammar rather than hiding them.
4. **Simplify `wit/emery.wit`:** one source world exporting `extract` (+ `metadata`); no survey operation, no `lead` / `survey-result`, no target world. Extract returns specifications (or a claim set that *is* the spec IR), not evidence for a terminal lead. The claim record is open — core fields plus per-kind extras, fail-closed when required extras are absent (A8/A16 as a design input to the new seam, not a patch to the old one).
5. **Keep the guest HTTP mutating catch-all disabled** (C3) — the only prior containment item that stays live, because the runtime shell survives.

## Phase 1 — Decisions (days, not weeks)

- **ADR-0002 (Wasm-primary):** stands as accepted. The component seam survives; the WIT it carries shrinks. Capability profiles (D7) and dispatch budgets (D8) defer with the build programme — they are not a gate on the spec skeleton.
- **ADR-0003 (one lifecycle):** **accepted.** `crates/system` stays in the archive; architecture modelling returns, if ever, as a projection over the mined corpus.
- **ADR-0006 (rebuild shape):** **accepted, narrowed.** New spine for extract + synthesise only; the archive is tag `v1`, not an in-tree quarantine.
- **ADR-0008 (this programme):** **accepted.** The live journey is sources → specification. Survey collapses into extract. First artifacts are `spec.md` / `design.md`.
- **ADR-0001 (state store):** **accepted as re-scoped (2026-08-19).** A spec generator's state is source bindings, extract receipts, and the spec set. Atomically swapped documents committed by a generation-pointer swap; observability is `wasi:otel` and the journal is deleted outright; no SQLite spike, no merge-commit matrix.
- **ADR-0004 (conflict disposition):** **accepted as re-scoped (2026-08-19).** Conflicts appear inline (`[conflict]`). There is no build gate yet, so there is no auto-defer vs operator-gate fight. Disposition-before-build returns with the build programme.
- **ADR-0005 (change home):** **accepted as re-scoped (2026-08-19).** One output home for specs. Do not spend this programme encoding in-place vs detached.

## Phase 2 — Destination + a red journey test (~1 week)

1. Stamp [target-architecture.md](target-architecture.md) v1 around the pipeline above. The executor, phase machine, merge, `status`, and `fix` remain in the **deferred annex** — design intent the build programme re-derives from, not current scope.
2. **Land the fitness functions before the build starts:** journey test (red first — it is the Phase 3 exit criterion), route budget (2 verbs: `init` + `specify`), LOC ratchet with the archive excluded, layering test, prose budgets, ADR-required-paths check.
3. Specify the walking skeleton as an executable test: scripted model, offline, across the component seam — embedded engine guest + mock source component. `emery specify` over intent + one docs source → assert synthesised `spec.md` / `design.md`, gaps typed `[unknown]`, conflicts visible, byte-stable re-run diff.
4. Keep [capability-conservation.md](capability-conservation.md) to the live generator entries. Topology, workspaces, waves, and publication stay deferred-with-the-build-programme, not Preserve-for-this-skeleton.

Do not wait on ADR-0002's original spike (survey/extract + build phase report + MCP). The spec-generator CI rung is extract-only across the seam.

## Phase 3 — The walking skeleton (the build begins)

New crates only, per target-architecture v1's module map; borrow from tag `v1` by explicit reviewed port, never by linking archive crates. Order within the phase: output home → extract (one operation, typed claims with extras) → synthesise under authority precedence → emit `spec.md` / `design.md`. Milestone: **that journey passes in CI across the component seam.** Nothing else counts as progress.

## Phase 4 — Make the generator reliable, then stop

> Status: **landed 2026-08-19** — the three items below are in the tree ([capability-conservation.md](capability-conservation.md) Phase 4 exit carries the acceptance evidence); the conditional fourth-source ports (contracts, screenshots, captures) wait on demonstrated Propellerhead need, and the first live green scorecard is the operator's to record.

Each increment keeps the skeleton green:

1. **Sources:** documentation, code, intent. Then contracts, screenshots, captures if Propellerhead needs them — porting extract prose from `emery-adapters` at tag `v1`, extras honored per A8.
2. **Re-mine diffs:** a changed source shows the reviewer what changed.
3. **Graded eval** against product.md's "time to first reviewable specification"; the eval runner as a public-contract client (T6); wired as the release gate.

Ship to Propellerhead. Only then open the next programme gate: **build planning, build, merge** — re-derived from the conserved annex and tag `v1` by explicit port, behind its own decision record.

Do not add a slice inventory, a refine loop that spawns slices, or collation until a real Propellerhead spec is wrong without them. Those are build-planning wearing a spec-generator coat.

## Conserving v1

**Decided: no second repository, and no `crates-v1/` on the live branch.** Git history in this repo is the archive; a clone re-creates the zombie failure mode this project is specifically prone to — two trees drift and you have a second product again. An in-tree `crates-v1/` is greppable convenience that fights the anti-reversion strategy: agents extend whatever they can see (~101k lines of the old system sitting in the workspace). A `LEGACY` note will not hold.

1. **Tag `v1` (done).** Annotated tag on this repo and on `augentic/emery-adapters`. A buildable old tree on demand is `git worktree add ../emery-v1 v1` — disposable, detached at the tag, no second remote, no drift. An `archive/v1` branch is optional convenience, not required.
2. **Live tree deletes superseded crates** when Phase 3 starts. New work goes in new crate names. History is `git show v1:crates/slice/...` and `git log v1`. When a port happens: open the worktree, copy the kernel, review it against the new WIT and docs, land it in the new crate, close the worktree. Never `path =` depend on archive crates.
3. **`emery-adapters` stays a sibling repo**, tagged `v1` in lockstep. Source-adapter extract prose is load-bearing for the new product; target adapters freeze there. Survey prompts are not ported.

The discipline against legacy patterns leaking into the new spine is: new spine in new crates, the journey test runs only against the new spine, and legacy code is read from the tag — not relocated back — as ported capability lands.

## Anti-reversion strategy

Prose rules did not hold — agents faithfully extend whatever exists, and lab pressure deleted a designed gate (R3). Enforcement is therefore mechanical (the CONSTITUTION.md fitness functions) plus a ratchet, with prose only as explanation:

- **The ratchet converts aggregate drift into individual red builds.** Per-crate LOC ceilings, route budgets, seam-copy counts, and prose budgets in a committed baseline file make each increment of drift a CI failure someone must justify with an ADR reference.
- **Gate tripwires make policy deletion loud.** One integration test per operator gate, named `adr_NNNN_*`. Deleting a conflict-visibility check means deleting a test that names its decision record.
- **The journey test makes composition failure immediate.** T3 (the wasm example silently became an illegal workflow) and the S2/S32 divergence classes are exactly what a permanently-green end-to-end journey prevents.
- **The monthly scorecard** (CONSTITUTION.md ritual) walks the review's acceptance list and ratchet deltas — 30 minutes, recorded as a dated note.

## Insights ledger

Context from the review sessions (2026-08-17) that must not be lost:

1. **Scale datum.** Engine ~101k lines of Rust (project 33.9k, change 17.8k, slice 15k, system 6.7k) + ~27k adapter prose + ~12k adapter Rust. Omnia — the entire runtime platform, including twelve WASI host-capability crates, guest SDK, macros, conformance suite — is ~29.8k. A workflow CLI 3.4× its runtime is a scope symptom before an engineering one. Omnia's coherence came from being built to a settled architecture; repair does not produce that property.
2. **The yardstick error.** The first review audited the implementation against platform.md and never audited platform.md against the product. Findings-driven repair without a destination converges on the same system, hardened. Hence: product.md first, ADRs second, architecture third, cuts re-derived last.
3. **The dissolution logic.** One transactional store per change home dissolves (not fixes) S1–S3, S6–S8, S10, S11, D9, D10 and the addendum's reducer-class findings — but *not* the missing types and seams (authoring generation, CorrectionTarget, SurveyReceipt, claim family, wave antichain), which must be designed regardless. Do not let the store decision masquerade as the whole fix. *(Re-scoped 2026-08-18: with the programme narrowed to the spec generator, the store question shrinks to atomically swapped documents — see Phase 1.)*
4. **Native-only is the highest-leverage subtraction** (~a third of the blocker list) — but A8 (claim-extras drop) lives in the native converter too, D14 (unscoped MCP grant) survives on the native shelf, and the isolation requirement is deferred, not deleted. *(Superseded in part by insight 11: the subtraction actually taken was the native provider and the resolution matrix, not the platform.)*
5. **A8 is the quiet product killer.** The seam silently drops the structured claim fields (`statement`, `criterion`, `replay-digest`) that first-party extract prompts require and synthesis prefers. Eval "worked" via the `synopsis` fallback, so the degradation of the core spec-mining function was invisible. Lesson: eval greenness is not evidence the designed data path is exercised.
6. **The lab shaped the product** (R3/P3/T6). Auto-deferral replaced a designed operator gate so unattended eval could finish; the probe runner cannot represent a typed stop (exit 2 fails the case) and grades a build back door as a workflow. The measurement instrument must be a client of the public contract or it will keep selecting product shortcuts.
7. **`plan correct` was the R-pattern happening live**: a new durable authority plane, unscoped, with the constraint keyed to the wrong node (S25) and a resume path force can break (S26) — landed while the review warning against new planes stood. The process rules exist for exactly this. *(Resolved 2026-08-17: removed in a dedicated commit; the design intent returns as `fix` in the deferred annex.)*
8. **Cap-one is not a soundness proof.** S32 (wave = ready batch, not antichain) and S34 (refine retracts frozen waves) are membership/isolation design bugs that reproduce at cap one. They live in the deferred build programme now, but the lesson generalizes: serial greenness is not a concurrency design.
9. **Development-loop causes** (R1–R4): RFC-at-a-time with no walking skeleton; AGENTS.md as load-bearing spec compounding prose and code sprawl; policy changes without decision records; addition-only scope because agents don't push back. The countermeasures are the constitution's invariants — mechanically enforced, because prose did not hold.
10. **Second-pass yield stayed high** (~65 new findings after a ~45-finding first pass), which is itself evidence for rebuilding the spine over refactoring it (ADR-0006): finding density did not fall with a second look at the same regions.
11. **Wasm is foundational — an operator decision, not an engineering preference** (ADR-0002, accepted 2026-08-17). Two requirements predate this codebase and killed a prior non-Wasm generation: adapters must be addable dynamically without rebuilding the host, and one core must run as a desktop CLI and a web service. Native plugins, subprocess adapters, containers, and embedded scripting each fail one of them. The review's real finding was the *duality* (the tested seam was not the shipped seam) and premature *distribution* plumbing — so the subtraction taken deletes the native provider and the five-mode resolver, keeps and hardens the component seam, and re-prices D7/D8/T1 as scheduled platform features rather than deferred costs. For *this* programme, D7/D8 wait with the build programme; the CI rung is extract across the seam (ADR-0008). The operator recorded **no preference on guest-side vs host-side workspace kernel** — the D3 benchmark decides placement on evidence (now a build-programme question).
12. **The programme narrowed to the specification generator** (operator decision, 2026-08-18, ADR-0008). Survey collapses into extract; the WIT shrinks to the source axis; the CLI prunes to `init` + `specify`; the archive is tag `v1` (worktree on demand), not an in-tree `crates-v1/` and not a second repository; build planning, building, and merging return only after the generator ships and proves reliable. First artifacts are `spec.md` and `design.md`.
