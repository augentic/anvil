# RFC-43: Release Proving

> Status: Draft · Serves: [RM-05](roadmap.md#rm-05-multi-repo-eval-suite) · Complements: [RFC-42](rfc-42-acceptance.md) (capability-axis enrichment), [RFC-39](future/rfc-39-acceptance-shape-traces.md) (deferred shape tier), [RFC-44](rfc-44-architecture-seams.md) (the CLI-side levers), [RM-14](roadmap.md#rm-14-local-structured-workflow-events) (journal events)
> Provenance: findings from the 2026-06-11 architecture review of both repos (live trees; `specify-cli` mid extraction-cache removal). Sibling review: [REVIEW.md](../REVIEW.md), whose D2 items cover the accuracy fixes this RFC does not repeat.

> **Status note.** Point (1) has since been resolved differently: the pack was renamed `acceptance/` → `evals/`, the `backend` field was removed from the scenario schema entirely (no `live`/`fixture` enum survives), the automated-coverage matrix was deleted, and deterministic proof lives only as named tests in `augentic/specify-cli` ([docs/contributing/evals.md](../docs/contributing/evals.md)). Points (2)–(4) — probe/judgment separation, lint-pinned catalog tables, and the tiered gate — remain open proposals; read their `backend:`-based mechanics against the new evals vocabulary. Body kept as written.

## Abstract

Four improvements to the acceptance surface: (1) rename the activity to **release proving** — the existing **deterministic proofs** plus **live trials** — retiring the word "acceptance" and the `backend: manual` misnomer; (2) separate *who drives* a trial from *who grades* it, giving every assertion id a mechanical probe (journal query or artifact check) and an explicit judgment flag for the irreducibly human residue; (3) pin the catalog status tables and the automated-coverage matrix with lint rules instead of editorial discipline; (4) tier the release gate into a blocking core and a full-catalog cadence.

RFC-42's capability-axis plan, fixture-vs-manual decision rule, and one-catalog principle are unchanged. This RFC renames the frame and mechanises the clerical layers around it; it adds no lifecycle authority and keeps every `negative-expectation` held.

## Motivation — findings

**F1 — "acceptance" is the wrong word, three ways.** It collides with the framework's own artifact vocabulary — requirement blocks and the SoW mapping already use *acceptance criteria* in the `spec.md` sense ([requirement-block.md](../plugins/spec/references/synthesis/requirement-block.md)). It implies customer UAT, when this is a vendor-run release gate. And the docs' own prose has already abandoned it: [docs/contributing/evals.md](../docs/contributing/evals.md) says "the deterministic CLI **proof**", "a release is **proven** only when both surfaces are green", "the manual **sweep**". `backend: manual` is similarly off — an agent drives the runbook, and the pack README has to clarify "the scenarios covered here are agent-based". The honest discriminator is **live** (live agent, live forge, live binary) vs **fixture** (deterministic, every commit).

**F2 — grading is unstructured.** The agent runbook says "self-grade only the structurally checkable assertions", but no assertion defines its check. [pure-intent](../evals/scenarios/pure-intent.md) §Scope concedes that *every in-scope assertion is deterministic structure* — what is live is the skill loop, yet the grading of `plan-exists`, `plan-validates`, `sources-intent-only`, `refine-reaches-refined` is re-derived per run. The `negative-expectations` forbid an automated **runner**, fake forge, CI target, and golden bytes; none of them forbids a mechanical **grader** run after a live trial. The current posture conflates driving with grading.

**F3 — status is hand-maintained in three places and has already rotted.** Catalog tables in [scenarios/README.md](../evals/scenarios/README.md), run-record filenames under [runs/](../evals/runs/README.md), and the RM-05 rollup each carry status by hand. [REVIEW.md](../REVIEW.md) D2 caught the automated-coverage matrix citing a test fn that does not exist (`synthesize_resolves_same_authority_conflict` vs the real `synthesize_same_authority_conflict`).

**F4 — the gate as defined has never closed.** 7 of 14 live scenarios are `pending`, and RFC-42 Phases 1–4 will grow the catalog. A single undifferentiated "every non-deferred entry passed" gate gets more expensive with every scenario added.

**F5 — half the manual surface is manual for architecture reasons.** The "skill-loop orchestration" category exists because park/resume/all-done decisions live in skill prose, not in a CLI verb. That is [RFC-44](rfc-44-architecture-seams.md) R2's territory; each verb migration promotes scenarios out of the live sweep.

## R1 — the rename

| Current | Proposed | Rationale |
| --- | --- | --- |
| "acceptance" (umbrella) | **release proving** | "The release proof is green" is already the docs' sentence. |
| Surface 1 — deterministic CLI proof | unchanged | Already well named. |
| Surface 2 — "manual sweep" / `evals/` | **live trials**, directory `trials/` | Sea-trials connotation: the built system proven under real conditions before commissioning. `runs/` read naturally as trial records. |
| `backend: manual` | `backend: live` | Accurate pair with `fixture`; an agent, not a human hand, drives it. |
| `docs/contributing/evals.md` | `docs/contributing/proving.md` | The umbrella doc. |

Alternates considered: `proving/` for the directory too (strongest fit with existing prose, but overloads one word for umbrella and surface), `exercises/` (live-exercise sense, vaguer), `rehearsals/` (implies the run doesn't count). Keep the RFC and RM identifiers (RFC-39/40/42, RM-05); update their prose on next touch.

Blast radius — one lockstep change across both repos:

| Surface | Repo | Change |
| --- | --- | --- |
| `schemas/authoring/scenario.schema.json` `backend` enum | specify-cli | `manual` → `live` (hard cut, no alias — house rule) |
| `scenarios` wasi-tool | specify-cli | Re-validate against the new enum; rebuild `dist/` blob |
| `evals/` tree | specify | Rename to `trials/`; frontmatter `backend:` flip in 14 scenario files |
| CORE rule `config:` path globs touching `evals/` | specify | Update; includes the stale CORE-031 `evals/recorded/` reference (REVIEW D3) |
| `Makefile`, `scripts/snapshot.sh`, `.gitignore` (`.sandbox`) | specify | Path updates |
| `docs/contributing/evals.md`, mdBook `SUMMARY`, `AGENTS.md`, `README.md`, `rfcs/roadmap.md` prose | specify | Rename + terminology sweep |

## R2 — separate driving from grading

A trial stays live; its grading becomes mechanical. Every assertion id gets exactly one of:

- **A probe** — a deterministic post-run check: a CLI read verb, an artifact predicate, or a journal query. Probes never drive the workflow and never transition anything.
- **A judgment flag** — the irreducibly human residue (prose quality, ergonomics), graded by operator or agent with an evidence pointer, as today.

The journal is the natural probe substrate: lifecycle assertions are (or should be) journal events, and the journal refactor is in flight. This aligns with RM-14's `events tail / export` target surface — the live trials are RM-14's first consumer. Illustrative mapping for `pure-intent`:

| Assertion id | Probe sketch |
| --- | --- |
| `plan-exists` | `plan.yaml` present after `/spec:plan` |
| `plan-validates` | `specify plan validate` exits 0 |
| `intent-single-lead` | exactly one lead block under `## Lead inventory` in `discovery.md` |
| `gate-1-not-auto-stamped` | `plan.yaml` lifecycle is `pending` immediately after the skill run |
| `sources-intent-only` | `Sources: [intent]` via `specify slice provenance` |
| `refine-reaches-refined` | slice lifecycle `refined`; journal shows the transition event |

Where an assertion has no event or read verb to probe (e.g. transition actor), that is a gap to close in the closed taxonomy by decision — not a reason to keep grading by hand.

This folds directly into RFC-42 Phase 0: the planned assertion-id taxonomy doc becomes *id → definition + probe (or judgment flag)*, turning it from a glossary into an executable contract. The run-template's assertion table then cites probe output as evidence, and the agent runbook's "self-grade" step becomes "run the probes, judge the flagged residue".

The `negative-expectations` stand unchanged: `automated-runner-added`, `fake-forge-added`, `transcript-replay-added`, `ci-target-added`, `golden-output-required` all constrain *driving*; probes are post-run *grading*. If the distinction needs to be explicit, a one-line clarification in the proving doc suffices — no new expectation id.

## R3 — pin the catalog with lint

Two checks, both shapes the lint engine already owns:

1. **Catalog ↔ runs cross-reference** (Road A `cross-reference`): every catalog row's status agrees with the latest `runs/<id>.<result>.md` record; every `live` row has a scenario file; every `automated` row does not; the catalog's scenario count matches the tree.
2. **Automated-coverage test names exist** (Road B, or an extension of the binding model): each test fn cited in the automated-coverage matrix resolves in the pinned `specify-cli` checkout that `make lint` already builds via `scripts/specify.rs`. The matrix stops being prose; the REVIEW D2 class of rot becomes a lint finding.

## R4 — tier the gate

Add a `gate: release-blocker | full` axis (frontmatter field or catalog column — consistent with RFC-42's "one catalog, tags not forks"). The release-blocking set stays minimal — `pure-intent` plus one workspace scenario and one failure-path scenario — and must be green per release; the full catalog drains on a slower cadence (e.g. per minor, or monthly). This makes "the gate has never been green" a solvable statement instead of a permanent one.

## Execution plan

| Phase | Deliverable | Repo(s) | Effort | Depends |
| --- | --- | --- | --- | --- |
| 0 | Name sign-off (trials vs proving vs exercises); then the mechanical rename: CLI schema enum + `scenarios` tool rebuild, then directory/frontmatter/docs/lint-config sweep, same branch name both repos (CI sibling checkout is branch-matching) | both | S–M | operator decision |
| 1 | Probe taxonomy: extend RFC-42 Phase 0's assertion-id doc with probe-or-judgment per id; update run-template and agent runbook; journal-event gap list for unprobeable assertions | specify (+CLI for missing events) | M | none (can precede Phase 0) |
| 2 | Lint pinning: catalog↔runs cross-reference rule; automated-coverage test-name check against the pinned checkout | both | M | none |
| 3 | Gate tiering: `gate:` axis, blocking-set definition, drain the blocking set to green | specify | S + sweep time | Phase 0 (or lands pre-rename) |
| 4 | Category-2 promotions as RFC-44 R2 verbs land: `execute-build-failure`, `stepthrough-breakout`, `workspace-breakout` structural assertions promote toward fixture; `dual-driving-refused` promotes fully once refusal is CLI-enforced | both | M, paced by RFC-44 | RFC-44 Phase 2 |

Phases 1–3 are independent of each other; only Phase 4 blocks on RFC-44. Nothing here blocks RFC-42's enrichment phases — the probe taxonomy strengthens them.

## Non-Goals

- **No automated driving.** No runner, fake forge, recorded transcript, CI target, or golden-byte comparison — every `negative-expectation` is held. This RFC adds post-run grading only.
- **No machine prose grading.** Judgment-flagged assertions stay human/agent-judged with evidence pointers.
- **No catalog fork** and no new tiering mechanism beyond the `gate:` axis — RFC-42's principles stand; RFC-39's shape tier remains deferred and, if probes prove sufficient, may stay deferred indefinitely.
- **No lifecycle authority.** Probe output is evidence, never a transition.

## Open Questions

1. **Name sign-off.** `trials/` + "release proving" is the recommendation; `proving/` and `exercises/` are live alternates. Operator decision before Phase 0.
2. **Probe home.** Taxonomy doc first (RFC-42 OQ1's preference) — but should probes eventually be schema-carried (an `assertions[].probe` field) so the scenario file is self-contained? Current preference: doc first, schema once stable.
3. **Journal coverage.** Which assertions need taxonomy additions (e.g. transition actor for `gate-1-not-auto-stamped` variants)? Needs a one-pass gap analysis against the post-refactor event set.
4. **Rename scope.** Does `backend: live` ride the directory-rename PR pair, or land first as a schema-only change? Lockstep is simpler; staging is safer if the sweep is mid-drain.

## References

- [docs/contributing/evals.md](../docs/contributing/evals.md) — the two-surface model, the "what keeps a scenario manual" categories, and the agent runbook this RFC mechanises.
- [evals/scenarios/README.md](../evals/scenarios/README.md) — the catalog, status legend, and automated-coverage matrix.
- [evals/shared/run-template.md](../evals/shared/run-template.md) and [evals/shared/prompts.md](../evals/shared/prompts.md) — the grading surfaces Phase 1 touches.
- [RFC-42](rfc-42-acceptance.md) — capability axis, fixture-vs-manual rule, Phase 0 taxonomy this RFC extends.
- [RFC-39](future/rfc-39-acceptance-shape-traces.md) — the deferred shape tier; journal probes are its cheap substitute for structural assertions.
- [RFC-44](rfc-44-architecture-seams.md) — R2 (control flow into CLI verbs) drives Phase 4's promotions.
- [scenario.schema.json](https://github.com/augentic/specify-cli/blob/main/schemas/authoring/scenario.schema.json) — the `backend` enum Phase 0 changes.
- [REVIEW.md](../REVIEW.md) — D2/D3 accuracy fixes adjacent to (not duplicated by) this RFC.
