# RFC-40 — Composition accumulation and agent-driven component inference

## Status

Ready for review. A first-scan consistency review surfaced one material issue and four secondary accuracy fixes — see [Review findings (first scan)](#review-findings-first-scan). These are folded in below as discrete, numbered items (R1–R5) to be resolved one-by-one before the RFC is accepted.

## Motivation

Two design defects surfaced during end-to-end acceptance testing of the Vectis target on a multi-slice plan (14 slices, screenshots + documentation sources):

### Problem 1: Composition baseline destroyed by replace-not-accumulate merge

The composition merge engine (`crates/workflow/src/merge/composition.rs`) supports two document shapes:

- `**screens:` (full baseline)** — treated as a wholesale replacement of the existing baseline.
- `**delta: { added, modified, removed }` (incremental)** — screen-level operations applied to the existing baseline.

The [composition build brief](../adapters/targets/vectis/briefs/build/composition.md) instructs the agent to "Walk every `### Requirement:` block in `spec.md`" and regenerate `composition.yaml` from the slice's own `spec.md` + `design.md`. Since each slice carries only its own artifacts, the agent produces a standalone `screens:` document containing only the screens that slice introduces. At merge time, the CLI treats that as "this is the new baseline" and replaces whatever existed before.

In a 14-slice plan:

1. Each per-screen slice produced a composition with 1 screen.
2. Each merge replaced the baseline — not accumulated into it.
3. A late documentation-only slice (`platform-requirements`) generated `screens: {}` because it had no UI surface but the agent did not skip the composition step as instructed.
4. That empty document replaced the rich 8-screen composition the prior slice had deposited.
5. Final baseline: `version: 1\nscreens: {}` — total data loss.

The journal tells the story: every merge says "created baseline with N requirement(s)" rather than "added N screen(s)" — confirming the delta path was never used.

**Root causes:**

- The build brief does not instruct the agent to read the existing baseline composition and produce a superset or a delta.
- The merge engine has no guard against replacing a richer baseline with a poorer one.
- Non-UI slices are supposed to skip composition entirely ("When the slice has no UI surface at all… this step writes no `composition.yaml`") but this relies on agent compliance with no CLI-enforced gate.

### Problem 2: Component catalog is operator-curated when it should be agent-inferred

The current design positions the component catalog (`.specify/design-system/components.yaml`) as an **operator-curated, opt-in** file:

- The screenshots adapter's stage-6 emits `notes.candidate_component: <slug>` hints.
- The operator is expected to notice repeated structures across slices and manually create/edit the file.
- The Vectis build reads confirmed entries and factors shared code.

This contradicts the design intent established through the RFC process: component identification should be **agent-driven**, not operator-driven. The cross-slice memory problem that the catalog was meant to solve — the screenshots adapter having no memory across runs — is an agent orchestration problem, not a human curation problem. In practice:

- Operators never see the `notes.candidate_component` hints (they are buried in Evidence YAML).
- No skill or brief prompts the operator to create the catalog.
- The "opt-in" posture means the feature effectively never activates on first-time projects.
- The tab-bar-across-7-screens pattern (the canonical example in `plugins/spec/references/components.md`) is trivially detectable by an agent that reads the accumulated composition baseline.

## Non-goals

- Fixing up any specific generated project (test outputs have been discarded).
- Changing the `composition.yaml` schema or the Vectis WASI tool validator.
- Changing the merged baseline path (`.specify/specs/composition.yaml`).
- Removing operator override capability — operators can still reject or confirm components, but inference is the default path.
- Introducing composition generation into synthesis/refine (it remains a build-time concern).

## Design

### Part A: Composition accumulation

#### A1. Build brief reads baseline before regeneration

The [composition build brief](../adapters/targets/vectis/briefs/build/composition.md) gains a new input at priority 0 (above the existing priority 1–5 list):

> 1. `${PROJECT_DIR}/.specify/specs/composition.yaml` — the merged baseline composition. When present, the regeneration step produces an **accumulating composition**: it retains all existing baseline screens unchanged and adds, modifies, or removes only the screens this slice's `spec.md` positively references.

The regeneration algorithm (currently steps 1–9) is amended:

- **Step 1 (Identify screens)** now distinguishes **new screens** (slugs not in baseline) from **modified screens** (slugs already in baseline whose spec requirements have materially changed in this slice) and **removed screens** (baseline slugs the slice *explicitly* retires — see below). Screens present in the baseline but not referenced by this slice's spec are **carried forward unchanged**.
- **Removals are explicit, never inferred.** A baseline screen is removed *only* when the slice's own `spec.md` / `design.md` positively signals retirement (a requirement the slice removes or supersedes that owned that screen, or a design note deleting it). A screen simply not mentioned by this slice is **carried forward**, never removed — absence means "belongs to another slice," not "delete." When a retirement signal is present, the agent emits the slug under `delta.removed` (see A2/A2a).
- **Step 9 (Surface gaps)** adds: when the baseline contains screens not referenced by this slice, do not surface them as gaps — they belong to prior slices.

#### A2. Build brief emits delta format for non-bootstrap slices

When the baseline composition exists and is non-empty (`screens` map has ≥1 entry), the agent MUST write in `delta:` format:

```yaml
version: 1
delta:
  added:
    new-screen:
      name: New Screen
      maps_to: "ViewModel::NewScreen(NewScreenView)"
      # ... full screen entry
  modified:
    existing-screen:
      name: Existing Screen
      maps_to: "ViewModel::ExistingScreen(ExistingScreenView)"
      # ... full replacement for this screen
  removed: {}
```

When no baseline exists (first slice that introduces UI) or the baseline is empty (`screens: {}`), the agent writes the `screens:` format to establish the initial baseline.

This makes the merge engine's accumulation path the default rather than the replacement path.

#### A2a. Why an explicit delta envelope is necessary (accuracy over simplicity)

A simpler model was considered and rejected: drop the `added` / `modified` / `removed` distinction and treat every incoming slice composition as an implicit **sectional upsert** — each screen present in the incoming document replaces-or-inserts that screen in the baseline, and screens absent from the incoming document are carried forward unchanged. It trades away accuracy the workflow cannot recover, and accuracy must win here.

The root reason is that **slice boundaries do not map one-to-one onto composition screens**. A slice owns a set of requirements; the screens those requirements touch are an emergent property of synthesis, not something the slice declares up front. Because of that mismatch, the merge engine cannot reliably infer operation intent from screen presence/absence alone:

- **Changes are not always add/replace — removal is a first-class operation, and it is inexpressible under implicit upsert.** Absence-means-carry-forward gives no way to say "delete this screen." (Absence-means-replace-all is the opposite failure — the exact bug A3 guards against.) Only an explicit `removed:` set distinguishes "I did not touch this screen" from "this screen should be gone." Per A1, that `removed:` entry is emitted *only* on a positive retirement signal in the slice's own artifacts, never inferred from non-mention.
- **Cross-slice slug collisions go silent under implicit upsert.** When two independently authored slices each believe they are introducing screen `settings`, an upsert silently clobbers one with the other. The explicit envelope turns this into a loud error: `added` rejects a slug already in the baseline and `modified` rejects a slug absent from it (both raise `composition-screen-conflict` in `crates/workflow/src/merge/composition.rs`). These invariants catch real cross-slice accidents an upsert would bury.
- **Intent is authored, not inferred.** The agent knows whether it is adding, modifying, or removing; the delta envelope records that intent so the merge becomes a verification rather than a guess. This is the same principle as A3 — explicit authorisation over silent replacement.

So the answer to "does the incoming composition always imply a sectional replacement of the merged composition?" is **no**: a `modified` entry *is* a per-screen sectional replacement, but `added` vs `modified` vs `removed` carry intent and invariants a bare sectional replacement cannot. The delta envelope is exactly "sectional replacement **plus** authored intent."

**Merge granularity is the screen, and `modified` is whole-screen replacement.** The engine replaces the entire `screenEntry` for each `modified` slug (`crates/workflow/src/merge/composition.rs`); it does not merge sub-screen regions. Accuracy therefore depends on A1: the agent reads the baseline screen, applies its change, and emits a faithful **superset** of the unchanged regions for that screen. Single-screen reproduction is bounded and tractable — unlike the whole-document reproduction rejected under "Alternatives considered."

**Cross-cutting sub-screen contributions — two sub-cases.** They differ in blast radius:

1. **Factoring an already-present shared structure.** When prior-slice screens already render a structurally-identical group (e.g. an inlined tab-bar that recurs), inference discovers it and the build attaches a `component:` directive to those screens **inline** — directive-only `delta.modified`, behaviour-preserving, no full-document replacement. This is the common case and is handled by B7.
2. **Restructuring prior screens' layouts.** Genuinely adding or reshaping a region across many existing screens (e.g. introducing a tab-bar to screens that *lacked* one) is a layout change, not factoring; at screen granularity it means reproducing each affected screen in full. This is best routed through a dedicated refactoring slice taking the `--allow-composition-replace` path (A3) with a faithfully regenerated full document.

Region-level delta merge (merging `header` / `body` / `footer` / `states` / `overlays` independently) would reduce the reproduction burden for case (2), but requires a schema/merge-semantics change this RFC lists as a non-goal. Recorded as future work.

#### A3. CLI regression gate on composition merge

`specify slice merge` gains a new pre-merge precondition check (`composition-baseline-overwrite-blocked`):

- When the slice's `composition.yaml` uses the `screens:` format (whole-document replacement) AND a non-empty baseline already exists at `.specify/specs/composition.yaml`, the merge **aborts with a typed error** (`composition-baseline-overwrite-blocked`), consistent with the existing `composition-`* aborts the merge engine already raises (e.g. `composition-screen-conflict`). It is not surfaced as a non-blocking finding.
- Error message: "Slice composition uses whole-document replacement format but a non-empty baseline exists. Use `delta:` format, or pass `--allow-composition-replace` to authorise full replacement."
- The narrow, self-documenting `--allow-composition-replace` flag on `specify slice merge` is the **only** override, reserved for intentional full-baseline rewrites (e.g., a dedicated refactoring slice). A generic `--force` is deliberately **not** introduced: whole-document replacement is extremely rare (routine per-screen edits — add, modify, or remove — flow through `delta:` and never reach this gate), so there is no ergonomic case for a broad override, and a habitual `--force` would re-open the accidental-wipe vector this gate exists to close.

This makes it impossible for a non-UI slice to accidentally wipe the composition baseline, regardless of agent compliance.

#### A4. CLI enforcement: skip composition for non-UI slices

`specify slice build --phase finalize` gains a new validation check:

- When `proposal.md` declares only `core` in its `## Platforms` section AND the build report includes a `composition.yaml` output, emit a **warning diagnostic** (`composition-unexpected-for-core-only`).
- When the slice's `composition.yaml` contains `screens: {}` (explicitly empty) AND the proposal declares UI platforms, emit a **warning diagnostic** (`composition-empty-for-ui-slice`).

These are warnings (non-blocking) that surface agent non-compliance without halting the build.

### Part B: Agent-driven component inference

#### B1. Retire operator-curated posture; catalog becomes agent-written

The component catalog transitions from "operator-curated, opt-in" to **"agent-inferred, operator-reviewable"**. The file location, schema, and validation surfaces remain unchanged. What changes is **who writes it and when**.

#### B2. New CLI verb: `specify catalog infer`

A new deterministic CLI verb that reads the composition baseline and proposes catalog updates:

```bash
specify catalog infer [--dry-run] [--min-occurrences <N>]
```

**Algorithm:**

1. Load `.specify/specs/composition.yaml` (the merged baseline).
2. For every screen, extract the structural skeleton of each `group` subtree (strip `bind`, `event`, `*-when` values but retain the tree shape, item kinds, and nesting depth).
3. Compute a structural fingerprint (SHA-256 of the normalized skeleton) for each group.
4. Identify groups that appear across ≥N **screens** (default N=2; counted per screen, not per group instance — a group repeated within a single screen's list counts once) with identical structural fingerprints.
5. For each cluster of identical groups:
  - Derive a slug from the group's semantic content (e.g., `footer` group with `icon-button` items mapping to `Navigate(*)` events → `tab-bar`; repeating `card` with `checkbox` + `text` → `task-row`).
  - If the slug already exists in the catalog with `status: confirmed`, no action.
  - If the slug already exists with `status: rejected`, no action.
  - Otherwise, propose `status: confirmed` with an auto-generated description.
6. Write the updated catalog (or print the diff in `--dry-run` mode).

**Detection scope — `group` only.** The unit of detection is the `group`. The walk descends through `states` and `overlays`, so any structure wrapped in a `group` inside a state body or overlay content participates in inference; but `states` and `overlays` are not first-class detection units. The `component:` directive — the only factoring path — attaches solely to `groupProps`, so an inferred state/overlay pattern would have no wiring path, and a second fingerprint algorithm over those shapes would contradict the reuse mandate below and the schema-change non-goal. Factoring un-grouped state/overlay patterns is deferred to a future RFC once a schema mechanism (`component:` on `stateEntry` / `overlayEntry`) exists. See Open question 1.

**Slug derivation heuristic** (deterministic, not model-assisted):

- Groups in `footer` regions across multiple screens → `tab-bar` (or `bottom-nav` if the items are navigation-only).
- Groups in `body` regions that are `list` item templates → `<content-type>-row` (e.g., `task-row`, `list-row`).
- Groups containing a `card` with a fixed structure repeated across screens → `<card-purpose>-card`.

When the heuristic cannot derive a meaningful slug, emit `component-<fingerprint-prefix>` and mark with `description: "Auto-inferred; rename recommended."`.

**Identity is the structural fingerprint; the slug is a disambiguated label.** Two groups are the same component *iff* their fingerprints (step 3) match — never because their derived slugs match. The slug-derivation heuristic is lossy: distinct skeletons can map to the same name (e.g., two different structures both heuristically named `card-row`). Such collisions are resolved deterministically, not by merging:

- **Clustering and the candidate cache are keyed by fingerprint, not slug.** Distinct fingerprints are always distinct candidates, even when they derive the same slug; identical fingerprints are always one candidate, even across the cache and the baseline.
- **First-writer-wins for the bare slug.** The first fingerprint to claim a slug (already written to the catalog) keeps it; a later, distinct fingerprint deriving the same slug is suffixed with its fingerprint prefix (`card-row` → `card-row-<fp-prefix>`), reusing the fallback convention above. The suffix is fingerprint-derived, **never ordinal** (`-2`), so it is stable across runs — the same skeleton always yields the same slug. A first-ever run with a simultaneous tie breaks by lexicographic fingerprint order.
- This honours the downstream invariant: the composition validator's `check_structural_identity` rejects two different skeletons sharing one `component: <slug>` directive (`wasi-tools/vectis/src/validate/engine/composition.rs`), so inference must never emit a colliding slug in the first place.
- Operators rename auto-suffixed slugs (B5); B6's no-overwrite rule keeps the rename stable on subsequent runs.

**Implementation placement — reuse the existing skeleton engine, do not re-derive it.** Steps 2–4 (skeleton normalization + structural fingerprint) are already implemented in the vectis WASI tool at `wasi-tools/vectis/src/validate/engine/composition.rs` — `build_group_skeleton`, `check_structural_identity`, and `walk_for_components` strip `bind` / `event` / `error` / `*-when` / asset / token / free-text values and compare the residual group skeleton across instances. This is the same normalization the inference verb needs. The verb MUST reuse that logic rather than re-deriving a second skeleton algorithm in the host crate (a divergence between the two would let inference propose a `component: <slug>` directive that the composition validator then rejects under its own structural-identity rule). Two viable shapes, to be settled before Phase 2:

- **(preferred) Add an `infer` mode to the vectis tool** alongside its existing `verify` (`--mode detect` / `--mode verify`) subcommand, and have the host invoke it via `specify tool run vectis -- infer …`. This keeps all skeleton logic inside the WASI carve-out.
- **Extract the skeleton normalization** into a shared crate consumed by both the vectis tool and a host-side `specify catalog infer`.

The cross-repo touchpoints below assume the host-side verb shape; if the tool-subcommand shape is chosen, `infer.rs` becomes a thin `specify tool run` dispatch and the algorithm lands in `wasi-tools/vectis/`.

#### B3. Build brief invokes inference before composition regeneration

The [build brief phase order](../adapters/targets/vectis/briefs/build.md) gains a step 0.5 between the current "load composition.md" and the regeneration:

1. Run `specify catalog infer --dry-run` against the current baseline.
2. If new components are proposed, run `specify catalog infer` (non-dry-run) to update the catalog.
3. Proceed with composition regeneration (which now reads the updated catalog at step 6).

This makes component detection a **build-time, agent-driven, deterministic** process rather than an operator-initiated one.

#### B4. Screenshots adapter stage-6 feeds forward into catalog inference

Stage-6 `notes.candidate_component` hints currently dead-end at Evidence and surface as `[unknown]` tags. Under this RFC:

- Stage-6 gains an additional output: when the hint is emitted, the adapter also writes a structured sidecar entry to `.specify/.cache/component-candidates/<fingerprint>.yaml` recording the structural skeleton that triggered the hint. **The cache is keyed by structural fingerprint, not slug** — keying by `<slug>.yaml` would let two distinct skeletons that derive the same heuristic slug silently overwrite each other (the same clobber failure mode A3 guards against in the composition baseline). The candidate's derived slug is stored *inside* the entry as a label, alongside the skeleton.
- `specify catalog infer` reads both the composition baseline AND the candidate cache, using cached skeletons as supplementary evidence for cross-slice structural identity. Candidates are deduplicated and clustered by fingerprint (see B2 "Identity is the structural fingerprint"), so a cached skeleton and a baseline group with the same fingerprint are recognised as one component.

This gives the inference verb memory across slices even before the composition baseline accumulates the screens — it can detect shared structures from extraction evidence before those structures reach the composition.

#### B5. Operator review surface (preserved, not removed)

The operator retains:

- The ability to set `status: rejected` on any entry — this permanently suppresses that slug.
- The ability to rename auto-inferred slugs before the next build.
- Visibility into what was inferred via `specify catalog infer --dry-run` (read-only inspection).
- The `slice-catalog-drift` finding on `specify slice validate` (unchanged).
- The composition validator's catalog cross-reference (check 5, unchanged).

The change is directional: inference proposes `confirmed` by default; operators demote to `rejected`. This is the inverse of the current model where operators must promote from nothing.

#### B6. Migration from current operator-curated model

For projects with an existing `components.yaml`:

- Existing `confirmed` entries are preserved unchanged.
- Existing `rejected` entries are preserved unchanged — `specify catalog infer` never overwrites a `rejected` entry.
- New entries from inference are appended with `status: confirmed`.
- No entries are removed by inference (only additions).

For projects without `components.yaml`:

- First `specify catalog infer` run creates the file if any components are detected.
- If no components are detected (single-screen app, no repeated structures), the file is not created — preserving the current "absent catalog = no factoring" behavior.

#### B7. Retroactive cross-slice factoring (modifying prior-slice screens and code)

Component inference is incremental (B3): a shared component only becomes detectable once ≥N screens carrying it exist in the accumulated baseline, and those screens are typically introduced by *different* slices. So at the build where the Nth screen lands, inference promotes a component (e.g. `tab-bar`) whose other instances live in screens authored by *prior* slices. The build that makes the discovery MUST be able to fold it back into those prior screens — both their composition entries and their generated code. This is the resolution of Open question 5: **yes, a slice may modify baseline screens it did not author**, for this specific purpose, without a dedicated refactoring slice.

**Composition side.** For each baseline screen outside the current slice's units that carries a group structurally identical (same fingerprint) to the newly promoted component, the build emits a `delta.modified.<screen>` entry reproducing that prior screen as a faithful superset (A1/A2a) with a `component: <slug>` directive attached to the matching group. The modification is **directive-only**: the sole permitted change to a not-authored baseline screen is attaching (or detaching) a `component:` directive to a group whose skeleton already matches the factored component. Restructuring a prior screen's layout is out of scope for inline factoring and stays on the dedicated-refactoring-slice path (A2a case 2, A3). The directive-only constraint is what makes this safe — the structural-identity invariant (`check_structural_identity`) guarantees the directive changes *factoring*, never *rendering*.

**Code side.** When the catalog gains a confirmed component referenced by baseline screens outside the current slice's units, the writer sub-briefs (`build/core/write.md`, `build/ios/write.md`, `build/android/write.md`) — which run in `update` mode and already edit the live shell tree rather than a slice sandbox — MUST: (a) generate the shared component (`shared/src/components/<slug>.rs`, iOS `Components/<Slug>View.swift`, Android `components/<Slug>Component.kt`); and (b) refactor the affected prior screens' generated views to consume the shared component in place of the inlined structure. Because the skeletons are identical by construction, the refactor is behaviour-preserving; the per-platform verify-repair loops and reviewers catch any regression.

**Why this reconciles cleanly (no cross-branch hazard).** `/spec:execute` drives slices sequentially under an exclusive plan lock, and each slice merges into the baseline before the next begins. So when slice N builds, every prior slice's screens and code are already merged into the project tree; slice N edits the *current* state, not a divergent branch — there is no concurrent-edit / merge-conflict hazard between slices for this refactor. Idempotence holds: once the directive and shared code land, a later build's inference sees the component as already `confirmed` (B6 no-overwrite) and the prior screens already carrying the directive, so re-runs are no-ops.

## Implementation plan

### Phase 1 — Composition accumulation (critical path)

1. **Brief amendment.** Update `adapters/targets/vectis/briefs/build/composition.md` with the baseline-reading and delta-format instructions (A1, A2).
2. **CLI regression gate.** Implement `composition-baseline-overwrite-blocked` check in `specify slice merge` with the `--allow-composition-replace` escape hatch (A3).
3. **CLI warnings.** Implement `composition-unexpected-for-core-only` and `composition-empty-for-ui-slice` in `specify slice build --phase finalize` (A4).
4. **Tests.** Extend `tests/fan_in_fan_out.rs` with a multi-slice composition accumulation scenario asserting the baseline grows monotonically across screen-introducing slices.

### Phase 2 — Component inference

1. `**specify catalog infer` verb.** Implement the structural-fingerprint algorithm against `composition.yaml` baseline. Land in `src/runtime/commands/catalog/infer.rs` alongside tests under `tests/catalog_infer.rs`.
2. **Brief amendment.** Update `adapters/targets/vectis/briefs/build.md` to invoke `specify catalog infer` before composition regeneration (B3).
3. **Candidate cache.** Update the screenshots adapter pipeline brief (stage 6) to write structural skeletons to `.specify/.cache/component-candidates/` keyed by fingerprint (B4). Update `specify catalog infer` to read from the cache.
4. **Retroactive factoring briefs.** Update `build/composition.md` to emit directive-only `delta.modified` for prior-slice screens that match a newly promoted component, and update the writer sub-briefs (`build/core/write.md`, `build/ios/write.md`, `build/android/write.md`) to generate the shared component and refactor the affected prior screens' views (B7).
5. **Documentation.** Rewrite the canonical runtime explainer `plugins/spec/references/components.md` to reflect the agent-inferred model (the `docs/explanation/components.md` mdBook stub and the `adapters/sources/screenshots/references/spec-runtime/components.md` symlink both resolve here — do not edit them directly).

### Phase 3 — Acceptance

1. **Acceptance scenario.** Add a cross-repo acceptance scenario exercising: 3+ slices each introducing a screen with a shared tab bar → assertion that the catalog is auto-populated after the second slice's build, the prior screens are retroactively given the `component:` directive and refactored to consume the shared component (B7), and composition accumulates correctly across all slices.

## Migration

Phase 1 is **schema-compatible**: no changes to `composition.yaml` format, `plan.yaml`, or any existing schema. The brief amendment is advisory (agents read it on next invocation); the CLI gate is additive.

Phase 2 is **additive**: `specify catalog infer` is a new verb; the candidate cache is a new directory under `.specify/.cache/`; catalog inference adds entries but never removes them.

**Breaking change:** The posture flip from "operator-curated, opt-in" to "agent-inferred, operator-reviewable" is a documentation and workflow expectation change. Existing projects with `status: rejected` entries are unaffected (those entries are respected). Projects relying on the absent-catalog-means-no-factoring guarantee will see component factoring activate once any shared structures exist — this is the intended improvement.

## Alternatives considered

**Require agents to always produce full `screens:` documents including baseline screens.** Rejected. This requires the agent to faithfully reproduce potentially hundreds of baseline screens it didn't author, inviting transcription errors and bloating context windows. The `delta:` format is purpose-built for this.

**Make the merge engine detect and prevent regressions heuristically (e.g., refuse to shrink the screen count).** Rejected as too brittle. A legitimate refactoring slice might remove screens. The right answer is the `delta:` format contract plus an explicit override flag for intentional replacements.

**Keep component catalog operator-curated but add a CLI suggestion command.** Rejected as insufficient. The suggestion-only model is what `notes.candidate_component` already provides today, and it demonstrably does not work — operators don't see the hints, don't act on them, and the feature never activates. The inference must be **active by default**.

**Run component inference at refine time (during synthesis).** Rejected. Synthesis is platform-neutral and does not read `composition.yaml`. Component detection requires spatial structure that only exists after composition regeneration. Build time is the correct moment.

**Use model-assisted (LLM) component detection instead of structural fingerprinting.** Rejected for the deterministic path. The structural-fingerprint algorithm is deterministic, reproducible, and auditable. Model-assisted judgment is appropriate as a supplementary layer (e.g., for slug naming when the heuristic fails) but should not be the primary detection mechanism for a feature that affects code generation.

## Cross-repo touchpoints


| Change                                   | Repo        | Files                                                                                                                                                                                                                                            |
| ---------------------------------------- | ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Composition brief amendment (A1, A2)     | specify     | `adapters/targets/vectis/briefs/build/composition.md`                                                                                                                                                                                            |
| Build brief step 0.5 (B3)                | specify     | `adapters/targets/vectis/briefs/build.md`                                                                                                                                                                                                        |
| Retroactive cross-slice factoring (B7)   | specify     | `adapters/targets/vectis/briefs/build/composition.md` (directive-only `delta.modified` for prior-slice screens), `adapters/targets/vectis/briefs/build/{core,ios,android}/write.md` (refactor prior-slice views to consume the shared component) |
| Screenshots stage-6 candidate cache (B4) | specify     | `adapters/sources/screenshots/briefs/extract/pipeline.md`                                                                                                                                                                                        |
| Components explanation rewrite (B6)      | specify     | `plugins/spec/references/components.md` (canonical; `docs/explanation/components.md` + screenshots `spec-runtime/components.md` symlink resolve here)                                                                                            |
| Composition regression gate (A3)         | specify-cli | `crates/workflow/src/merge/composition.rs`, `crates/workflow/src/merge/slice.rs` (apply path), `src/runtime/commands/slice/merge.rs`                                                                                                             |
| Build finalize warnings (A4)             | specify-cli | `src/runtime/commands/slice/build.rs`                                                                                                                                                                                                            |
| `specify catalog infer` verb (B2)        | specify-cli | `src/runtime/commands/catalog/infer.rs` (new) — thin dispatch if the vectis-tool-subcommand shape is chosen (see B2 "Implementation placement")                                                                                                  |
| Skeleton engine reuse (B2)               | specify-cli | `wasi-tools/vectis/src/validate/engine/composition.rs` (`build_group_skeleton`, `check_structural_identity`) — reuse, do not re-derive                                                                                                           |
| Candidate cache reader                   | specify-cli | `crates/workflow/src/design_system.rs` (extend — load-only today; B2/B6 add the writer)                                                                                                                                                          |
| Fan-in/fan-out composition test          | specify-cli | `tests/fan_in_fan_out.rs`                                                                                                                                                                                                                        |
| Catalog inference tests                  | specify-cli | `tests/catalog_infer.rs` (new)                                                                                                                                                                                                                   |


## Open questions

1. **Resolved (limit to `group`).** `specify catalog infer` limits detection to `group` structural identity, reusing the existing skeleton engine (`build_group_skeleton` / `check_structural_identity`) verbatim per B2. The walk still descends through `states` and `overlays`, so any structure wrapped in a `group` inside a state body or overlay content participates in inference; the *unit of detection* is always the group. `states` and `overlays` are **not** treated as first-class detection units, because (a) the `component:` directive — the only factoring path — attaches solely to `groupProps`, and (b) adding a second fingerprint algorithm over the state/overlay shapes would contradict the "do not re-derive the skeleton" mandate and the schema-change non-goal. Factoring un-grouped state/overlay patterns is deferred to a future RFC once a schema mechanism (`component:` on `stateEntry` / `overlayEntry`) exists.
2. **Resolved (default N=2).** `--min-occurrences` defaults to 2, matching the screenshots stage-6 promotion threshold (`pipeline.md` ≥2) and the B4 candidate cache, so the hint threshold and the inference threshold stay aligned. It also satisfies the Phase 3 acceptance scenario, which asserts the catalog auto-populates after the *second* slice's build. Counting is by screens, not raw group instances, so a row repeated within a single screen's list does not count toward the threshold. The false-positive risk from B1's confirmed-by-default flip is bounded by exact structural-fingerprint identity, content/region-aware slug heuristics, the B3 `--dry-run` preview, and the operator `reject` path; projects wanting higher confidence pass `--min-occurrences 3` per invocation.
3. **Resolved (hard error, narrow override).** The A3 gate aborts the merge with a typed error (`composition-baseline-overwrite-blocked`), consistent with the existing `composition-`* aborts in the merge engine — not a non-blocking finding. The **only** override is the narrow, self-documenting `--allow-composition-replace` flag; no generic `specify slice merge --force` is introduced. Rationale: whole-document replacement is extremely rare (routine per-screen add/modify/remove flows through `delta:` and never reaches the gate), so there is no ergonomic case for a broad override, and a habitual `--force` would re-open the accidental-wipe vector the gate closes. See A3, and A2a for why the explicit delta envelope (including first-class, explicit `removed`) is necessary rather than an implicit sectional upsert.
4. **Resolved (fingerprint is identity; slug is a disambiguated label).** Identity is the structural fingerprint, not the slug. The candidate cache is keyed by `<fingerprint>.yaml` (not `<slug>.yaml`, which would silently clobber), and clustering deduplicates by fingerprint. When two distinct fingerprints derive the same heuristic slug, first-writer-wins keeps the bare slug and later fingerprints are suffixed with a stable, fingerprint-derived prefix (`card-row-<fp-prefix>`, never an ordinal), reproducible across runs. This respects the downstream `check_structural_identity` invariant (one skeleton per `component: <slug>`), so inference never emits a colliding slug. Operators rename auto-suffixed slugs (B5), kept stable by B6's no-overwrite rule. See B2 "Identity is the structural fingerprint" and the B4 cache-key change.
5. **Resolved (yes — directive-only inline factoring; see B7).** A slice may modify baseline screens it did not author, for the specific purpose of folding in a cross-slice component discovery, without a dedicated refactoring slice. Inference is incremental, so the build that detects the Nth instance of a shared structure attaches the `component:` directive to the prior-slice screens (directive-only `delta.modified`, structurally identical by construction, behaviour-preserving) and refactors their generated code to consume the newly factored shared component. This reconciles cleanly because `/spec:execute` runs slices sequentially under an exclusive plan lock — prior screens and code are already merged into the project tree when slice N builds, so there is no cross-branch conflict. **Restructuring** a prior screen's layout (as opposed to attaching a directive to an already-matching group) remains out of scope for inline factoring and stays on the dedicated-refactoring-slice path (A2a case 2, A3).

## Review findings

### R1 — A4's "core-only slice" premise contradicts how `proposal.md ## Platforms` is populated (blocking)

**Affects:** §A4, Problem 1 point 3, the `composition-unexpected-for-core-only` check.

§A4 (and its framing of Problem 1, point 3) assumes `proposal.md ## Platforms` reflects *this slice's* UI involvement, so a non-UI slice would "declare only core". The current contract says the opposite, emphatically and in four places:

- `adapters/targets/vectis/briefs/shape.md` (line 23): "Read `project.yaml.platforms` directly and stamp the full set verbatim — do not cherry-pick or trim per slice."
- `shape.md` (line 15) and `build.md` (line 41): platforms are an "app-level fact … carried verbatim to every slice … not per-slice opt-in."
- `docs/reference/targets/vectis.md` (line 62): "stamped verbatim from `project.yaml.platforms` (not per-slice opt-in)."

Consequences:

- `**composition-unexpected-for-core-only` is effectively inert for the motivating scenario.** The documentation-only `platform-requirements` slice in a 14-slice iOS+Android plan has `## Platforms: core, ios, android`, not `core`. The A4(a) check only ever fires for a genuinely core-only *app*, never a non-UI slice in a multi-platform app.
- **The RFC mis-attributes the root cause.** Problem 1 point 3 blames "the agent did not skip the composition step as instructed." But the instruction it relies on — `composition.md` line 43, "Detect by checking whether `proposal.md` lists any non-`core` platform; when only `core` is present, skip" — is **structurally unreachable** for a non-UI slice in a multi-platform app. The agent was effectively instructed *not* to skip. This is a contract defect, not (only) agent non-compliance.

What this means for the plan: A3 (the merge gate) still catches the actual data-loss, because `screens: {}` is the `screens:` format and the baseline is non-empty → `composition-baseline-overwrite-blocked` fires regardless of platforms. So Phase 1's safety net holds. But A4 needs a real per-slice "has UI surface" signal (`proposal ## Platforms` is **not** one), and the Problem 1.3 root-cause prose must be corrected.

**Resolution options to settle in the RFC** (pick one for A4's per-slice "has UI surface" signal — but *not* `## Platforms`):

- Derive the signal from the build report `outputs[]` / `composition` presence.
- Derive it from whether `spec.md` yields any screens.
- Introduce a new per-slice marker.

### R2 — A3 call-site and flag plumbing are understated in the Cross-repo touchpoints table (non-blocking)

**Affects:** §A3, Cross-repo touchpoints table.

The table lists `merge/composition.rs`, `merge/slice.rs`, `slice/merge.rs`. But the actual composition-merge invocation is in `crates/workflow/src/merge/slice/read.rs::merge_composition_delta` (not `slice.rs` directly), and `composition::merge` currently takes only `(baseline, delta_text)` with no baseline-empty / override awareness — today the `has_screens && !has_delta` branch in `composition.rs` unconditionally returns `CreatedBaseline`, ignoring whether the baseline is non-empty. The `--allow-composition-replace` flag has to thread CLI handler → `slice::commit` → `plan_three_way` → `merge_composition_delta` → `composition::merge`.

**Resolution:** add `crates/workflow/src/merge/slice/read.rs` to the table and acknowledge the flag-threading path through the merge call chain.

### R3 — B2 overstates "fingerprint already implemented" (non-blocking)

**Affects:** §B2 (step 3, "Implementation placement"), §B4 cache key.

The WASI engine (`wasi-tools/vectis/src/validate/engine/composition.rs`) compares the `Skeleton` enum by `PartialEq` (`#[derive(Debug, Eq, PartialEq, Clone)]`) — there is **no** SHA-256 fingerprint. B2 step 3 ("SHA-256 of the normalized skeleton") and the B4 cache key `<fingerprint>.yaml` both require **adding** a canonical serialization + hash over `Skeleton`. "Reuse the skeleton normalization" is correct; "the structural fingerprint is already implemented" is not.

**Resolution:** reword B2's "Implementation placement" to "skeleton normalization is implemented; fingerprinting is a thin addition over it," and make explicit that canonical serialization + hashing of `Skeleton` is new work.

### R4 — B4 fingerprint-coherence gap (non-blocking)

**Affects:** §B4.

Screenshots stage-6 is an agent/vision brief (`extract/pipeline.md`), not the deterministic WASI tool. Having it write `.specify/.cache/component-candidates/<fingerprint>.yaml` means an **agent-computed** fingerprint that must byte-match the tool's **canonical** fingerprint, or the "identity is the fingerprint" guarantee breaks across the cache↔baseline boundary (and pre-composition skeletons are not even in `composition.yaml` shape yet). The RFC does not say how stage-6 obtains a tool-consistent fingerprint.

**Resolution:** add a sentence to B4 — e.g. the cache stores the **normalized skeleton** and `specify catalog infer` computes the canonical fingerprint **at read time**, rather than trusting an agent-written filename.

### R5 — Doc-inversion sweep must extend beyond `components.md` (non-blocking)

**Affects:** Phase 2 step 9, Cross-repo touchpoints (B6 row).

`plugins/spec/references/components.md` currently states, under "What the catalog does not do": "No auto-population — operator-curated only" and "No retroactive baseline rewrite without a refactor slice" — both directly inverted by B1/B7. Phase 2 step 9 already calls for rewriting this file; the rewrite must **remove those two bullets**. Additionally, the merge-brief / `build.md` lines describing the catalog as "operator-curated, opt-in" (e.g. `build/composition.md` step 6, and `build.md` lines 7 / 22) should be swept in the **same pass**, not only `components.md`.

**Resolution:** broaden Phase 2 step 9 (and the B6 touchpoints row) to enumerate the additional brief locations and the two specific bullets to delete.

## References

- [Composition build brief](../adapters/targets/vectis/briefs/build/composition.md) — the current regeneration algorithm.
- [Merge brief](../adapters/targets/vectis/briefs/merge.md) — Vectis-specific merge gates.
- [Component catalog explanation](../plugins/spec/references/components.md) — current operator-curated model (canonical runtime file; `docs/explanation/components.md` is an mdBook stub redirecting here).
- [Screenshots pipeline stage 6](../adapters/sources/screenshots/briefs/extract/pipeline.md) — current conservative detection.
- `[crates/workflow/src/merge/composition.rs](https://github.com/augentic/specify-cli/blob/main/crates/workflow/src/merge/composition.rs)` — merge engine supporting `screens:` and `delta:` shapes.
- [Layout inferer contract](../adapters/targets/vectis/references/layout-inferer-contract.md) — structural identity rules for component detection.

