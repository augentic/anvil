# RFC-40 — Composition accumulation and agent-driven component inference

## Status

Draft.

## Motivation

Two design defects surfaced during end-to-end acceptance testing of the Vectis target on a multi-slice plan (14 slices, screenshots + documentation sources):

### Problem 1: Composition baseline destroyed by replace-not-accumulate merge

The composition merge engine (`crates/workflow/src/merge/composition.rs`) supports two document shapes:

- **`screens:` (full baseline)** — treated as a wholesale replacement of the existing baseline.
- **`delta: { added, modified, removed }` (incremental)** — screen-level operations applied to the existing baseline.

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
- The tab-bar-across-7-screens pattern (the canonical example in `docs/explanation/components.md`) is trivially detectable by an agent that reads the accumulated composition baseline.

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

> 0. `${PROJECT_DIR}/.specify/specs/composition.yaml` — the merged baseline composition. When present, the regeneration step produces an **additive composition**: it retains all existing baseline screens unchanged and adds/modifies only screens whose requirements appear in the current slice's `spec.md`.

The regeneration algorithm (currently steps 1–9) is amended:

- **Step 1 (Identify screens)** now distinguishes **new screens** (slugs not in baseline) from **modified screens** (slugs already in baseline whose spec requirements have materially changed in this slice). Screens present in the baseline but not referenced by this slice's spec are **carried forward unchanged**.
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

#### A3. CLI regression gate on composition merge

`specrun slice merge` gains a new pre-merge validation check (`composition-regression-guard`):

- When the slice's `composition.yaml` uses the `screens:` format (full replacement) AND a non-empty baseline already exists at `.specify/specs/composition.yaml`, emit a **blocking diagnostic** (`composition-baseline-overwrite-blocked`).
- The diagnostic message: "Slice composition uses full-replacement format but a non-empty baseline exists. Use `delta:` format or pass `--allow-composition-replace` to force replacement."
- An explicit `--allow-composition-replace` flag on `specrun slice merge` overrides the guard for intentional full-baseline rewrites (e.g., a dedicated refactoring slice).

This makes it impossible for a non-UI slice to accidentally wipe the composition baseline, regardless of agent compliance.

#### A4. CLI enforcement: skip composition for non-UI slices

`specrun slice build --phase finalize` gains a new validation check:

- When `proposal.md` declares only `core` in its `## Platforms` section AND the build report includes a `composition.yaml` output, emit a **warning diagnostic** (`composition-unexpected-for-core-only`).
- When the slice's `composition.yaml` contains `screens: {}` (explicitly empty) AND the proposal declares UI platforms, emit a **warning diagnostic** (`composition-empty-for-ui-slice`).

These are warnings (non-blocking) that surface agent non-compliance without halting the build.

### Part B: Agent-driven component inference

#### B1. Retire operator-curated posture; catalog becomes agent-written

The component catalog transitions from "operator-curated, opt-in" to **"agent-inferred, operator-reviewable"**. The file location, schema, and validation surfaces remain unchanged. What changes is **who writes it and when**.

#### B2. New CLI verb: `specrun catalog infer`

A new deterministic CLI verb that reads the composition baseline and proposes catalog updates:

```bash
specrun catalog infer [--dry-run] [--min-occurrences <N>]
```

**Algorithm:**

1. Load `.specify/specs/composition.yaml` (the merged baseline).
2. For every screen, extract the structural skeleton of each `group` subtree (strip `bind`, `event`, `*-when` values but retain the tree shape, item kinds, and nesting depth).
3. Compute a structural fingerprint (SHA-256 of the normalized skeleton) for each group.
4. Identify groups that appear across ≥N screens (default N=2) with identical structural fingerprints.
5. For each cluster of identical groups:
   - Derive a slug from the group's semantic content (e.g., `footer` group with `icon-button` items mapping to `Navigate(*)` events → `tab-bar`; repeating `card` with `checkbox` + `text` → `task-row`).
   - If the slug already exists in the catalog with `status: confirmed`, no action.
   - If the slug already exists with `status: rejected`, no action.
   - Otherwise, propose `status: confirmed` with an auto-generated description.
6. Write the updated catalog (or print the diff in `--dry-run` mode).

**Slug derivation heuristic** (deterministic, not model-assisted):

- Groups in `footer` regions across multiple screens → `tab-bar` (or `bottom-nav` if the items are navigation-only).
- Groups in `body` regions that are `list` item templates → `<content-type>-row` (e.g., `task-row`, `list-row`).
- Groups containing a `card` with a fixed structure repeated across screens → `<card-purpose>-card`.

