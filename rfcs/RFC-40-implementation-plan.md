# RFC-40 implementation plan

This is the step-by-step build plan for [RFC-40](./RFC-40-composition-accumulation-and-component-inference.md). The RFC itself is the source of intent — read it first; this document is the execution contract. Each step is sized for a single agent session, assumes every preceding step has merged, and lists the concrete files, symbols, error codes, and tests it must touch. Steps are ordered so that dependencies always flow downward.

## How to use this plan

- **Two repos.** `specify` (this repo) holds adapter briefs, references, and docs. `specify-cli` (sibling at `../specify-cli`) holds the Rust CLI, schemas, and the `wasi-tools/` carve-out. Each step names its repo. A step never spans both repos unless explicitly noted.
- **Verification per repo.** In `specify-cli`, run `cargo make ci` (CI uses `RUSTFLAGS=-Dwarnings`; do not substitute bare `cargo test`). For changes inside `wasi-tools/`, also run `cargo clippy -p specify-vectis -- -D warnings` and the tool's own `cargo test` from inside `wasi-tools/` — the host `cargo make` does not cover that workspace. In `specify`, run `make lint` (forwards to `specify lint framework`) for brief/doc consistency.
- **Schema discipline (`specify-cli`).** Any new schema file under `schemas/` that is embedded as a constant in `crates/schema/src/constants.rs` must also be added to the byte-for-byte parity table in `crates/schema/tests/schemas.rs::embedded_schemas_match_on_disk_sources` and given a `compile_schema` smoke test. Edits to an existing schema file are picked up automatically by `include_str!`.
- **Never hand-edit lifecycle files.** Route all `.metadata.yaml` / `plan.yaml` / baseline writes through the CLI, per `AGENTS.md`.

## Architectural decision settled up front (RFC §B2 "two viable shapes")

RFC §B2 leaves one decision open: whether the structural-fingerprint + clustering algorithm lands as a new `infer` subcommand inside the `wasi-tools/vectis` tool (RFC-preferred) or via a skeleton crate extracted and shared with a host-side verb. **This plan adopts the RFC-preferred `infer`-subcommand shape.** Rationale: it keeps all skeleton + fingerprint logic inside the existing WASI carve-out (no new cross-workspace shared crate, no risk of a second divergent skeleton algorithm), and it reuses `build_group_skeleton` verbatim. The split of responsibility is:

- **Tool side (`wasi-tools/vectis`)** — deterministic structural analysis only: walk groups, normalize via `build_group_skeleton`, fingerprint, cluster across the composition baseline + candidate cache + parts, derive slugs, resolve slug collisions, apply pin bindings. Emits a JSON *proposal* (clusters + matched/unmatched parts). Touches no catalog file.
- **Host side (`specify catalog infer`)** — orchestration and all file I/O: resolve input paths, dispatch the tool, parse the proposal, load the existing `components.yaml`, apply the B6 reconciliation rules (preserve `confirmed`/`rejected`, never overwrite `rejected`, append new `confirmed`, project matched pinned parts), write the catalog atomically (or print the diff under `--dry-run`), and render the non-blocking `part-unmatched` report.

If the team later prefers the extraction shape instead, only Steps 5–6 and 11–12 change shape (the algorithm moves to a shared crate consumed directly by a host verb); the brief, schema, and test steps are unaffected.

---

## Phase 1 — Composition accumulation (critical path)

Phase 1 closes the data-loss bug. It is schema-compatible and self-contained; ship it before touching Part B/C.

### Step 1 — A3: composition-overwrite merge gate (`specify-cli`)

**Goal.** Make it impossible for a whole-document (`screens:`) slice composition to silently replace a non-empty baseline at merge time, with a single narrow override.

**Files.**

