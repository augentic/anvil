# RFC-43: Release Proving

> Status: Draft · Serves: [RM-05](roadmap.md#rm-05-multi-repo-eval-suite) · Complements: [RFC-39](future/rfc-39-acceptance-shape-traces.md) (deferred shape tier), [RFC-44](rfc-44-architecture-seams.md) (the CLI-side levers), [RM-14](roadmap.md#rm-14-local-structured-workflow-events) (journal events)
> Provenance: findings from the 2026-06-11 architecture review of both repos. Sibling review: [REVIEW.md](../REVIEW.md). Revised 2026-06-11 after the `acceptance/` → `evals/` rename landed and RFC-42 was retired: F1/R1 are recorded as resolved, and the live proposals (R2–R4) are restated against the evals vocabulary. The open questions were resolved the same day; the decisions are inline in R2–R4.

## Abstract

This RFC originally proposed four improvements to what was then the `acceptance/` surface. The first — renaming the activity and retiring the `backend: manual` misnomer — has since been resolved by a different, deeper cut: the pack is `evals/`, the `backend` field is gone from the scenario schema entirely, the automated-coverage matrix is deleted, and deterministic proof lives only as named tests in `augentic/specify-cli` with no catalog entries. R1 records that outcome; "release proving" survives only as this RFC's working title — the on-disk name is **evals**.

Three proposals remain live: (R2) separate *who drives* a scenario from *who grades* it, giving every assertion id a mechanical probe (journal query or artifact check) and an explicit judgment flag for the irreducibly human residue; (R3) pin the hand-maintained catalog status surfaces with lint rules instead of editorial discipline; (R4) tier the release gate into a blocking core and a full-catalog cadence. The RFC adds no lifecycle authority and keeps every `negative-expectation` held.

## Motivation — findings

**F1 — "acceptance" was the wrong word, three ways (resolved).** It collided with the framework's artifact vocabulary (*acceptance criteria* in the `spec.md` sense — [requirement-block.md](../plugins/spec/references/synthesis/requirement-block.md)), implied customer UAT, and the docs' own prose had already abandoned it. Resolved by the `acceptance/` → `evals/` rename; see R1 for what landed versus what this RFC proposed.

**F2 — grading is unstructured (live).** The [agent runbook](../docs/contributing/evals.md) says "self-grade only the structurally checkable assertions", but no assertion defines its check. [pure-intent](../evals/scenarios/pure-intent.md) §Scope concedes that *every in-scope assertion is deterministic structure* — what needs a live agent is the skill loop, yet the grading of `plan-exists`, `plan-validates`, `sources-intent-only`, `refine-reaches-refined` is re-derived per run. The `negative-expectations` forbid an automated **runner**, fake forge, CI target, and golden bytes; none of them forbids a mechanical **grader** run after the sweep. The current posture conflates driving with grading.

**F3 — status is hand-maintained in three places and the rot is live (live).** Catalog status columns in [scenarios/README.md](../evals/scenarios/README.md), run-record filenames under [runs/](../evals/runs/README.md), and the RM-05 rollup each carry status by hand. Two live drifts today: the [RM-05 rollup](roadmap.md#rm-05-multi-repo-eval-suite) still says only `pure-intent` has passed while the catalog shows seven `passed` rows; and `pure-intent` sits `passed` in the catalog with no run record under `evals/runs/` (its record was deleted in the pack simplification, #153) even though the status legend defines `passed` as "run-summary filled". A third instance — the automated-coverage matrix citing a test fn that did not exist (REVIEW D2) — was resolved by deleting the matrix, which also shrank R3's scope.

**F4 — the gate as defined has never closed (live).** After the first sweep, 7 of the 14 catalog entries are `passed` and 7 are `pending` — and the pending tail is exactly the expensive half (live forge, workspace, breakout/failure paths). The gate ("every non-deferred entry `passed`" — [evals.md §The gate signal](../docs/contributing/evals.md#the-gate-signal)) is still all-or-nothing, so the whole catalog must drain per release even when the blocking question is only "does the core loop hold". New scenarios are admitted only through the three irreducibility categories, but each admission raises the cost of an undifferentiated gate.

**F5 — half the catalog is manual for architecture reasons (live).** The "skill-loop orchestration" admission category exists because park/resume/all-done decisions live in skill prose, not in a CLI verb. That is [RFC-44](rfc-44-architecture-seams.md) R2's territory; each verb migration shrinks the eval surface — and under the post-rename posture, a scenario whose every assertion becomes deterministic *leaves the catalog* for a named CLI test rather than being promoted to another tier.

## R2 — separate driving from grading

A scenario run stays agent-driven; its grading becomes mechanical. Every assertion id gets exactly one of:

- **A probe** — a deterministic post-run check: a CLI read verb, an artifact predicate, or a journal query. Probes never drive the workflow and never transition anything.
- **A judgment flag** — the irreducibly human residue (prose quality, ergonomics), graded by operator or agent with an evidence pointer, as today.

The journal is the natural probe substrate: lifecycle assertions are (or should be) journal events, and RFC-44 R3's journal read surface (`specify journal show --filter`, the lineage of RM-14's `events tail / export` target) is built to serve exactly these probes — the eval sweep is RM-14's first consumer. Illustrative mapping for `pure-intent`:


| Assertion id              | Probe sketch                                                       |
| ------------------------- | ------------------------------------------------------------------ |
| `plan-exists`             | `plan.yaml` present after `/spec:plan`                             |
| `plan-validates`          | `specify plan validate` exits 0                                    |
| `intent-single-lead`      | exactly one lead block under `## Lead inventory` in `discovery.md` |
| `gate-1-not-auto-stamped` | `plan.yaml` lifecycle is `pending` immediately after the skill run |
| `sources-intent-only`     | `Sources: [intent]` via `specify slice provenance`                 |
| `refine-reaches-refined`  | slice lifecycle `refined`; journal shows the transition event      |


**Journal coverage (audited, decided).** The gap pass does not wait on RFC-44 R3 — the read surface changes how probes *execute* (the journal is plain JSONL, so `jq` serves until `specify journal show --filter` lands), not which events *exist*. Audited against the closed taxonomy in [journal/event.rs](https://github.com/augentic/specify-cli/blob/main/crates/workflow/src/journal/event.rs): `slice.transition.refined` already covers `refine-reaches-refined`, and `plan.reconcile.completed` carries `slice_count` + `slice_names`, covering `intent-single-lead` and `multiple-slices-proposed` directly. Three additions close the rest, in priority order: (a) an `actor: operator | agent` field on `plan.transition.approved` — today it carries only `plan-name`, so `gate-1-not-auto-stamped` is probeable only via artifact state; (b) an entry-advance event where `specify plan next` writes per-entry `in-progress`, so Phase 4's park/resume probes can assert "parked and did not advance"; (c) workspace-verb events (`workspace sync` / `push`) for the workspace scenarios. Gaps are closed in the taxonomy by decision — never a reason to keep grading by hand.

RFC-42 (retired) proposed an assertion-id taxonomy doc; with its retirement **this RFC owns that proposal**: one reference doc mapping every assertion id (free-form kebab strings shared across scenario files, e.g. `plan-exists` / `plan-validates`) to *definition + probe (or judgment flag)* — an executable contract, not a glossary. The [run-template](../evals/shared/run-template.md)'s assertion table then cites probe output as evidence, and the agent runbook's "self-grade" step becomes "run the probes, judge the flagged residue".

**Probe home (decided).** The taxonomy stays a reference doc. If it later needs machine enforcement, it becomes one central file (e.g. `evals/shared/assertions.yaml`) plus a `reference-resolves`-shaped lint check that every id a scenario uses resolves there — never a per-scenario `assertions[].probe` field. Ids are deliberately shared across scenario files, so a per-scenario carrier would fork one id's definition across N files — the drift class R3 exists to kill. One authored home per fact; derive the rest.

The `negative-expectations` stand unchanged: `automated-runner-added`, `fake-forge-added`, `transcript-replay-added`, `ci-target-added`, `golden-output-required` all constrain *driving*; probes are post-run *grading*. If the distinction needs to be explicit, a one-line clarification in [docs/contributing/evals.md](../docs/contributing/evals.md) suffices — no new expectation id.

## R3 — pin the catalog with lint

Two checks, sized to what survives the matrix deletion:

1. **Catalog ↔ runs cross-reference** (Road A `cross-reference`): every catalog row's status agrees with the latest `runs/<id>.<result>.md` record; every row has a scenario file; the catalog's scenario count matches the tree (14 today). One contract change rides along (decided): **status-bearing rows require a committed record.** The "fully local run" allowance in [runs/README.md](../evals/runs/README.md) survives for triage and practice runs, but flipping a catalog status requires the filed record — the record is the write that licenses the status, which matches the legend's own definition of `passed` ("run-summary filled") and gives the rule full coverage instead of agreement-where-present. Consequence: `pure-intent` is re-run to restore its record; resurrecting the deleted `pure-intent-2026-06-05.md` would be stale evidence (it predates the extract-cache removal, lead reconciliation, and the `<id>.<result>.md` naming).
2. **Named-test citations resolve** — shrunk by the matrix deletion. What survives is a handful of prose citations of CLI test paths ([docs/contributing/evals.md](../docs/contributing/evals.md) and the RM-05 rollup cite `tests/plan/end_to_end.rs`, `tests/slice/synthesize.rs`, `tests/workflow/`, `tests/slice/build.rs`, `tests/workspace.rs`). Checking those against the pinned `specify-cli` checkout that `make lint` already builds is a small instance of [RFC-44](rfc-44-architecture-seams.md) R1's contract-dump cross-check; build it there, not as a bespoke rule here.

## R4 — tier the gate

The catalog already singles out `pure-intent` as the release-blocking hard halt; R4 generalizes that into a `gate: release-blocker | full` axis.

**Carrier (decided): a catalog column, not frontmatter.** A column is specify-repo-only and folds into R3's cross-reference rule (validate the value set alongside status); a frontmatter field forces a cross-repo lockstep — `scenario.schema.json` is `additionalProperties: false`, so it means a CLI schema bump plus a `scenarios` WASI-tool rebuild. The group tables keep carrying *execution order*; the column carries *gate tier* — orthogonal concerns, so blockers do not migrate between groups. Promote to frontmatter only if scenario files later need to be self-contained for another consumer.

**Membership (decided): `pure-intent`, `workspace-execute-two-projects`, `execute-build-failure`.** The workspace slot proves the happy multi-repo loop — RM-05's stated goal; the other workspace scenarios are failure/recovery variants that would double-count the failure slot. The failure slot takes `execute-build-failure`: parking on a build failure is the most operationally consequential failure mode (it protects against bad merges) and it is single-project, the cheapest of the candidates. `dual-driving-refused` is excluded because Phase 4 retires it once refusal is CLI-enforced. `workspace-execute-two-projects` is a live-forge scenario, so the per-release cost is real — accepted: a multi-repo control plane whose release gate never exercises a multi-repo flow proves the wrong thing.

The blocking set must be green per release; the full catalog drains on a slower cadence (e.g. per minor, or monthly). Today the set stands at 1 `passed` + 2 `pending`, so draining it to green (Phase 3) is exactly two runs. With the pending tail concentrated in the expensive live-forge / workspace / breakout scenarios (F4), tiering is what makes "the gate has never been green" a solvable statement instead of a permanent one.

## Execution plan


| Phase | Deliverable                                                                                                                                                                                                                                          | Repo(s)                       | Effort             | Depends        |
| ----- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------- | ------------------ | -------------- |
| 0     | ~~Name sign-off + mechanical rename~~ — **resolved**: landed as the `acceptance/` → `evals/` rename with `backend` deleted, independent of this RFC                                                                                                  | —                             | —                  | —              |
| 1     | Probe taxonomy: author the assertion-id doc (id → definition + probe-or-judgment); update run-template and agent runbook; land the three journal additions (transition actor, entry advance, workspace verbs)                                        | specify (+CLI for the events) | M                  | none           |
| 2     | Lint pinning: catalog↔runs cross-reference rule with committed records required for status-bearing rows (re-run `pure-intent` to restore its record); fold surviving named-test citations into RFC-44 R1's cross-check                               | both                          | M                  | none           |
| 3     | Gate tiering: add the `gate:` catalog column and drain the blocking set — two pending runs — to green                                                                                                                                                | specify                       | S + sweep time     | none           |
| 4     | Scenario retirements as RFC-44 R2 verbs land: `execute-build-failure`, `stepthrough-breakout`, `workspace-breakout` structural assertions become probes; `dual-driving-refused` leaves the catalog for a named CLI test once refusal is CLI-enforced | both                          | M, paced by RFC-44 | RFC-44 Phase 2 |


Phases 1–3 are independent of each other; only Phase 4 blocks on RFC-44.

## Non-Goals

- **No automated driving.** No runner, fake forge, recorded transcript, CI target, or golden-byte comparison — every `negative-expectation` is held. This RFC adds post-run grading only.
- **No machine prose grading.** Judgment-flagged assertions stay human/agent-judged with evidence pointers.
- **No catalog fork** and no new tiering mechanism beyond the `gate:` axis — [scenarios/README.md](../evals/scenarios/README.md) stays the single catalog; RFC-39's shape tier remains deferred (its `backend: shape` carrier no longer exists) and, if probes prove sufficient, may stay deferred indefinitely.
- **No lifecycle authority.** Probe output is evidence, never a transition.

## References

- [docs/contributing/evals.md](../docs/contributing/evals.md) — the two proof surfaces, the three admission categories, the gate signal, and the agent runbook this RFC mechanises.
- [evals/scenarios/README.md](../evals/scenarios/README.md) — the single catalog, groups, and status legend.
- [evals/runs/README.md](../evals/runs/README.md) and [evals/shared/run-template.md](../evals/shared/run-template.md) — the recording and grading surfaces Phases 1–2 touch.
- [scenario.schema.json](https://github.com/augentic/specify-cli/blob/main/schemas/authoring/scenario.schema.json) — the frontmatter contract; its free-form `assertions[]` ids are the strings the probe taxonomy defines.
- [journal/event.rs](https://github.com/augentic/specify-cli/blob/main/crates/workflow/src/journal/event.rs) — the closed journal event taxonomy the R2 gap list was audited against.
- [RFC-39](future/rfc-39-acceptance-shape-traces.md) — the deferred shape tier; probes are its cheap substitute for structural assertions.
- [RFC-44](rfc-44-architecture-seams.md) — R1 (contract dump) hosts the surviving test-name check, R2 (control flow into CLI verbs) drives Phase 4, R3 (journal read surface) serves the probes.
- [REVIEW.md](../REVIEW.md) — D2/D3 accuracy fixes adjacent to this RFC; D2's stale-matrix item was resolved by the matrix deletion.