When the heuristic cannot derive a meaningful slug, emit `component-<fingerprint-prefix>` and mark with `description: "Auto-inferred; rename recommended."`.

#### B3. Build brief invokes inference before composition regeneration

The [build brief phase order](../adapters/targets/vectis/briefs/build.md) gains a step 0.5 between the current "load composition.md" and the regeneration:

1. Run `specrun catalog infer --dry-run` against the current baseline.
2. If new components are proposed, run `specrun catalog infer` (non-dry-run) to update the catalog.
3. Proceed with composition regeneration (which now reads the updated catalog at step 6).

This makes component detection a **build-time, agent-driven, deterministic** process rather than an operator-initiated one.

#### B4. Screenshots adapter stage-6 feeds forward into catalog inference

Stage-6 `notes.candidate_component` hints currently dead-end at Evidence and surface as `[unknown]` tags. Under this RFC:

- Stage-6 gains an additional output: when the hint is emitted, the adapter also writes a structured sidecar entry to `.specify/.cache/component-candidates/<slug>.yaml` recording the structural skeleton that triggered the hint.
- `specrun catalog infer` reads both the composition baseline AND the candidate cache, using cached skeletons as supplementary evidence for cross-slice structural identity.

This gives the inference verb memory across slices even before the composition baseline accumulates the screens — it can detect shared structures from extraction evidence before those structures reach the composition.

#### B5. Operator review surface (preserved, not removed)

The operator retains:

- The ability to set `status: rejected` on any entry — this permanently suppresses that slug.
- The ability to rename auto-inferred slugs before the next build.
- Visibility into what was inferred via `specrun catalog infer --dry-run` (read-only inspection).
- The `slice-catalog-drift` finding on `specrun slice validate` (unchanged).
- The composition validator's catalog cross-reference (check 5, unchanged).

The change is directional: inference proposes `confirmed` by default; operators demote to `rejected`. This is the inverse of the current model where operators must promote from nothing.

#### B6. Migration from current operator-curated model

For projects with an existing `components.yaml`:

- Existing `confirmed` entries are preserved unchanged.
- Existing `rejected` entries are preserved unchanged — `specrun catalog infer` never overwrites a `rejected` entry.
- New entries from inference are appended with `status: confirmed`.
- No entries are removed by inference (only additions).

For projects without `components.yaml`:

- First `specrun catalog infer` run creates the file if any components are detected.
- If no components are detected (single-screen app, no repeated structures), the file is not created — preserving the current "absent catalog = no factoring" behavior.

## Implementation plan

### Phase 1 — Composition accumulation (critical path)

1. **Brief amendment.** Update `adapters/targets/vectis/briefs/build/composition.md` with the baseline-reading and delta-format instructions (A1, A2).
2. **CLI regression gate.** Implement `composition-baseline-overwrite-blocked` check in `specrun slice merge` with the `--allow-composition-replace` escape hatch (A3).
3. **CLI warnings.** Implement `composition-unexpected-for-core-only` and `composition-empty-for-ui-slice` in `specrun slice build --phase finalize` (A4).
4. **Tests.** Extend `tests/fan_in_fan_out.rs` with a multi-slice composition accumulation scenario asserting the baseline grows monotonically across screen-introducing slices.

### Phase 2 — Component inference

5. **`specrun catalog infer` verb.** Implement the structural-fingerprint algorithm against `composition.yaml` baseline. Land in `src/runtime/commands/catalog/infer.rs` alongside tests under `tests/catalog_infer.rs`.
6. **Brief amendment.** Update `adapters/targets/vectis/briefs/build.md` to invoke `specrun catalog infer` before composition regeneration (B3).
7. **Candidate cache.** Update the screenshots adapter pipeline brief (stage 6) to write structural skeletons to `.specify/.cache/component-candidates/` (B4). Update `specrun catalog infer` to read from the cache.
8. **Documentation.** Rewrite `docs/explanation/components.md` to reflect the agent-inferred model. Update `adapters/sources/screenshots/references/spec-runtime/components.md`.

### Phase 3 — Acceptance

9. **Acceptance scenario.** Add a cross-repo acceptance scenario exercising: 3+ slices each introducing a screen with a shared tab bar → assertion that the catalog is auto-populated after the second slice's build and composition accumulates correctly across all slices.

## Migration

Phase 1 is **schema-compatible**: no changes to `composition.yaml` format, `plan.yaml`, or any existing schema. The brief amendment is advisory (agents read it on next invocation); the CLI gate is additive.