- `crates/workflow/src/merge/composition.rs` — add a **pure** predicate (no I/O, no policy): `pub fn is_whole_document_replacement(text: &str) -> Result<bool, Error>` that parses the delta text and returns `true` iff a `screens` key is present and a `delta` key is absent (mirroring the existing `has_screens && !has_delta` branch logic at lines 30–42). Add a sibling `pub fn baseline_is_non_empty(text: &str) -> bool` (parses, returns `true` iff `screens` is a mapping with ≥1 entry). `merge(baseline, delta_text)` keeps its exact signature and behaviour — do **not** thread the flag into it.
- `crates/workflow/src/merge/slice/read.rs` — add `pub(super) fn composition_overwrite_gate(slice_dir: &Path, class: &ArtifactClass, allow_replace: bool) -> Result<(), Error>`. It reads the slice's top-level `composition.yaml` (via the existing `COMPOSITION_FILENAME` + `read_optional_file`) and the baseline at `class.baseline_dir.join(COMPOSITION_FILENAME)`. When the slice file is whole-document replacement **and** the baseline is non-empty **and** `!allow_replace`, return `Error::Diag { code: "composition-baseline-overwrite-blocked", detail: "Slice composition uses whole-document replacement format but a non-empty baseline exists. Use `delta:` format, or pass `--allow-composition-replace` to authorise full replacement." }`. No slice file, or an absent/empty baseline, or `delta:` format → `Ok(())`.
- `crates/workflow/src/merge/slice.rs` — `commit` gains a new parameter `allow_composition_replace: bool`. Invoke `read::composition_overwrite_gate(...)` against `first_three_way(classes)` **after** the `LifecycleStatus::Built` check (lines 207–212) and **before** `plan_three_way` (line 214). Export `composition_overwrite_gate` through the `read::{...}` use-list at the top of `slice.rs`. `preview` is unchanged (read-only dry run never aborts).
- `src/runtime/commands/slice/cli.rs` — add `#[arg(long)] allow_composition_replace: bool` to the `SliceMergeAction::Run { name }` variant (lines 164–168).
- `src/runtime/commands/slice/merge.rs` — thread the flag: `run(ctx, name, allow_composition_replace)` → `commit_run(ctx, name, allow_composition_replace)` → `slice::commit(&slice_dir, &classes, &archive_dir, now, allow_composition_replace)`. Update the dispatch site that calls `merge::run` (in `slice.rs` / the slice command router) to pass the parsed flag.

**Implementation notes.** Keep the gate a single-responsibility precondition (RFC §A3 "Placement" / R2). The flag threads exactly two hops (CLI handler → `slice::commit`) and must never reach `plan_three_way`, `merge_composition_delta`, or `composition::merge`. Find every existing caller of `slice::commit` (tests included) and add the new `false` argument.

**Tests.** `crates/workflow/tests/merge_slice.rs`: (a) whole-document slice + non-empty baseline + `allow=false` → `composition-baseline-overwrite-blocked`; (b) same + `allow=true` → succeeds and replaces; (c) `delta:` slice + non-empty baseline → no gate, accumulates; (d) whole-document slice + absent/empty baseline → no gate (establishes initial baseline). Add the pure-predicate unit tests in `crates/workflow/src/merge/composition.rs`'s `mod tests`.

**Done when.** `cargo make ci` is green and the four gate cases assert as above.

### Step 2 — A4: `ui_surface` report field, finalize coherence checks, non-blocking warning channel (`specify-cli`)

**Goal.** Add a per-slice "has UI surface" signal authored by the build brief, and two deterministic self-consistency warnings at `--phase finalize` — without changing the verb's exit code.

**Files.**

- `schemas/target/build-report.schema.json` — add an optional object property `ui-surface` with a single required `screens` integer (`minimum: 0`), `additionalProperties: false`. Additive; existing reports without it stay valid.
- `crates/workflow/src/slice/build/wire.rs` — add `pub struct UiSurface { pub screens: u32 }` (`#[serde(rename_all = "kebab-case", deny_unknown_fields)]`) and a `#[serde(default, skip_serializing_if = "Option::is_none")] pub ui_surface: Option<UiSurface>` field on `BuildReport`. Add two pure functions returning warning diagnostics (not aborts): `pub fn evaluate_ui_surface_coherence(report: &BuildReport, composition_path: &Path) -> Vec<Diagnostic>`. Logic: inspect the produced `composition.yaml` at `composition_path` (absent/`screens: {}`/all-empty `delta:` ⇒ "empty"; a `screens:` map with ≥1 entry or a non-empty `delta:` ⇒ "non-empty"). When `ui_surface.screens == 0` and composition is non-empty → push a `composition-unexpected-for-non-ui-slice` warning. When `ui_surface.screens > 0` and composition is empty/absent → push a `composition-empty-for-ui-slice` warning. When `ui_surface` is `None`, return no warnings (back-compat). Build each `Diagnostic` with `source = deterministic`, `kind = violation`, a non-blocking severity (`suggestion`), and a stable fingerprint — reuse the `specify-diagnostics` constructors already imported here.
- `src/runtime/commands/slice/build.rs` — add `warnings: Vec<Diagnostic>` to `BuildResult` (line 82). In `finalize_report` (after the `Built` transition, line 222), call `evaluate_ui_surface_coherence(&report, &slice_dir.join("composition.yaml"))` and store the result; warnings never gate the transition or the exit code. Render them in `write_result_text` (line 345) and let `serde` carry them in the JSON body. Import `Diagnostic` from `specify_diagnostics`.

