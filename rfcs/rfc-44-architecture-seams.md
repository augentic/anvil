# RFC-44: Architecture Seam Hardening

> Status: Draft · Serves: roadmap principles "[Keep the CLI authoritative](roadmap.md#principles)" and "Core owns reconciliation" · Complements: [RFC-43](rfc-43-release-proving.md) (release proving), [RM-14](roadmap.md#rm-14-local-structured-workflow-events) / [RM-15](roadmap.md#rm-15-structured-change-lifecycle-status-for-re-entry) (observability), the [REVIEW.md](../REVIEW.md) backlog (mechanics)
> Provenance: findings from the 2026-06-11 architecture review of both repos (live trees; `specify-cli` mid extraction-cache removal).

## Abstract

The framework's core architecture is healthy; the risk concentrates at four seams: the cross-repo contract is enforced by `rg` discipline rather than by a checked artifact, workflow control flow still lives in skill prose, status is scattered across hand-maintained surfaces instead of projected from the journal, and shared vocabulary is restated in dozens of files. This RFC proposes one hardening move per seam — a machine-checked contract dump, continued migration of decisions into CLI verbs, a projection-over-persistence rule anchored on the journal, and digest-pinned canonical prose — plus a mechanics batch (delegated to REVIEW.md) and two forward flags (adapter versioning, reference-corpora context budget). One explicit anti-recommendation: do not split `specify-workflow`.

## What is already strong (do not spend budget here)

- **Layering invariants are type-system facts, not conventions** — `specify-standards` ⊥ `specify-workflow`, `specify-validate` → `specify-model` only; a lint rule physically cannot transition a slice ([architecture.md](https://github.com/augentic/specify-cli/blob/main/docs/standards/architecture.md)).
- **Single-writer lifecycle ownership** and closed enums at parse boundaries (operations, lifecycles, journal taxonomy, divergence).
- **Projection over persistence has a working precedent** — provenance is projected on demand; no persisted `provenance.yaml` to drift.
- **Decision discipline** — 44-entry [DECISIONS.md](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md); no production file over ~600 lines in a ~95k-line workspace; a REVIEW cadence with a calibration post-mortem.
- **Subtraction instinct** — the in-flight extraction-cache removal is the right kind of change (see F1 for its doc tail).

## Motivation — findings

**F1 — the cross-repo contract is held together by grep discipline.** Skills and briefs cite CLI verbs, kebab-case error discriminants, and journal event ids in prose; the only enforcement is [AGENTS.md](https://github.com/augentic/specify-cli/blob/main/AGENTS.md) rule 5 ("rg across both repos, same PR"). Every REVIEW round finds the resulting drift: CI docs vs `ci.yaml` (D1), stale source paths (B1), a cited test fn that doesn't exist (D2). The live case: the in-flight cache removal deletes `crates/workflow/src/adapter/cache.rs` and the `cache: opt-out` manifest key, while the parent repo's `AGENTS.md` ("cache fingerprints" vocabulary entry, the `cache: opt-out` gotcha), [claim-reconciliation.md](../plugins/spec/references/synthesis/claim-reconciliation.md), the CLI's `docs/standards/workflow.md` cache passage, and two DECISIONS entries (§"Cache layout", §"Extraction cache fingerprint inputs") still describe it.

**F2 — control flow lives in skill prose.** The `/spec:execute` loop's park/resume/all-done decisions and the dual-driving pre-flight refusal are skill-markdown behavior, testable only by live trials (the "skill-loop orchestration" category in [docs/contributing/evals.md](../docs/contributing/evals.md)). The two-phase `slice build --phase prepare|finalize` migration is the worked precedent: the skill stopped hand-transitioning and the gate became deterministic.

**F3 — status is scattered.** Trial-catalog status is hand-maintained in three places (RFC-43 F3); plan/change re-entry status is re-derived from artifact state per skill run; RM-14/RM-15 are roadmap items waiting on a substrate the journal refactor is already building.

**F4 — shared prose is restated, not referenced.** The authority hierarchy appears in ~17 files; the vocabulary block is tripled across `AGENTS.md`, `.cursor/rules/project.mdc`, and `README.md`; the slice loop is phrased in ~60 files. The canonicalization tools already exist (the `spec-runtime` symlink bundle, mdBook stubs, the `content-digest-eq` Road A kind) but are not applied to these surfaces.

**F5 — mechanics, already catalogued.** [REVIEW.md](../REVIEW.md) ranks them correctly: the tracked 37.7 MB binary (0.2), the path-pinned committed `Specify.toml` (0.3), wasi-tools outside CI (A1/C1), exit-4 untestability (A2), the duplicated DiagnosticReport envelope across seven tools (C2). Two additions from this review: the embedded framework-tool blobs resolve with `sha256: None` ("interim" digest pinning that will calcify), and the contract tool's dist blob has no sidecar or drift test (C3).

**F6 — two forward-looking gaps.** (a) `omnia@v1` pins a *repo ref*, not an adapter version; RM-21's third-party adapter ecosystem will need per-adapter versioning and a compatibility story. (b) The omnia + vectis reference corpora are ~30k markdown lines — roughly 48% of the repo — and are LLM-facing input with no context-budget governance, while skills carry hard 200/45/512 caps.

## Recommendations

**R1 — machine-check the cross-repo contract.** Add a `specify contract dump` verb emitting the CLI's surface as JSON: verbs and flags (clap-introspectable), exit codes, error discriminants, journal event ids, embedded schema ids. Then a framework lint rule (Road A `cross-reference` or a Road B tool) checks every `` `specify …` `` invocation, error id, and event id cited in skill and brief bodies against the dump of the pinned binary `make lint` already builds via `scripts/specify.rs`. The F1 drift class — including the cache-removal tail — becomes a lint finding instead of a review discovery. RFC-43 R3's test-name check shares this machinery.

**R2 — keep migrating control flow into CLI verbs.** Next two candidates, chosen because they shrink the live-trial surface (RFC-43 Phase 4): a deterministic next-action verb for the execute loop (e.g. `specify plan status --next-action` returning `park | resume <slice> | all-done` with a reason), reducing the skill to a renderer; and a CLI-owned plan lease so dual-driving refusal is enforced by the runtime, not by skill pre-flight prose. Writer-ownership rules apply as everywhere: one writer per field, closed transitions.

**R3 — write "projection over persistence" into DECISIONS and anchor it on the journal.** Status is projected from journal + artifacts; never a new persisted state file. Provenance already works this way; trial-catalog status and RM-15's re-entry status are next. A near-term journal read surface (`specify journal show --filter`, the lineage of RM-14's `events tail / export` target) serves RFC-43's probes, the R2 next-action verb, and any future dashboard (RM-22) from one substrate.

**R4 — finish the cache subtraction across both repos in the same change.** Sweep `rg -i 'extraction cache|cache fingerprint|cache: opt-out'` across both repos: parent `AGENTS.md` vocabulary and gotchas, `claim-reconciliation.md`, the CLI's `workflow.md` cache passage, and the two DECISIONS entries (mark superseded with a pointer, per DECISIONS culture — don't silently delete). This is exactly AGENTS rule 4/5; R1 automates the class going forward.

**R5 — one canonical home per concept; digest-pin the necessary restatements.** Authority hierarchy → [synthesis/authority.md](../plugins/spec/references/synthesis/authority.md) canonical, the other ~16 sites become links or one-line summaries. Vocabulary → `AGENTS.md` §Vocabulary canonical; where an agent-context file must restate a table (`.cursor/rules/project.mdc`), pin it with `content-digest-eq` so divergence is a lint finding.

**R6 — mechanics batch.** Execute the REVIEW backlog in its existing order (P0s first); add to it: pin the seven framework-tool blob digests (replace `sha256: None`), extract the `framework-wire` shared crate before tool #10 appears, and give the contract dist blob the same sidecar + drift test the framework tools have.

**R7 — two cheap forward flags.** Write the DECISIONS placeholder for adapter versioning now (what `@v1` means today, what per-adapter semver would change) so RM-21 starts from a recorded position. Add a lightweight context-budget hint for reference corpora — e.g. a per-reference-dir index requirement via existing `presence`/`cardinality` kinds — before the corpora double again.

**Anti-recommendation — do not split `specify-workflow`.** Its ~30k lines (≈20k production) are organized, its boundaries encode the real invariants, and the propose kernel is already kernel-shaped internally. A `specify-plan` crate earns its existence only when a second consumer appears.

## Execution plan

| Phase | Deliverable | Repo(s) | Effort | Depends |
| --- | --- | --- | --- | --- |
| 0 | R4 cache-removal doc sweep rides the in-flight change (same PRs); REVIEW P0s (tracked binary, `Specify.toml` pin, red `cargo make check`) | both | S | in-flight work |
| 1 | R1 `specify contract dump` + framework lint cross-check rule; wire RFC-43's test-name check into the same mechanism | both | M–L | none |
| 2 | R2 next-action verb + plan lease; thin the `/spec:execute` and pre-flight skill prose; promote the unblocked trial scenarios (RFC-43 Phase 4) | both | M | none (R1 helps verify) |
| 3 | R3 DECISIONS "projection over persistence" entry; journal read surface; RM-15 status projection as its first consumer | specify-cli | M | journal refactor lands |
| 4 | R5 canonicalization sweep — authority hierarchy first (~17 sites), then vocabulary triple, with digest pins | specify | M | none |
| ongoing | R6 mechanics per REVIEW order; R7 flags (DECISIONS placeholder S, corpora hint S) | both | S–M each | none |

Phases 1, 2, and 4 are mutually independent; Phase 3 waits only on the journal refactor already in flight. Phase 2 is the highest-leverage single item because it pays out twice — runtime-enforced behavior and a smaller live-trial surface.

## Non-Goals

- **No crate splits for size** — `specify-workflow` stays whole; no `specify-plan` until a second consumer exists.
- **No new persisted state files** — R3 is a constraint, not a feature; status surfaces are projections.
- **No compatibility shims** — renames and contract changes are hard cuts per house rules; R1 makes the cuts visible, it does not soften them.
- **No re-litigation of the REVIEW backlog** — its priorities stand; this RFC adds to it, not over it.
- **The roadmap's non-goals stand** — no lifecycle authority outside the CLI, no hosted-infrastructure requirement for the core loop.

## Open Questions

1. **Contract-dump shape.** Pure clap introspection plus const tables, or a hand-curated manifest with a parity test (the embedded-schema pattern)? Does the dump carry the binary version so the lint finding can say *which* pin disagrees?
2. **Lease mechanism.** Plan lease as a journal event pair, a lockfile under `.specify/`, or a `plan.yaml` field? Single-writer and crash-recovery semantics decide; journal-derived ownership is the most consistent with R3.
3. **Canonical homes.** Is `AGENTS.md` or a `docs/` page the canonical vocabulary home, given `AGENTS.md` must stay navigational ("not long-form documentation" is a roadmap non-goal)?
4. **Adapter versioning timing.** Does the DECISIONS placeholder commit to per-adapter semver pre-1.0, or only record the repo-ref status quo until RM-21 activates?

## References

- [REVIEW.md](../REVIEW.md) — the mechanics backlog (Part 0, A1/A2, B1–B4, C1–C3, D1–D3) this RFC delegates to rather than duplicates.
- [specify-cli AGENTS.md](https://github.com/augentic/specify-cli/blob/main/AGENTS.md) — crate graph, rule 4/5 (the discipline R1 mechanises), module map.
- [specify-cli DECISIONS.md](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md) — §"Cache layout", §"Extraction cache fingerprint inputs" (R4 targets), and the home for R3's and R7's new entries.
- [docs/standards/architecture.md](https://github.com/augentic/specify-cli/blob/main/docs/standards/architecture.md) and [docs/standards/workflow.md](https://github.com/augentic/specify-cli/blob/main/docs/standards/workflow.md) — the documented invariants this RFC extends.
- [docs/explanation/standards-layer.md](../docs/explanation/standards-layer.md) — the Road A / Road B lint machinery R1 and R5 reuse.
- [RFC-43](rfc-43-release-proving.md) — the proving-side consumer of R1, R2, and R3.
- [Roadmap](roadmap.md) — RM-14/RM-15 (R3), RM-21 (R7), RM-22 (downstream of R3), and the principles this RFC serves.