Phase 2 is **additive**: `specrun catalog infer` is a new verb; the candidate cache is a new directory under `.specify/.cache/`; catalog inference adds entries but never removes them.

**Breaking change:** The posture flip from "operator-curated, opt-in" to "agent-inferred, operator-reviewable" is a documentation and workflow expectation change. Existing projects with `status: rejected` entries are unaffected (those entries are respected). Projects relying on the absent-catalog-means-no-factoring guarantee will see component factoring activate once any shared structures exist — this is the intended improvement.

## Alternatives considered

**Require agents to always produce full `screens:` documents including baseline screens.** Rejected. This requires the agent to faithfully reproduce potentially hundreds of baseline screens it didn't author, inviting transcription errors and bloating context windows. The `delta:` format is purpose-built for this.

**Make the merge engine detect and prevent regressions heuristically (e.g., refuse to shrink the screen count).** Rejected as too brittle. A legitimate refactoring slice might remove screens. The right answer is the `delta:` format contract plus an explicit override flag for intentional replacements.

**Keep component catalog operator-curated but add a CLI suggestion command.** Rejected as insufficient. The suggestion-only model is what `notes.candidate_component` already provides today, and it demonstrably does not work — operators don't see the hints, don't act on them, and the feature never activates. The inference must be **active by default**.

**Run component inference at refine time (during synthesis).** Rejected. Synthesis is platform-neutral and does not read `composition.yaml`. Component detection requires spatial structure that only exists after composition regeneration. Build time is the correct moment.

**Use model-assisted (LLM) component detection instead of structural fingerprinting.** Rejected for the deterministic path. The structural-fingerprint algorithm is deterministic, reproducible, and auditable. Model-assisted judgment is appropriate as a supplementary layer (e.g., for slug naming when the heuristic fails) but should not be the primary detection mechanism for a feature that affects code generation.

## Cross-repo touchpoints

| Change | Repo | Files |
| --- | --- | --- |
| Composition brief amendment (A1, A2) | specify | `adapters/targets/vectis/briefs/build/composition.md` |
| Build brief step 0.5 (B3) | specify | `adapters/targets/vectis/briefs/build.md` |
| Screenshots stage-6 candidate cache (B4) | specify | `adapters/sources/screenshots/briefs/extract/pipeline.md` |
| Components explanation rewrite (B6) | specify | `docs/explanation/components.md` |
| Composition regression gate (A3) | specify-cli | `crates/workflow/src/merge/composition.rs`, `src/runtime/commands/slice/merge.rs` |
| Build finalize warnings (A4) | specify-cli | `src/runtime/commands/slice/build.rs` |
| `specrun catalog infer` verb (B2) | specify-cli | `src/runtime/commands/catalog/infer.rs` (new) |
| Candidate cache reader | specify-cli | `crates/workflow/src/design_system.rs` (extend) |
| Fan-in/fan-out composition test | specify-cli | `tests/fan_in_fan_out.rs` |
| Catalog inference tests | specify-cli | `tests/catalog_infer.rs` (new) |

## Open questions

1. Should `specrun catalog infer` also detect shared `states` and `overlays` patterns, or limit to `group` structural identity?
2. What is the right `--min-occurrences` default — 2 (current stage-6 threshold) or 3 (higher confidence)?
3. Should the regression gate (A3) be a hard error or a blocking diagnostic that `specrun slice merge --force` can override?
4. How should the candidate cache handle slug collisions across different structural skeletons (two different structures both heuristically named `card-row`)?
5. Should the composition brief allow the agent to modify baseline screens it didn't author (e.g., adding a `component: tab-bar` directive to a screen from a prior slice) or should that require a dedicated refactoring slice?

## References

- [Composition build brief](../adapters/targets/vectis/briefs/build/composition.md) — the current regeneration algorithm.
- [Merge brief](../adapters/targets/vectis/briefs/merge.md) — Vectis-specific merge gates.
- [Component catalog explanation](../docs/explanation/components.md) — current operator-curated model.
- [Screenshots pipeline stage 6](../adapters/sources/screenshots/briefs/extract/pipeline.md) — current conservative detection.
- [`crates/workflow/src/merge/composition.rs`](https://github.com/augentic/specify-cli/blob/main/crates/workflow/src/merge/composition.rs) — merge engine supporting `screens:` and `delta:` shapes.
- [Layout inferer contract](../adapters/targets/vectis/references/layout-inferer-contract.md) — structural identity rules for component detection.
