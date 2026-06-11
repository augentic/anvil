# RFC-44: Architecture Seam Hardening

> Status: Draft · Serves: roadmap principles "[Keep the CLI authoritative](roadmap.md#principles)" and "Core owns reconciliation" · Complements: [RFC-43](rfc-43-release-proving.md) (release proving), [RM-14](roadmap.md#rm-14-local-structured-workflow-events) / [RM-15](roadmap.md#rm-15-structured-change-lifecycle-status-for-re-entry) (observability), the [REVIEW.md](../REVIEW.md) backlog (mechanics)
> Provenance: 2026-06-11 architecture review of both repos, revised the same day against the landed tree. Resolved items (the R4 cache-removal sweep, the journal-refactor dependency, the lease-mechanism question, two REVIEW P0s) are pruned; recommendation and phase numbering is kept stable for cross-references.

## Abstract

The framework's core architecture is healthy; the risk concentrates at four seams: the cross-repo contract is enforced by `rg` discipline rather than by a checked artifact, workflow control flow still lives in skill prose, status is scattered across hand-maintained surfaces instead of projected from the journal, and shared vocabulary is restated in dozens of files. This RFC proposes one hardening move per seam — a machine-checked contract dump (R1), continued migration of control flow into CLI verbs (R2), a projection-over-persistence rule anchored on the journal (R3), and digest-pinned canonical prose (R5) — plus a mechanics batch (R6, delegated to REVIEW.md) and two forward flags (R7: adapter versioning, reference-corpora context budget). One explicit anti-recommendation: do not split `specify-workflow`.

## What is already strong (do not spend budget here)

- **Layering invariants are type-system facts, not conventions** — `specify-standards` ⊥ `specify-workflow`, `specify-validate` → `specify-model` only; a lint rule physically cannot transition a slice ([architecture.md](https://github.com/augentic/specify-cli/blob/main/docs/standards/architecture.md)).
- **Single-writer lifecycle ownership** and closed enums at parse boundaries (operations, lifecycles, journal taxonomy, divergence).
- **Projection over persistence has a working precedent** — provenance is projected on demand; no persisted `provenance.yaml` to drift.
- **Decision discipline** — ~52-entry [DECISIONS.md](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md); no production file over ~600 lines in a ~95k-line workspace; a REVIEW cadence with a calibration post-mortem.
- **Subtraction instinct** — the extraction-cache removal landed with its cross-repo documentation tail swept in the same change, and DECISIONS gained a superseding entry rather than silently losing two: the right kind of change, executed to pattern.

## Motivation — findings

**F1 — the cross-repo contract is held together by grep discipline.** Skills and briefs cite CLI verbs, kebab-case error discriminants, and journal event ids in prose; [cli-contract.md](../docs/standards/cli-contract.md) hand-maintains the verb tree, envelope shape, and exit-code summary skills depend on; the only enforcement is [AGENTS.md](https://github.com/augentic/specify-cli/blob/main/AGENTS.md) rule 5 ("rg across both repos, same PR"). Every REVIEW round finds the resulting drift — CI docs vs `ci.yaml` (D1, still live: the parent claims "CI runs only `make lint`" while `ci.yaml` runs a stable-toolchain sibling checkout via `cargo run … lint framework`), stale source paths (B1) — and the class keeps minting: the CLI's own [`plan/cli.rs`](https://github.com/augentic/specify-cli/blob/main/src/runtime/commands/plan/cli.rs) module doc cites "nested `plan lock *` verbs" that were deliberately never built (the lock landed skill-side, F2), and one evidence doc-comment still says "cache fingerprinting" ([example.rs](https://github.com/augentic/specify-cli/blob/main/crates/model/src/evidence/claim/example.rs)).

**F2 — control flow lives in skill prose.** The `/spec:execute` loop's dispatch and stop decisions are skill-markdown behavior, testable only by live trials (the "skill-loop orchestration" category in [docs/contributing/evals.md](../docs/contributing/evals.md)). The loop's contracts are now *documented* — [stop-conditions.md](../plugins/spec/references/stop-conditions.md) fixes the structured `stop: build-failed` / `stop: merge-conflict` / `drained` strings, and [plan-lock.md](../plugins/spec/references/plan-lock.md) fixes the `flock`-based `.specify/plan.lock` snippet that mediates dual-driving — but both stay skill-enforced: the CLI neither emits the stop hints nor takes or checks the lock. The migration direction has two worked precedents: the two-phase `slice build --phase prepare|finalize` (the skill stopped hand-transitioning and the gate became deterministic), and the global `--plan-dir` / `SPECIFY_PLAN_DIR` plan-root override, which moves workspace-routing plan resolution from skill path-joins into a CLI seam ([workspace-routing.md](../plugins/spec/skills/execute/references/workspace-routing.md)).

**F3 — status is scattered; the journal substrate has landed.** The closed journal taxonomy now carries the eval-probe events (`plan.entry.advanced`, `workspace.sync.completed` / `workspace.push.completed`, the closed `actor` enum on `plan.transition.approved`), and RFC-43's grading probes consume the JSONL with `jq` as "the interim substrate until RFC-44 R3's `specify journal show --filter` lands" ([assertions.md](../evals/shared/assertions.md)). Still scattered: plan/change re-entry status is re-derived from `plan.yaml` + slice `metadata.yaml` per skill run (RM-15), the RM-05 rollup is hand-maintained, and `specify journal` still has exactly one verb — `emit`.

**F4 — shared prose is restated, not referenced.** The authority hierarchy appears in ~17 files; the slice loop is phrased in ~65. The vocabulary block is part-way canonicalised — `.cursor/rules/project.mdc` §Vocabulary is now a bare link to `AGENTS.md`, but `README.md`'s cheat sheet still restates the two workflow nouns verbatim, unpinned. The canonicalization tools already exist (the `spec-runtime` symlink bundle, mdBook stubs, the `content-digest-eq` Road A kind) but are not applied to the remaining surfaces.

**F5 — mechanics, already catalogued.** [REVIEW.md](../REVIEW.md) ranks them correctly. Still open: the path-pinned committed `Specify.toml` (0.3), wasi-tools outside CI (A1/C1), exit-4 untestability (A2), the duplicated DiagnosticReport envelope across seven tools (C2). Two additions from this review: the embedded framework-tool blobs resolve with `sha256: None` ("interim" digest pinning that will calcify), and the contract tool's dist blob has no sidecar or drift test (C3).

**F6 — two forward-looking gaps.** (a) `omnia@v1` pins a *repo ref*, not an adapter version; DECISIONS records the status quo (§"First-party `<adapter>` shorthand at init") but not what per-adapter versioning would change — RM-21's third-party adapter ecosystem needs that position recorded. (b) The omnia + vectis reference corpora are ~29k markdown lines — roughly 45% of the repo — and are LLM-facing input with no context-budget governance, while skills carry hard 200/45/512 caps.

## Recommendations

**R1 — machine-check the cross-repo contract.** Add a `specify contract dump` verb emitting the CLI's surface as JSON: verbs and flags (clap-introspectable), exit codes, error discriminants, journal event ids, embedded schema ids. Then a framework lint rule (Road A `cross-reference` or a Road B tool) checks every `` `specify …` `` invocation, error id, and event id cited in skill and brief bodies against the dump of the pinned binary `make lint` already builds via `scripts/specify.rs`. [cli-contract.md](../docs/standards/cli-contract.md) — today's hand-curated statement of the same surface — gains a parity check or is generated outright. The F1 drift class becomes a lint finding instead of a review discovery. RFC-43 R3's named-test-citation check shares this machinery.

**R2 — keep migrating control flow into CLI verbs.** Two candidates next, chosen because they shrink the live-trial surface (RFC-43 Phase 4). First, a deterministic next-action verb (e.g. `specify plan status --next-action`) returning the dispatch the skill currently derives from slice lifecycle plus stop classification — `refine|build|merge <slice>`, `stop <reason>`, `drained`, rendered with [stop-conditions.md](../plugins/spec/references/stop-conditions.md)'s structured strings — reducing `/spec:execute` to a renderer. Second, runtime enforcement of the landed plan lock: the mechanism is decided (`.specify/plan.lock` held via OS advisory `flock`, skill-acquired, deliberately not a CLI verb — [plan-lock.md](../plugins/spec/references/plan-lock.md)), so the remaining move is for plan-state-writing verbs (`plan next`, `plan transition`, `slice merge`) to probe the same lock and refuse an unlocked driver — making dual-driving refusal a runtime property instead of a per-skill snippet discipline, with the probe kept consistent with journal-derived ownership (R3). Writer-ownership rules apply as everywhere: one writer per field, closed transitions.

**R3 — write "projection over persistence" into DECISIONS and anchor it on the journal.** Status is projected from journal + artifacts, or pinned to its single authored home by lint — never a second hand-maintained copy. Provenance already works this way, and the eval catalog is lint-pinned ([CORE-056](../adapters/shared/rules/core/CORE-056-scenarios-catalog-runs-drift.md)); RM-15's re-entry status is the next projection consumer. The journal read surface (`specify journal show --filter`, the lineage of RM-14's `events tail / export` target) has named consumers waiting: the eval probes' documented `jq` bridge, the R2 next-action verb, and any future dashboard (RM-22) — one substrate for all three.

*(R4 — the cross-repo cache-subtraction doc sweep — landed as prescribed and is retired; numbering is kept stable. Its one-line residue is in R6.)*

**R5 — one canonical home per concept; digest-pin the necessary restatements.** Authority hierarchy → [synthesis/authority.md](../plugins/spec/references/synthesis/authority.md) canonical, the other ~16 sites become links or one-line summaries. Vocabulary → `AGENTS.md` §Vocabulary canonical — `.cursor/rules/project.mdc` already links; the remaining verbatim restatement (`README.md`'s cheat sheet) gets a `content-digest-eq` pin so divergence is a lint finding.

**R6 — mechanics batch.** Execute the REVIEW backlog in its existing order; the `Specify.toml` fetchable pin (0.3) is the surviving P0. Add to it: pin the seven framework-tool blob digests (replace `sha256: None`), extract the `framework-wire` shared crate before tool #10 appears (nine crates today), give the contract dist blob the same sidecar + drift test the framework tools have, and sweep the two stale doc-comments F1 catalogues (`plan/cli.rs`, `example.rs`).

**R7 — two cheap forward flags.** Extend the DECISIONS shorthand entry (or add a placeholder beside it) with the forward half: what per-adapter semver would change relative to today's repo-ref `@v1`, so RM-21 starts from a recorded position. Add a lightweight context-budget hint for reference corpora — e.g. a per-reference-dir index requirement via existing `presence`/`cardinality` kinds — before the corpora double again.

**Anti-recommendation — do not split `specify-workflow`.** Its ~34k lines (≈20k production) are organized, its boundaries encode the real invariants, and the propose kernel is already kernel-shaped internally. A `specify-plan` crate earns its existence only when a second consumer appears.

## Execution plan

| Phase | Deliverable | Repo(s) | Effort | Depends |
| --- | --- | --- | --- | --- |
| 1 | R1 `specify contract dump` + framework lint cross-check rule (parity-check or generate `cli-contract.md`); wire RFC-43's named-test-citation check into the same mechanism | both | M–L | none |
| 2 | R2 next-action verb + runtime lock probing; thin `/spec:execute` to a renderer of the stop-conditions strings; retire the unblocked eval scenarios (RFC-43 Phase 4 — `dual-driving-refused` leaves the catalog once refusal is CLI-enforced) | both | M | none (R1 helps verify) |
| 3 | R3 DECISIONS "projection over persistence" entry; `specify journal show --filter` read surface (retiring the probes' documented `jq` bridge); RM-15 status projection as its first consumer | specify-cli | M | none |
| 4 | R5 canonicalization sweep — authority hierarchy first (~17 sites), then the `README.md` cheat-sheet digest pin | specify | M | none |
| ongoing | R6 mechanics per REVIEW order; R7 flags (DECISIONS forward half S, corpora hint S) | both | S–M each | none |

All four phases are mutually independent. Phase 2 is the highest-leverage single item because it pays out twice — runtime-enforced behavior and a smaller live-trial surface.

## Non-Goals

- **No crate splits for size** — `specify-workflow` stays whole; no `specify-plan` until a second consumer exists.
- **No new persisted state files** — R3 is a constraint, not a feature; status surfaces are projections. (`.specify/plan.lock` is consistent with this: the lock identity is the OS advisory lock, and the file body is holder diagnostics, not status — [plan-lock.md](../plugins/spec/references/plan-lock.md).)
- **No compatibility shims** — renames and contract changes are hard cuts per house rules; R1 makes the cuts visible, it does not soften them.
- **No re-litigation of the REVIEW backlog** — its priorities stand; this RFC adds to it, not over it.
- **The roadmap's non-goals stand** — no lifecycle authority outside the CLI, no hosted-infrastructure requirement for the core loop.

## Open Questions

1. **Contract-dump shape.** Pure clap introspection plus const tables, or a hand-curated manifest with a parity test (the embedded-schema pattern)? `cli-contract.md` already plays the hand-curated role, untested — does the dump check it or replace it? Does the dump carry the binary version so the lint finding can say *which* pin disagrees?
2. **Canonical homes.** Is `AGENTS.md` or a `docs/` page the canonical vocabulary home, given `AGENTS.md` must stay navigational ("not long-form documentation" is a roadmap non-goal)? The landed `project.mdc` link pattern works for agent-context files either way.
3. **Adapter versioning timing.** Does the DECISIONS position commit to per-adapter semver pre-1.0, or only record the repo-ref status quo (§"First-party `<adapter>` shorthand at init") until RM-21 activates?

## References

- [REVIEW.md](../REVIEW.md) — the mechanics backlog this RFC delegates to rather than duplicates.
- [docs/standards/cli-contract.md](../docs/standards/cli-contract.md) — the hand-maintained CLI-surface statement R1 mechanises.
- [plan-lock.md](../plugins/spec/references/plan-lock.md) and [stop-conditions.md](../plugins/spec/references/stop-conditions.md) — the landed skill-side control-flow contracts R2 migrates runtime-side.
- [evals/shared/assertions.md](../evals/shared/assertions.md) and [CORE-056](../adapters/shared/rules/core/CORE-056-scenarios-catalog-runs-drift.md) — the probe taxonomy waiting on R3's read surface, and the catalog pin R3 cites as projection-or-pin precedent.
- [specify-cli AGENTS.md](https://github.com/augentic/specify-cli/blob/main/AGENTS.md) — crate graph, rule 4/5 (the discipline R1 mechanises), module map.
- [specify-cli DECISIONS.md](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md) — §"First-party `<adapter>` shorthand at init" (R7's recorded status quo) and the home for R3's and R7's new entries.
- [docs/standards/architecture.md](https://github.com/augentic/specify-cli/blob/main/docs/standards/architecture.md) and [docs/standards/workflow.md](https://github.com/augentic/specify-cli/blob/main/docs/standards/workflow.md) — the documented invariants this RFC extends.
- [docs/explanation/standards-layer.md](../docs/explanation/standards-layer.md) — the Road A / Road B lint machinery R1 and R5 reuse.
- [RFC-43](rfc-43-release-proving.md) — the proving-side consumer of R1, R2, and R3.
- [Roadmap](roadmap.md) — RM-14/RM-15 (R3), RM-21 (R7), RM-22 (downstream of R3), and the principles this RFC serves.