**Implementation notes.** A4 is a self-consistency check (RFC §A4 / R1): never re-derive screen identification in the host, and never key anything off `## Platforms`. The warning channel is new finalize surface (RFC §A4 "Surfacing the warnings") — it is additive and must be wired explicitly. Check `crates/schema/tests/schemas.rs` already covers `BUILD_REPORT_JSON_SCHEMA` parity (it does); add a `build_report_accepts_ui_surface` example test there.

**Tests.** `tests/slice/build.rs`: finalize with `ui_surface.screens: 0` + a non-empty staged `composition.yaml` → exit 0, body carries `composition-unexpected-for-non-ui-slice`; `ui_surface.screens: 2` + empty/absent composition → exit 0, body carries `composition-empty-for-ui-slice`; matched cases → no warnings; absent `ui_surface` → no warnings.

**Done when.** `cargo make ci` green; warnings appear in both text and JSON finalize output and never alter the exit code.

### Step 3 — A1/A2/A4 brief amendments: composition accumulation + delta format + skip re-key (`specify`)

**Goal.** Teach the composition build brief to read the baseline and emit accumulating deltas, and re-key the non-UI skip off `spec.md` (not `## Platforms`).

**Files.**

- `adapters/targets/vectis/briefs/build/composition.md`:
  - Add a new **priority-0 input** above the current list (lines 7–13): `${PROJECT_DIR}/.specify/specs/composition.yaml` — the merged baseline composition; when present, regeneration accumulates (retain all baseline screens unchanged; add/modify/remove only screens this slice's `spec.md` positively references).
  - Amend **Step 1** (line 17) to distinguish new vs modified vs removed screens, and state that baseline screens not referenced by this slice are carried forward unchanged. Add the **explicit-removal** rule: emit a slug under `delta.removed` only on a positive retirement signal in this slice's own `spec.md`/`design.md`; non-mention is never a removal.
  - Amend **Step 9** (line 30): when the baseline contains screens this slice does not reference, do not surface them as gaps.
  - Add a **delta-format section** (A2): when the baseline exists and is non-empty (`screens` map ≥1 entry), the agent MUST write the `delta: { added, modified, removed }` envelope (each `modified` entry is a whole-screen faithful superset); when no baseline exists or the baseline is empty (`screens: {}`), write the `screens:` format to establish the initial baseline. Include the YAML example from RFC §A2.
  - **Re-key the skip rule** (line 43): replace the `proposal.md`/`## Platforms`-keyed detection with: skip composition regeneration when this slice's `spec.md` describes no screen-bearing requirements, regardless of which platforms `## Platforms` lists.
  - **Re-label input #4** (line 12): "operator-curated, read-only" → "agent-inferred, read-only" (the catalog is now agent-written per B1).
- `adapters/targets/vectis/briefs/build.md` — in the **Build report** section (around lines 116–131), document the new optional `ui-surface: { screens: <N> }` field: the brief sets it from its own `spec.md` screen-identification judgement (the count of screen-bearing requirements this slice introduces or modifies; `0` means no UI surface), **never** from `## Platforms`.

**Implementation notes.** Prose only; no code. Keep paragraphs single-line per the repo markdown rule. Do not touch the lines flagged "not swept" by RFC §R5 (those land in Step 10).

**Tests.** `make lint` in `specify`.

**Done when.** `make lint` passes and the brief instructs baseline-read, delta emission, the spec-keyed skip, and `ui-surface` authoring.

### Step 4 — Phase 1 end-to-end accumulation test (`specify-cli`)

**Goal.** Prove the baseline grows monotonically across screen-introducing slices and that the A3 gate fires in a realistic multi-slice run.

**Files.**

- `tests/plan/end_to_end.rs` — add a multi-slice scenario (this is the relocated fan-in/fan-out suite; `tests/fan_in_fan_out.rs` no longer exists): three slices each contributing a `delta.added` screen, merged in sequence, asserting the baseline `screens` map grows 1→2→3 and no prior screen is lost. Add one slice that emits a whole-document `screens:` composition against a non-empty baseline and assert the merge aborts with `composition-baseline-overwrite-blocked` unless `--allow-composition-replace` is passed.

**Implementation notes.** Reuse the `tests/common/mod.rs` `Project` harness (`Project::init().with_schemas()`, `stage_slice`, `seed_plan`). The kernel-level accumulation assertions already live in `crates/workflow/tests/merge_slice.rs` from Step 1; this step is the integration layer.

**Done when.** `cargo make ci` green with the monotonic-growth and gate assertions.

---

## Phase 2 — Component inference

Phase 2 is additive. Steps 5–7 build the deterministic engine and the host verb; Steps 8–10 wire the briefs and docs; Steps 11–12 add Part C (operator parts).

### Step 5 — Structural fingerprint + `vectis infer` subcommand, baseline-only (`specify-cli`, `wasi-tools/vectis`)

**Goal.** Land the deterministic detection core: a canonical fingerprint over the existing `Skeleton`, and an `infer` subcommand that clusters identical groups across the composition baseline and proposes catalog candidates as JSON.

**Files (all under `wasi-tools/vectis/`).**

- `src/validate/engine/composition.rs` — expose the existing normalizer for reuse: make `build_group_skeleton`, `build_node_skeleton`, and the `Skeleton` enum `pub(crate)` (they are private today). Add a canonical fingerprint: `pub(crate) fn fingerprint(skeleton: &Skeleton) -> String` = SHA-256 over a **canonical, deterministic serialization** of the `Skeleton` tree (define the serialization explicitly — e.g. a recursive `Group(when_keys_sorted, [children...])` / `Item(kind)` byte encoding; `when_keys` are already sorted+deduped). Reuse the crate's existing hashing dependency if present, else add `sha2` to the tool's `Cargo.toml` (the carve-out may carry its own deps). Do **not** make identity fuzzy — exactness is mandated by `check_structural_identity` (RFC §B2 / R3).
- `src/infer.rs` (new) — `pub struct InferArgs` (clap) with `--composition <path>` (the baseline), `--candidate-cache <dir>` (optional, used from Step 7), `--parts <path>` (optional, used from Step 11), and `--min-occurrences <N>` (default 2). `pub fn run(args: &InferArgs) -> Value` and `pub fn render_json(...)`. Algorithm for this step (baseline only): parse the composition baseline; walk every screen's `group` subtrees (reuse the `walk_for_components`-style descent through `screens` and `delta.added`/`delta.modified`, and descend through `states`/`overlays` but treat only `group` as the detection unit per RFC §B2 "Detection scope"); build the skeleton + fingerprint for each group; cluster by fingerprint counting **distinct screens** (a group repeated within one screen counts once); keep clusters seen across ≥`min_occurrences` screens; derive a slug per cluster via the deterministic heuristic (RFC §B2: `footer` icon-button→`Navigate(*)` ⇒ `tab-bar`/`bottom-nav`; `body` list item template ⇒ `<content-type>-row`; repeated `card` ⇒ `<purpose>-card`; fallback `component-<fp-prefix>`); resolve slug collisions deterministically (first-writer-wins for the bare slug, later distinct fingerprints suffixed `slug-<fp-prefix>`, never ordinal; lexicographic-fingerprint tiebreak). Emit a JSON proposal: `{ version: 1, clusters: [{ slug, fingerprint, occurrences, screens: [<slug>...], pinned: false, origin: "inferred" }], unmatched_parts: [] }`.
- `src/lib.rs` — register `Infer(infer::InferArgs)` in `VectisCommand` (lines 71–84) and dispatch it in `run` (lines 89–96): `VectisCommand::Infer(v) => infer::render_json(infer::run(v))`. Add `pub mod infer;` (line 27–32 area). Update the crate doc comment listing subcommands (lines 14–22).

**Implementation notes.** This step keeps everything inside the carve-out and reuses `build_group_skeleton` verbatim — do not write a second skeleton algorithm. In-process clustering keys on the `Skeleton`/fingerprint directly; the fingerprint **string** is what crosses the process boundary into the JSON proposal and the later `card-row-<fp-prefix>` suffix.

**Tests.** `wasi-tools/vectis/tests/` (add `engine/infer.rs` and register in `engine.rs`): a baseline with the same `footer` group on 3 screens clusters to one `tab-bar` candidate; two distinct skeletons that derive the same heuristic slug yield two clusters, the second suffixed `-<fp-prefix>`; a group on a single screen is below the default threshold and is not proposed. Run `cargo clippy -p specify-vectis -- -D warnings` and `cargo test` from inside `wasi-tools/`.

**Done when.** The tool's own tests pass and `vectis infer --composition <baseline>` emits the proposal JSON.

### Step 6 — `specify catalog infer` host verb + B6 reconciliation (`specify-cli`)

**Goal.** Add the host verb that drives the tool, reconciles against the existing catalog, and writes `components.yaml` (or prints the diff). No parts/cache yet (added in Steps 7/11).

**Files.**

- `src/runtime/cli.rs` — add a top-level `Catalog { #[command(subcommand)] action: CatalogAction }` variant to `Commands` (near the other group variants, ~line 156). Import `CatalogAction`.
- `src/runtime/commands/catalog/cli.rs` (new) — `pub enum CatalogAction { Infer { #[arg(long = "dry-run")] dry_run: bool, #[arg(long)] min_occurrences: Option<u32> } }`.
- `src/runtime/commands/catalog.rs` (new) + `src/runtime/commands/catalog/infer.rs` (new) — register `pub mod catalog;` in `src/runtime/commands.rs` (lines 1–16) and dispatch `Commands::Catalog { action } => scoped(format, |ctx| catalog::run(ctx, action))` in `commands.rs::run` (the verb needs a project, so use `scoped`). `infer.rs`: resolve the composition baseline path (`ctx.project_dir/.specify/specs/composition.yaml`); when it is absent, the verb is a no-op (RFC §B6 "absent catalog = no factoring"). Dispatch the tool via the existing tool runner (`tool::run(ctx, "vectis", vec!["infer", "--composition", <path>, "--min-occurrences", <n>])`) and capture/parse its JSON proposal (the runner returns the guest exit code; capture stdout — see how `tests/tool/run.rs` exercises the runner, and reuse the same capture path). Load the existing catalog via `ComponentsCatalog::load(&ctx.project_dir)`. Apply **B6 reconciliation**: keep existing `confirmed` and `rejected` entries unchanged; for each proposed cluster slug, skip if already `confirmed` or `rejected`; otherwise add `status: confirmed` with an auto description. Never remove entries. Under `--dry-run`, print the diff and write nothing; otherwise write `components.yaml` atomically.
- `crates/workflow/src/design_system.rs` — add a catalog **writer**: `ComponentsCatalog::save(&self, project_dir: &Path) -> Result<()>` (atomic write via `specify_model::atomic`) and an `upsert_inferred(&mut self, slug: &str, description: Option<String>)` helper that respects B6 (no-op when slug exists as `confirmed`/`rejected`). Update the module doc comment (lines 1–6) from "operator-curated … opt-in" to the agent-inferred / operator-reviewable posture (this is the cli-repo half of the R5 sweep; the specify-repo prose lands in Step 10).

**Implementation notes.** Tool permissions: the `infer` subcommand needs read access to `.specify/specs/composition.yaml` (and, from Step 7/11, the candidate-cache dir and `parts.yaml`). Confirm the vectis adapter's `tools.yaml` read-permission set covers `.specify/` and extend it if not. `specify catalog infer` is deterministic and read-mostly: emit no journal events.

**Tests.** `tests/catalog_infer.rs` (new, top-level integration binary — cargo auto-discovers it): seed a baseline with a repeated group; `specify catalog infer --dry-run` prints the proposed `tab-bar` without writing; `specify catalog infer` writes `components.yaml` with `tab-bar: { status: confirmed }`; a pre-existing `rejected` entry is preserved and not re-added; a pre-existing `confirmed` entry is untouched; absent baseline → no file created.

**Done when.** `cargo make ci` green; the verb writes/reconciles the catalog from a baseline.

### Step 7 — B4: screenshots candidate cache + infer reads it (`specify` brief + `specify-cli`)

**Goal.** Give inference cross-slice memory before the baseline accumulates, by caching normalized group skeletons from screenshots stage-6 and feeding them into clustering.

**Files.**

- `adapters/sources/screenshots/briefs/extract/pipeline.md` — extend **stage 6** (lines 86–102): when a `notes.candidate_component: <slug>` hint is emitted, also write a sidecar under `.specify/.cache/component-candidates/<slice>/<screen>/<group-path>.yaml` containing the **normalized `group` skeleton** (a composition-`group`-shaped fragment), with the derived slug stored as an inner label alongside the skeleton. Key strictly by **provenance** (`<slice>/<screen>/<group-path>`), never by slug or fingerprint (RFC §B4 / R4 — stage-6 is an agent/vision brief and cannot compute the tool's canonical hash). Because stage-6 emits Evidence `container: group` claims, perform the Evidence→composition shape translation at write time so the cached body is already in the shape `build_group_skeleton` consumes.
- `wasi-tools/vectis/src/infer.rs` — implement `--candidate-cache <dir>`: when present, recursively read every `*.yaml` entry, extract its `group` fragment, normalize via `build_group_skeleton`, fingerprint **at read time**, and fold those skeletons into the same clustering pass as baseline groups (so a cached skeleton and a baseline group with one fingerprint are one candidate). No agent-written fingerprint is ever trusted (RFC §B4 R4: one normalizer, both sides).
- `src/runtime/commands/catalog/infer.rs` — pass `--candidate-cache <ctx.project_dir/.specify/.cache/component-candidates>` to the tool when the directory exists.

**Implementation notes.** This is the cache↔baseline application of "fingerprint at read time, one normalizer." The cache is a new directory under `.specify/.cache/` and needs no schema.

**Tests.** `wasi-tools/vectis/tests/engine/infer.rs`: a cached skeleton plus one matching baseline group on a different screen cluster to a single candidate at threshold 2. `tests/catalog_infer.rs`: seed one cache entry + one baseline screen with the matching group → the verb proposes the shared component. `make lint` in `specify` for the brief.

**Done when.** Cached skeletons participate in clustering and the brief documents provenance-keyed cache writes.

### Step 8 — B3: build brief invokes inference before composition regeneration (`specify`)

**Goal.** Make component detection a build-time, agent-driven, deterministic step ahead of composition regeneration.

**Files.**

- `adapters/targets/vectis/briefs/build.md` — add a **step 0.5** to the Phase order (between step 1 "load composition.md" framing and regeneration; see lines 57–67): (1) run `specify catalog infer --dry-run` against the current baseline; (2) if new components are proposed, run `specify catalog infer` (non-dry-run) to update the catalog; (3) proceed with composition regeneration, which reads the updated catalog at composition.md step 6.

**Implementation notes.** Prose only. Keep the existing phase numbering coherent (renumber or insert "Step 0.5" explicitly as the RFC names it).

**Tests.** `make lint`.

**Done when.** The build brief runs inference before regeneration.

### Step 9 — B7: retroactive cross-slice factoring briefs (`specify`)

**Goal.** When a build promotes a component whose other instances live in prior-slice screens, fold the component back into those screens (directive-only) and refactor their generated code.

**Files.**

- `adapters/targets/vectis/briefs/build/composition.md` — extend the regeneration steps so that, for each **baseline screen outside the current slice's units** carrying a group structurally identical (same fingerprint) to a newly promoted component, the build emits a `delta.modified.<screen>` entry reproducing that prior screen as a faithful superset with a `component: <slug>` directive attached to the matching group. State the **directive-only** constraint explicitly: the sole permitted change to a not-authored baseline screen is attaching/detaching a `component:` directive on an already-matching group; layout restructuring stays on the dedicated-refactoring-slice path (RFC §A2a case 2 / §A3).
- `adapters/targets/vectis/briefs/build/core/write.md` — extend **step 6** (line 23): when the catalog gains a confirmed component referenced by baseline screens outside the current slice's units, generate the shared component module **and** refactor the affected prior screens' generated `view()` code to consume it (the writers already run in `update` mode against the live shell tree). Note behaviour-preservation follows from identical skeletons; the verify-repair loops catch regressions.
- `adapters/targets/vectis/briefs/build/ios/write.md` and `adapters/targets/vectis/briefs/build/android/write.md` — mirror the same addition: generate `Components/<Slug>View.swift` / `components/<Slug>Component.kt` and refactor affected prior screens' views to consume the shared component.

**Implementation notes.** Prose only. Lean on the RFC's "no cross-branch hazard" argument (sequential execution under the exclusive plan lock) — do not invent a new synchronization mechanism. Idempotence: re-runs see the component already `confirmed` and the directive already attached.

**Tests.** `make lint`.

**Done when.** All four briefs describe directive-only retroactive factoring and the code-side refactor.

### Step 10 — R5: doc-inversion sweep (`specify`)

**Goal.** Flip the operator-curated framing to agent-inferred / operator-reviewable everywhere the prose still asserts the old posture.

**Files.**

- `plugins/spec/references/components.md` (canonical) — **delete** the two "What the catalog does not do" bullets ("No auto-population — operator-curated only"; "No retroactive baseline rewrite without a refactor slice"). **Reframe** the third bullet ("No CLI verbs for catalog edits …") to note `specify catalog infer` now writes the catalog while operators still hand-edit to reject/rename (keep the fourth bullet "No sharing across projects"). Invert the lede ("Vectis-only, opt-in" → agent-inferred / operator-reviewable, auto-created by `specify catalog infer` when shared structures exist), the "cross-slice component drift" section (→ agent infers from the accumulated baseline + candidate cache), and the "Operator workflow" section (inference-first: build runs `specify catalog infer`; operator reviews/rejects per B5).
- `adapters/targets/vectis/briefs/build.md` — re-key **line 7** (split `tokens.yaml`/`assets.yaml` = operator-curated from `components.yaml` = agent-inferred/operator-reviewable) and **line 22** ("opt-in component catalog" → "agent-inferred component catalog"). Do **not** touch lines 37/142/144 (path/merge behaviour, correct under B1).
- `adapters/targets/vectis/briefs/build/composition.md` — confirm input #4 framing flipped to "agent-inferred, read-only" (done in Step 3; verify here, do not re-edit step 6's factoring prose).

**Implementation notes.** Do **not** edit `docs/explanation/components.md` (mdBook stub) or `adapters/sources/screenshots/references/spec-runtime/components.md` (symlink) — both resolve to the canonical file. `merge.md` is out of scope (no operator-curated catalog language). The two cli-repo code comments (`design_system.rs` module doc, vectis `check_catalog_cross_references`) are handled in Steps 6 and 11, not here.

**Tests.** `make lint`.

**Done when.** No "operator-curated" / "opt-in" / "no auto-population" catalog framing remains in the swept files; `make lint` passes.

### Step 11 — C1/C2/C3: operator parts (`parts.yaml`) schema, loader, infer seeding, unused-parts report (`specify-cli`)

**Goal.** Add the authoritative operator-defined parts input that seeds inference with naming + promotion authority, projects matched parts into the catalog, and reports unmatched parts non-blockingly.

**Files.**

- `schemas/design-system/parts.schema.json` (new) — `version: const 1`; `parts` is a map of kebab-case slugs (`^[a-z][a-z0-9]*(-[a-z0-9]+)*$`) to `{ group: <composition group fragment>, description?: string }`, `group` required. The `group` must validate against the composition `group` shape so it round-trips through `build_group_skeleton` (reference/inline the relevant group sub-schema as the vectis composition schema defines it).
- `crates/schema/src/constants.rs` — add `pub const PARTS_JSON_SCHEMA: &str = include_str!("../../../schemas/design-system/parts.schema.json");`. Add it to the parity table and a `compile_schema` smoke test in `crates/schema/tests/schemas.rs`.
- `crates/workflow/src/design_system.rs` — add a `Parts` loader mirroring `ComponentsCatalog::load`: `Parts::load(project_dir) -> Result<Option<Parts>>` reading `.specify/design-system/parts.yaml`, schema-validated on read (`validate_parts_yaml`), **no coherence gate** beyond schema (RFC §C1). Expose each part's slug + `group` fragment for the tool.
- `wasi-tools/vectis/src/infer.rs` — implement `--parts <path>` (RFC §C2): **Step 0** register a pinned binding `{ fingerprint → slug }` per part (normalize the part's `group` via `build_group_skeleton`, fingerprint at read time). **Step 4** threshold bypass: a pinned fingerprint is confirmed as soon as it matches ≥1 baseline/cache group (ignore `--min-occurrences`); a pinned fingerprint matching **zero** groups is not projected — surface it under `unmatched_parts`. **Step 5** slug precedence: when a cluster's fingerprint matches a pin, use the operator slug and skip heuristic derivation + first-writer suffixing for that fingerprint; a heuristic cluster with a different fingerprint that derives the same name is suffixed `slug-<fp-prefix>`. Extend the JSON proposal: clusters gain `pinned: true, origin: "operator"` for pinned matches, and `unmatched_parts: [<slug>...]` lists pins that matched nothing.
- `src/runtime/commands/catalog/infer.rs` — pass `--parts <ctx.project_dir/.specify/design-system/parts.yaml>` when present. Project **matched** pinned parts into `components.yaml` as `status: confirmed` re-derived each run (RFC §C3 — durable source is `parts.yaml`, catalog is derived, so re-runs are no-ops). Emit the **`part-unmatched`** non-blocking report (informational, in normal output and the `--dry-run` diff) from the proposal's `unmatched_parts`; it never aborts and is never a merge precondition (RFC §C5).
- `wasi-tools/vectis/src/validate/engine/composition.rs` — flip the "operator-curated" wording in the `check_catalog_cross_references` doc comment (cli-repo half of R5).

**Implementation notes.** Reconciliation precedence (RFC §C6) is resolved silently, no findings: operator slug wins naming; pinned promotion bypasses the threshold; two parts normalizing to one skeleton bind to the lexicographically-first slug; a part slug colliding with a `rejected` catalog entry stays suppressed (rejected wins). No `source` discriminant is added to `components.yaml` (RFC Open question 7) — origin is recoverable from `parts.yaml`.

**Tests.** `tests/catalog_infer.rs`: a pinned part matching one baseline group is confirmed below threshold and uses the operator slug; a pinned part matching zero groups is reported `part-unmatched` and not written to the catalog; a heuristic cluster sharing the operator's name but a different fingerprint is suffixed; a part slug equal to a `rejected` entry stays unfactored. Tool-level seeding tests in `wasi-tools/vectis/tests/engine/infer.rs`.

**Done when.** `cargo make ci` green; parts seed inference, project when matched, and report when unmatched.

### Step 12 — C1–C6: operator parts doc + brief consumption (`specify`)

**Goal.** Document `parts.yaml` and note its consumption in the build briefs.

**Files.**

- `plugins/spec/references/components.md` — document `parts.yaml`: the inputs-vs-resolved split (`parts.yaml` is the hand-authored input beside `tokens.yaml`/`assets.yaml`; `components.yaml` is the agent-resolved catalog), the two authorities (naming + promotion), and that parts are best-effort matching hints like tokens/assets (matched ones factored, unmatched ones reported via `part-unmatched`, execution always proceeds).
- `adapters/targets/vectis/briefs/build.md` — note in step 0.5 (added in Step 8) that `specify catalog infer` reads `parts.yaml` as a third authoritative input.
- `adapters/targets/vectis/briefs/build/composition.md` — note that pinned parts participate in factoring per RFC §C4/B7 (they carry live fingerprints, so retroactive factoring reaches prior-slice screens unchanged).

**Tests.** `make lint`.

**Done when.** `parts.yaml` is documented and its consumption noted in both briefs; `make lint` passes.

---

## Phase 3 — Acceptance

### Step 13 — Cross-repo acceptance scenario (`specify-cli`, optionally `specify`)

**Goal.** Exercise the full loop end-to-end and lock the headline behaviours.

**Files.**

- `tests/plan/end_to_end.rs` (or a dedicated acceptance fixture under `tests/`) — a scenario with 3+ slices, each introducing a screen that shares a tab bar. Assert: (1) the catalog auto-populates after the **second** slice's build (default `--min-occurrences 2`); (2) the prior screens are retroactively given the `component:` directive (directive-only `delta.modified`) and their generated views refactored to consume the shared component (B7); (3) composition accumulates correctly across all slices (monotonic, no loss). Include a documentation-only slice asserting the A4 `composition-unexpected-for-non-ui-slice` warning surfaces at finalize and the A3 gate protects the baseline at merge.

**Implementation notes.** This is the integration capstone; keep code generation assertions focused on the composition/catalog artifacts and the presence of the shared component module path, not on full compilable shells (those are covered by the manual `acceptance/` packs).

**Done when.** `cargo make ci` green; the scenario asserts auto-population, retroactive factoring, and monotonic accumulation.

---

## Sequencing summary

| Step | Repo | Concern | Key artifacts |
| --- | --- | --- | --- |
| 1 | specify-cli | A3 merge gate | `merge/composition.rs`, `merge/slice.rs`, `merge/slice/read.rs`, `slice/cli.rs`, `slice/merge.rs` |
| 2 | specify-cli | A4 `ui_surface` + finalize warnings | `build-report.schema.json`, `slice/build/wire.rs`, `commands/slice/build.rs` |
| 3 | specify | A1/A2/A4 composition brief | `briefs/build/composition.md`, `briefs/build.md` |
| 4 | specify-cli | Phase 1 e2e test | `tests/plan/end_to_end.rs` |
| 5 | specify-cli (wasi) | Fingerprint + `infer` (baseline) | `wasi-tools/vectis/src/infer.rs`, `…/engine/composition.rs`, `…/lib.rs` |
| 6 | specify-cli | `specify catalog infer` + B6 | `runtime/cli.rs`, `commands/catalog/*`, `design_system.rs` |
| 7 | specify + cli | B4 candidate cache | `screenshots/.../pipeline.md`, `infer.rs`, `commands/catalog/infer.rs` |
| 8 | specify | B3 build brief step 0.5 | `briefs/build.md` |
| 9 | specify | B7 retroactive factoring briefs | `briefs/build/composition.md`, `build/{core,ios,android}/write.md` |
| 10 | specify | R5 doc-inversion sweep | `plugins/spec/references/components.md`, `briefs/build.md`, `briefs/build/composition.md` |
| 11 | specify-cli | C1/C2/C3 parts schema/loader/seeding/report | `parts.schema.json`, `constants.rs`, `design_system.rs`, `infer.rs`, `commands/catalog/infer.rs` |
| 12 | specify | C1–C6 parts doc + briefs | `plugins/spec/references/components.md`, `briefs/build.md`, `briefs/build/composition.md` |
| 13 | specify-cli | Acceptance capstone | `tests/plan/end_to_end.rs` |

## New error / diagnostic codes introduced

- `composition-baseline-overwrite-blocked` — A3 hard merge abort (Step 1).
- `composition-unexpected-for-non-ui-slice` — A4 non-blocking finalize warning (Step 2).
- `composition-empty-for-ui-slice` — A4 non-blocking finalize warning (Step 2).
- `part-unmatched` — C5 non-blocking informational report from `specify catalog infer` (Step 11).

All four reuse existing producers: the first is an `Error::Diag` consistent with the other `composition-*` aborts; the middle two are `specify-diagnostics` `Diagnostic` warnings on the new `BuildResult.warnings` channel; the last is an informational report line, not a `Diagnostic`.

