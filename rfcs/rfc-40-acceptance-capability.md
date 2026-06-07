# RFC-40: Acceptance Capability Coverage

> Status: Draft · Serves: [RM-05](../roadmap.md#rm-05-multi-repo-acceptance-suite) · Complements: [RFC-39](rfc-39-acceptance-shape-traces.md) (the `shape` tier these scenarios graduate through), [RFC-38 reconciliation polish](../roadmap.md#current-priorities) (the kernel several Phase 1 fixtures exercise)

## Abstract

The acceptance suite today is organised by **lifecycle-phase difficulty** — from N=1 through failure/breakout in [`acceptance/scenarios/`](../../acceptance/scenarios/README.md). That axis proves the `/spec:*` loop runs end-to-end, but it does not deliberately exercise the framework's distinct **capabilities** at depth: source→plan reconciliation, source→component synthesis-and-build, slice→baseline merge (composition and decision records), and target-project routing from a source synopsis. This RFC proposes a structured, phased plan to enrich acceptance along that **capability** axis without forking the catalog or weakening the deliberate `negative-expectations` posture.

The plan rests on one decision rule (fixture vs. manual), one repeatable authoring recipe, a coverage map of the named capabilities against today's catalog, and a four-phase rollout ordered by return on investment. It adds no new lifecycle authority: acceptance evidence remains evidence, never a transition.

## Motivation

`docs/contributing/acceptance.md` defines two acceptance surfaces — the deterministic CLI proof (`cargo make test` in `augentic/specify-cli`) and the manual operator sweep — and the three categories that keep a scenario manual (LLM-prose judgment, skill-loop orchestration, live-forge interaction). The catalog drains those surfaces by *phase*, which is the right shape for a release gate but the wrong shape for answering "how well do we test reconciliation?" or "how well do we test routing?".

Three forces make a capability-axis enrichment timely:

- **The bones are stable.** The reconciliation kernel (`crates/workflow/src/change/plan/core/propose.rs`), the synthesis path, the build envelope, and the routing projection (`resolve_topology` / `propose_from`) all exist and carry deterministic tests. Enrichment is now mostly pouring new scenarios into existing molds, not inventing structure.
- **Coverage is uneven by capability.** Reconciliation and routing are mature and mostly need more *deterministic edge cases* (cheap fixtures); the generated-output-correctness gate for real component builds is the genuinely thin surface; merge-into-decision-records mixes an existing capability (composition) with a possibly-new one (ADR generation) and needs a design decision before it can be tested.
- **Author discipline pays compound interest.** A written fixture-vs-manual rule and an assertion-id taxonomy keep the suite cheap to grow. Without them, scenario count grows faster than confidence.

## Principles

- **One catalog, capability tags — not a fork.** Keep `acceptance/scenarios/` as the single catalog. Express capability themes through a consistent `owner:` / id-prefix convention so scenarios are filterable by capability. Introduce sibling suite packs only if the catalog becomes unwieldy; do not split eagerly, because one catalog is easier to keep green.
- **Bias toward the deterministic surface.** Every new scenario forces the fixture-vs-manual decision. Prefer `backend: fixture` (a named test in `augentic/specify-cli`, run every commit) and keep only the irreducibly-prose, irreducibly-orchestration, or live-forge part manual.
- **Split a capability across surfaces rather than over-charging the sweep.** The existing `contract-routing` (fixture, deterministic routing) vs. `cross-repo-contract-flow` (manual, live-forge tail) pairing is the model: prove the deterministic half cheaply and reserve the manual half for what only a human or live agent can judge.
- **The CLI is authoritative.** Deterministic acceptance primitives (assertion evaluators, named tests, schemas) live in `augentic/specify-cli`; scenario files and fixtures live in `augentic/specify`.
- **No lifecycle authority.** Acceptance evidence never transitions a slice or stamps a plan; this RFC preserves that invariant.

## The fixture-vs-manual decision rule

Every new acceptance test classifies once, and the classification picks the repo it lives in:

- **All assertions reducible to deterministic CLI/host behaviour** → `backend: fixture`. Proof is a *named Rust test* in `augentic/specify-cli`; the `.md` under `acceptance/scenarios/` is a catalog stub with an **Automated coverage** section pointing at that test.
- **At least one assertion needs LLM-prose judgment, skill-loop orchestration, or a live forge** → `backend: manual`. Proven by the operator/agent sweep with `negative-expectations` held.

This is the single most load-bearing decision for each new scenario; make it *before* authoring. (RFC-39's `shape` tier, once landed, inserts an intermediate `backend: shape` rung for scenarios whose structural and orchestration assertions are machine-checkable but whose residual prose is not.)

## The authoring recipe

A repeatable loop for adding one scenario:

1. **Write the frontmatter** against the scenario schema ([`schemas/authoring/scenario.schema.json`](https://github.com/augentic/specify-cli/blob/main/schemas/authoring/scenario.schema.json) in `augentic/specify-cli`). Closed fields: `kind` (`suite` for these), `backend`, `entrypoint`, `stages` (a contiguous prefix of `[plan, refine, build, merge, drop]`), `isolation`. `assertions` / `expected-artifacts` are free-form kebab-case.
2. **Pick the backend** with the decision rule above.
3. **If fixture** — add or extend the named test in `augentic/specify-cli` (`tests/plan_orchestrate/`, `tests/plan/fan_in_fan_out.rs`, `tests/slice/synthesize.rs`, `tests/slice/build.rs`, `tests/workspace.rs`) and reference its corpus under `acceptance/fixtures/`. The `.md` carries the "Automated (`backend: fixture`)" callout plus an assertion→coverage map.
4. **If manual** — factor shared setup into [`shared/setup.md`](../../acceptance/shared/setup.md), inline only the scenario delta, and rely on the Prompt A / Prompt B [meta-prompts](../../acceptance/shared/meta-prompts.md) to drive it.
5. **Register it** in the [`acceptance/scenarios/README.md`](../../acceptance/scenarios/README.md) catalog (wave + status). That table is the single source of truth.
6. **Validate** — `make lint` checks frontmatter, id-uniqueness, artifact-path safety, and links.

## Capability coverage map

The honest picture today, so enrichment deepens rather than duplicates:

| Capability | Today | Gap to fill | Dominant backend |
| --- | --- | --- | --- |
| **1. Reconcile sources → plan** | Strong. `cross-source-merge` (manual), `combined-evidence` + `contract-routing` (fixture); kernel in `propose.rs` tested by `tests/plan_orchestrate/propose.rs` + `tests/plan/fan_in_fan_out.rs`. | 3+-source fan-in, *partial*-overlap merges, plan-time `--authority-override`, tentative-merge → amend → split. | fixture |
| **2. Synthesise sources → build a component** | Partial. `combined-evidence` / `divergence` / `conflict` cover synthesis; `fixtures/targets/{omnia,vectis}` cover the build *envelope*; `components.yaml` factoring exists. | The generated-output-correctness gate (real `cargo check` / `test` / replay) — the thinnest surface. The envelope + component-catalog factoring is fixture-able; the codegen correctness is not. | split (manual gate + fixture envelope) |
| **3. Merge specs/design → composition, decision records** | Thin. `fixtures/skills/merge/{success,conflict-replay}` exist; vectis `composition.yaml` has a build brief; "ADR" maps to `docs/explanation/decision-log.md`, **not a first-class Specify artifact**. | No dedicated lifecycle merge scenario beyond full flows. ADR-generation is not yet a real artifact — decide *specify-new-capability* vs. *test-existing-capability* before authoring. | RFC-first, then mixed |
| **4. Select target project from synopsis** | Decent. `contract-routing`, `multi-repo-workspace`, `workspace-execute-two-projects`; `resolve_topology` / `propose_from` deterministic. | Routing *ambiguity* and *mis-routing* (vague descriptions, two plausible targets, registry drift). | fixture (routing) + manual (synopsis reading) |

Headline: capabilities 1 and 4 are mature and want more deterministic edge cases (cheap); capability 2 needs a real generated-output gate (expensive, manual); capability 3 is partly a design question and should go through an RFC before it is tested.

## Phased rollout

### Phase 0 — instrument before enriching (once)

- **Assertion-id taxonomy.** Assertion ids are free-form kebab strings reused across files (`plan-exists`, `plan-validates`, …). Write them down (a short reference doc) before multiplying scenario count, so ids stay consistent and greppable.
- **Drain the existing catalog to green first.** `01-pure-intent` is the **hard halt** (N=1); it is now `passed` after being rescoped to `stages: [plan, refine]` so the gate rests only on deterministic plan/synthesis structure, not the non-deterministic codegen surface (capability 2 / Phase 2). No new manual scenario is meaningful while the N=1 scenario is red. This aligns with RM-05's immediate task.

### Phase 1 — deepen the deterministic floor (capabilities 1 & 4)

Highest ROI: every scenario runs on every commit with no sweep cost. Add reconciliation edge cases and routing-ambiguity cases as named tests in `augentic/specify-cli` with `backend: fixture` catalog stubs. Pure enrichment of mature kernels; complements RFC-38's reconciliation polish.

### Phase 2 — the generated-output gate (capability 2)

Stand up the "a slice is not done until the generated crate passes `cargo check` / `test` / replay" gate as real manual scenarios per target (omnia, vectis). This is the genuinely-missing surface and the one that proves *build a component*, not merely *build a valid envelope*. Inherently manual; pairs with a fixture-backed envelope scenario per target.

### Phase 3 — merge, composition, decision records (capability 3) — RFC-first

Write a short RFC deciding what "merge into decision records" means as an artifact (or that composition is the real target and the decision-log is the home for rationale). Then test the agreed capability. `rfcs/future/` is the home; RFC-39 is the adjacent precedent.

### Phase 4 — "and much more"

Once the taxonomy and the fixture-vs-manual discipline are habit, new capabilities slot in by repeating the authoring recipe. Candidate themes: drop/abandon paths, archive/retention GC, upgrade/migrate bootstrap lifecycle, and adapter-boundary negative scenarios.

## Non-Goals

- **No catalog fork.** This RFC does not split `acceptance/scenarios/` into per-capability packs by default; capability is a *tag*, not a directory, until the single catalog is demonstrably unwieldy.
- **No new tiering mechanism.** The `manual → shape → fixture` tiering and its primitives are RFC-39's scope; this RFC consumes them.
- **No prose-quality grading by machine.** Residual prose assertions stay human-judged.
- **No fake forge, no golden bytes.** The deliberate `fake-forge-added` and `golden-output-required` negative-expectations stay forbidden on every tier.
- **No lifecycle authority** from acceptance evidence.

## Open Questions

1. **Taxonomy home.** Should the assertion-id taxonomy live as a reference doc under `docs/contributing/`, or be enforced as a closed enum in the scenario schema? Current preference: doc first, enum once the set stabilises.
2. **Capability tagging mechanism.** Is an id-prefix convention enough to filter by capability, or does the frontmatter need an explicit `capability:` field? Current preference: id-prefix + `owner:` first; add a field only if filtering proves painful.
3. **Generated-output gate automation.** How much of the Phase 2 codegen-correctness gate can be promoted to `shape` (RFC-39) versus staying irreducibly manual? Revisit after the first per-target sweep.
4. **Decision-record artifact.** Does "ADR" become a first-class Specify artifact, or does composition plus the existing decision-log cover the intent? Phase 3 RFC decides.

## References

- [`docs/contributing/acceptance.md`](../../docs/contributing/acceptance.md) — the two-surface model and the "what keeps a scenario manual" categories.
- [`acceptance/scenarios/README.md`](../../acceptance/scenarios/README.md) — the scenario catalog, waves, and status legend.
- [`acceptance/shared/setup.md`](../../acceptance/shared/setup.md) and [`acceptance/shared/meta-prompts.md`](../../acceptance/shared/meta-prompts.md) — shared setup and the Prompt A / B operator aids.
- [`acceptance/scenarios/contract-routing.md`](../../acceptance/scenarios/contract-routing.md) and [`cross-repo-contract-flow.md`](../../acceptance/scenarios/cross-repo-contract-flow.md) — the fixture/manual split-by-surface precedent.
- [`schemas/authoring/scenario.schema.json`](https://github.com/augentic/specify-cli/blob/main/schemas/authoring/scenario.schema.json) — the scenario frontmatter contract.
- [RFC-39](rfc-39-acceptance-shape-traces.md) — the `shape` tier and promotion path these scenarios graduate through.
- [Specify Roadmap — RM-05](../roadmap.md#rm-05-multi-repo-acceptance-suite) — the acceptance-proof track this RFC serves.
