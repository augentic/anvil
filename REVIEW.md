# Code & Skill Review — single pass, quality-biased

Scope: `specify` + `specify-cli`, including shipped Skills. Pre-1.0, no back-compat.

## Summary

Prior pass (post-mortem at bottom) already landed ~−1500 LOC in `specify-cli` (journal.yaml, ChangeBrief, validation unification, resolve collapse, etc.). **This pass finds net-new debt**, not repeats.

Top three by LOC removed: **(1)** delete the RFC-20 `survey` module (~1908 LOC, zero CLI wiring); **(2)** retire `adapters/default` + pipeline integrity checks (~550 LOC in `specify`); **(3)** delete the orphaned `SliceTransitionRefined` journal variant + its fixture tests (~95 LOC). If every structural finding lands, **ΔLOC ≈ −2800** (~4% of Rust + skill/reference prose). Non-LOC axes moved: **−6 types**, **−3 enums**, **−1 clap subtree**, **−6 skill/CLI drift surfaces**, **−1 call-site mismatch** (skills vs CLI lifecycle). Most likely to break in remediation: **F4** (lifecycle rename touches merge gates, goldens, and every integration test touching `.metadata.yaml`).

### Reconnaissance (current)

| Command | Result |
|---|---|
| `tokei` (`specify-cli`) | 270 Rust files, **43,141** code lines; **65,832** total |
| `tokei` (`specify`) | **14,658** code lines (mostly TS checks + markdown) |
| `cargo tree --duplicates` | Transitive dupes only (`base64` 0.21/0.22, `bitflags`, `rustix`, `reqwest`) — no actionable host-edge trim without new deps |
| `rg -c '^#\[test\]' crates/ src/ tests/` | **597** unit/integration tests |
| `rg --files -g '**/mod.rs'` | **3** (`tests/common`, `crates/domain/tests/common`, `wasi-tools/vectis/tests/engine_support`) |
| `wc -l docs/standards/*.md AGENTS.md` | **638** lines |
| Files >500 lines (`crates/`, `src/`) | `workspace.rs` 1041, `finalize.rs` 947, `registry.rs` 922, `plan/core/model.rs` 809, `provenance.rs` 631, `survey_ingest.rs` 569, `tool/validate.rs` 520, `tool/package.rs` 504 |

---

## Structural findings

### F1 — Delete RFC-20 survey module

**Evidence.** `rg 'survey|SurfacesDocument|validate_surfaces' specify-cli/src/` → **0 hits**. Module surface: `wc -l crates/domain/src/survey*.rs crates/domain/src/survey/**/*.rs crates/domain/tests/survey*.rs schemas/survey*.json schemas/surfaces.schema.json` → **1908**. Header still says `specify change survey` (`crates/domain/src/survey/ingest.rs:1`). RFC-25 §Hard cut retires `/change:survey`; enumeration lives in source `enumerate`.

**Action.**
1. Delete `crates/domain/src/survey/` (+ `survey.rs`), `crates/domain/tests/survey.rs`, `crates/domain/tests/survey_ingest.rs`, `crates/domain/tests/sources.rs`, `crates/domain/tests/fixtures/survey{,_ingest}/`, `schemas/survey-metadata.schema.json`, `schemas/surfaces.schema.json`.
2. Remove `pub mod survey;` from `crates/domain/src/lib.rs`.
3. Drop any `Cargo.toml` `[[test]]` entries pointing at deleted files.

**Quality delta.** −1908 LOC, −6 types (`SurfacesDocument`, `Surface`, `SurfaceKind`, `MetadataDocument`, `IngestInputs`, `IngestOutcome`), −1 module edge, −2 schema files.

**Net LOC.** 1908 → 0.

**Done when.** `rg -c 'survey|SurfacesDocument' specify-cli/crates/ specify-cli/tests/ specify-cli/schemas/` → 0; `cargo make check` green.

**Rule?** No.

**Counter-argument.** "Keep DTOs for a future CLI verb." Loses: RFC-25 replaced survey with source adapters; re-adding means new schemas, not reviving RFC-20 shapes. *jj* deleted `debugsnapshot` when observability moved — same cut.

**Depends on.** none.

---

### F2 — Retire `adapters/default` pipeline shell

**Evidence.** `PROFILES_DIR = join(REPO_ROOT, "adapters")` (`scripts/checks/_shared.ts:26`). Only legacy manifest with `pipeline:` is `adapters/default/adapter.yaml` (no `axis:`). Codex resolver already probes `targets/default/` (`tests/codex.rs:105`). `wc -l adapters/default/**/*` → **447**; `checkAdapterIntegrity` pipeline graph (`scripts/checks/adapter.ts:82–187`) → **106 LOC** that only runs when `manifest.pipeline` exists.

**Action.**
1. Create `targets/default/adapter.yaml` (RFC-25 target manifest, `operations: [shape, build, merge]` or codex-only stub) + move `adapters/default/codex/**` there.
2. Delete `adapters/default/briefs/{define,build,merge}.md`, `adapters/default/adapter.yaml`, `adapters/default/README.md`.
3. Delete `checkAdapterIntegrity()` and its export from `scripts/checks.ts`; retarget `validateAdapterYaml()` to walk `sources/` + `targets/` instead of `adapters/`.
4. Update `scripts/checks/codex.ts` discovery roots from `adapters/<cap>/codex` to `targets/<cap>/codex` + `sources/<cap>/codex`.

**Quality delta.** −553 LOC, −1 legacy manifest shape, −1 check predicate, −1 module edge (`PROFILES_DIR` → axis dirs).

**Net LOC.** ~553 → ~40 (new `targets/default/adapter.yaml` only).

**Done when.** `rg -c 'pipeline:' adapters/default/` → 0; `rg -c 'checkAdapterIntegrity' scripts/` → 0; `make checks` green.

**Rule?** No.

**Counter-argument.** "Init still copies sibling `default`." Loses: init cache path is already `.specify/.cache/targets/default/` per codex tests; the pipeline briefs are never loaded post-RFC-25.

**Depends on.** none.

---

### F3 — Gut stale phase-outcome contract

**Evidence.** `plugins/spec/references/phase-outcome-contract.md` = **127 lines**; references deleted surfaces: `journal.yaml` (L6–7, L85–99), `specify slice journal append` (L90), `plan transition … failed|blocked` (L46–47, L117), 1.x phase table with `define` column (L121). `/spec:execute` body (`plugins/spec/skills/execute/SKILL.md`, 72 lines) uses stop-hints + lifecycle only — no `outcome set`, no self-heal. `drop/SKILL.md:17` still cites removed execute steps "11b, 12b".

**Action.**
1. Replace `phase-outcome-contract.md` with a ≤25-line stub pointing at `execute/references/stop-conditions.md` + `.specify/journal.jsonl` (RFC-25 §Observability).
2. Delete `drop/SKILL.md:17–19` self-heal paragraph; one sentence: "non-interactive mode forwards `--reason` to `specify slice drop`."
3. Trim `plugins/references/guardrails.md:13` (journal.yaml bullet) and `docs/reference/cli/plan.md:14,56` (failed/skipped transitions).

**Quality delta.** −~110 LOC prose, −4 skill/CLI drift surfaces, −2 dead CLI verbs documented.

**Net LOC.** 127 → ~25 (+ sibling trims ≈ −110 total).

**Done when.** `rg -n 'journal\.yaml|slice journal append|plan transition.*(failed|blocked|skipped)' plugins/spec/` → 0; `make checks` green.

**Rule?** No.

**Counter-argument.** "Operators rely on the outcome table." Loses: no shipped skill invokes `specify slice outcome set`; execute parks on exit code + lifecycle, not outcome translation.

**Depends on.** none (F6 optional follow-on).

---

### F4 — Align lifecycle enum to RFC-25 wire

**Evidence.** Skills command `specify slice transition … refined|built` (`plugins/spec/skills/refine/SKILL.md:70`, `build/SKILL.md:34`). CLI accepts only `defining|defined|building|complete|dropped` (`src/commands/slice/cli.rs:54–58`). `LifecycleStatus` variants (`crates/domain/src/slice/lifecycle.rs:23–35`) never emit `refined`. `SliceTransitionRefined` journal variant exists (`crates/domain/src/journal.rs:95–98`) but production never emits it (post-F4 note in prior pass post-mortem).

**Action.**
1. Rename variants: `Defining→Refining`, `Defined→Refined`, `Complete→Built`; **delete** `Building` and `build_started_at` (no skill transitions to `building`; `rg 'transition.*building' plugins/` → 0).
2. Collapse edges to `refining→refined→built→merged` (+ `dropped` from any non-terminal).
3. In `slice/actions/transition.rs`, emit `EventKind::SliceTransitionRefined` on `Refined` target (replacing deleted dead branch).
4. Regenerate goldens touching `.metadata.yaml` status strings.

**Quality delta.** −~60 LOC (drop `Building` + timestamp field + tests), −1 enum variant, −2 branches, −1 call-site mismatch axis (**+** burden fix dominates).

**Net LOC.** ~140 → ~80 (rename churn washes; net from deletions).

**Done when.** `specify slice transition demo refined` succeeds; `rg -c 'LifecycleStatus::Building' specify-cli/` → 0; `cargo make ci` green.

**Rule?** No.

**Counter-argument.** "Keep 1.x names for metadata compat." Loses: pre-1.0, hard-cut policy; skills already speak RFC-25. *cargo* renamed unstable flags rather than maintaining dual vocab.

**Depends on.** none (unblocks deleting F5's orphan variant if wired here instead).

---

### F5 — Delete orphan `SliceTransitionRefined` (if F4 parked)

**Evidence.** `rg 'SliceTransitionRefined' specify-cli/` → 8 hits, all in `journal.rs` + `tests/journal.rs` + fixture `tests/fixtures/journal/slice-transition-refined.json`. Production emit path removed in prior F4 pass.

**Action.** Delete variant from `EventKind`, self-tests at `journal.rs:268–281,369`, `tests/journal.rs:232–281`, golden fixture; trim DECISIONS.md / architecture.md one-liners.

**Quality delta.** −~95 LOC, −1 enum variant, −1 golden fixture.

**Net LOC.** 95 → 0.

**Done when.** `rg -c 'SliceTransitionRefined|slice\.transition\.refined' specify-cli/` → 0.

**Rule?** No.

**Counter-argument.** "RFC-25 lists the event." Loses: an event with no emitter is schema fiction; restore via F4 or delete. *tokio* drops unused trace kinds when nothing produces them.

**Depends on.** F4 **or** "none" if choosing deletion over wiring.

---

### F6 — Delete `slice outcome set` command

**Evidence.** `rg 'outcome set' plugins/spec/skills/` → **0** SKILL hits (only `phase-outcome-contract.md` + `guardrails.md`). Command surface: `src/commands/slice/outcome.rs` = **210 LOC**; `OutcomeAction::Set` + `RegistryAmendmentRequired` clap subtree (`cli.rs:125–202`). Merge still stamps outcome internally (`merge/slice.rs:204`) — that writer stays.

**Action.**
1. Delete `OutcomeAction::Set` and `outcome::set()`; keep `Show` for archive reads.
2. Delete `OutcomeKind::RegistryAmendmentRequired` variant + clap arm (tests-only).
3. Remove `slice outcome set` tests block in `tests/slice.rs` (~lines 352–980 footprint; `rg -c outcome tests/slice.rs` → 112 lines touch outcome).

**Quality delta.** −~320 LOC, −1 enum variant, −1 clap subcommand tree, −4 call-site docs.

**Net LOC.** ~320 → ~90 (show path only).

**Done when.** `specify slice outcome --help` lists only `show`; `rg -c 'OutcomeAction::Set' src/` → 0; merge tests still pass.

**Rule?** No.

**Counter-argument.** "Execute reads outcome on return." Loses: current execute body (`execute/SKILL.md:40`) contradicts its own stop-conditions — it never shells `outcome show`. Keeping a write verb no skill calls is *cargo*'s removed `cargo test -- --exact` pattern: delete unused surface.

**Depends on.** F3 (doc drift).

---

### F7 — Drop dead `feature` terminology branch

**Evidence.** `validate/run.rs:88` hardcodes `let terminology = "crate"`. `rg '"feature"' specify-cli/crates/domain/src/validate` → only `proposal.rs:18` and `primitives.rs:150` match arms — **no caller passes `"feature"`**.

**Action.**
1. Remove `terminology` field from `BriefContext` / `CrossContext` (`validate.rs:87–125`).
2. Inline `"## Crates"` in `proposal.rs` and `primitives.rs`; delete match arms and parameter threading through `run.rs` (`run_brief_rules`, `run_cross_rules`).

**Quality delta.** −~38 LOC, −2 struct fields, −2 match arms, −1 parameter at 4 call sites.

**Net LOC.** ~55 → ~17.

**Done when.** `rg -c 'terminology' specify-cli/crates/domain/src/validate/` → 0; `cargo nextest run -p specify-domain -- validate` green.

**Rule?** No.

**Counter-argument.** "Vectis uses Features headings." Loses: Vectis validation moved to target briefs per `validate/run.rs:74–78`; deterministic runner is Omnia-biased and always was.

**Depends on.** none.

---

## One-touch tidies

### T1 — Delete guardrails `journal.yaml` bullet

**Evidence.** `plugins/references/guardrails.md:13` documents `specify slice journal append`; command deleted in prior pass (`rg 'slice journal' specify-cli/src/` → 0).

**Action.** Delete line 13.

**Quality delta.** −1 LOC, −1 drift surface.

**Done when.** `rg 'journal\.yaml' plugins/references/guardrails.md` → 0.

**Depends on.** none.

---

### T2 — Fix `spec.mdc` lifecycle diagram

**Evidence.** `plugins/spec/rules/spec.mdc:28–37` documents `pending -> refining -> refined -> building -> built` but CLI wire is `defining|defined|building|complete` (pre-F4) — double drift.

**Action.** Either align to F4 names or, if F4 parked, replace with actual CLI states in ≤5 lines.

**Quality delta.** wash-LOC, −1 skill/CLI drift surface.

**Done when.** Diagram states match `LifecycleStatus` strum serialisation.

**Depends on.** F4 (preferred).

---

### T3 — Delete `init-runbook` `pipeline.define` prose

**Evidence.** `rg 'pipeline\.define' plugins/spec/` → `init-runbook.md:117`, `topology-flow.md:59`; init scaffolds hardcoded keys (`init/regular.rs:27`: `SCAFFOLDED_RULE_KEYS`).

**Action.** Replace two sentences with "init scaffolds empty `rules:` entries for `proposal|specs|design|tasks`."

**Quality delta.** −~4 LOC, −1 doc/code drift.

**Done when.** `rg 'pipeline\.define' plugins/spec/` → 0.

**Depends on.** none.

---

### T4 — Collapse `pass`/`fail`/`deferred` helpers in `validate/run.rs`

**Evidence.** Three 7-line constructors (`run.rs:39–64`) differ only in `ValidationStatus`. Used at 2 call sites each.

**Action.** One `fn summary(status, rule_id, rule, detail: Option<String>) -> ValidationSummary` — only if ≥2 call sites shrink (they do: lines 117–122, 220, 244).

**Quality delta.** −~15 LOC, −2 functions.

**Done when.** `rg -c '^fn (pass|deferred)\(' specify-cli/crates/domain/src/validate/run.rs` → 0.

**Depends on.** none.

---

### T5 — Delete `checkInstructionPreambles`

**Evidence.** `scripts/checks/adapter.ts:190–213` walks `adapters/**/instructions/*.md`; `rg --files -g '**/instructions/*.md' specify/adapters` → **0 files**.

**Action.** Delete function + `checks.ts` registration.

**Quality delta.** −24 LOC, −1 dead predicate.

**Done when.** `rg 'checkInstructionPreambles' scripts/` → 0.

**Depends on.** F2.

---

## Ranking and dependencies

Prior pass findings (journal.yaml, ChangeBrief, ValidationResult, resolve collapse, ToolError trim, Pipeline.plan, DivergenceState, etc.) are **already applied** — do not re-land. This list is net-new only.

| Rank | ID | ΔLOC | Axes |
|---|---|---|---|
| 1 | F1 | −1908 | LOC, types, module, schemas |
| 2 | F2 | −553 | LOC, checks, module edge |
| 3 | F6 | −320 | LOC, enum, clap |
| 4 | F3 | −110 | LOC, drift |
| 5 | F4/F5 | −95–155 | LOC, branches, call-site |
| 6 | F7 | −38 | LOC, fields, arms |

Structural findings rank by LOC: **F1 (1908) > F2 (553) > F6 (320) > F3 (110) > F4/F5 (95–155) > F7 (38)**. Tidies T1–T5 collapse into F2/F3/F4 where noted. No new modules, traits, or dependencies.

---

## Post-mortem (prior pass — already applied)

One line per finding from the previous review pass; retained for calibration context only.

- **F1 (prior)** — Delete per-slice `journal.yaml` apparatus: actual −643 vs predicted −640; `rg -c 'journal\.yaml' crates/ src/` → 0.
- **F2 (prior)** — Delete `ChangeBrief` parser: actual −518 vs predicted −485.
- **F3 (prior)** — Unify `ValidationResult` with `ValidationSummary`: actual −53 vs predicted −120.
- **F4 (prior)** — Delete dead `to_string() == "refined"` branch: actual −13 vs predicted −12; `SliceTransitionRefined` now production-unused.
- **F5 (prior)** — Collapse source/target resolve commands: actual −45 vs predicted −50.
- **F6 (prior)** — Collapse `Divergence` + `DivergenceState`: actual −28 vs predicted −45.
- **F7 (prior)** — Trim `ToolError` variants: actual **+28** vs predicted −100 (sign flip; inline `Diag` rewrites cost more than helpers saved).
- **F8 (prior)** — Drop `Pipeline.plan` + `Phase::Plan`: actual **−220** vs predicted −30 (orphaned tests/fixtures tail).
- **T1 (prior)** — Drop stale `plan transition failed/blocked` from drop skill: actual −7 vs predicted −8.
- **T2 (prior)** — Inline `artifact_classes` as private fn: wash-LOC.
- **T7 (prior)** — Remove redundant `last` in `is_valid_source_key`: actual −2 vs predicted −5.
- **T9 (prior)** — Delete `Adapter::probe_dir`: actual −1 vs predicted −10 (audit miscounted callers).

### This pass

- **F1** — Delete RFC-20 survey module: actual **−2277** vs predicted −1908; `rg -c 'survey|SurfacesDocument' crates/ tests/ schemas/` → 0; `cargo make check` green; no regressions (extra −369 from integration tests/fixtures tail).
- **F2** — Retire `adapters/default` pipeline shell (+ T5): actual **−173** vs predicted −553; all `rg` done-when assertions → 0; `make checks` green; no regressions (undershoot: codex moved intact, validation retargeted not deleted).
- **F3** — Gut stale phase-outcome contract (+ T1, T3): actual **−125** vs predicted −110; all `rg` done-when assertions → 0; `make checks` green; no regressions.
- **F4** — Align lifecycle enum to RFC-25 wire (+ T2): actual **−6** vs predicted ~−60; all done-when assertions flip cleanly; `cargo make check` + `make checks` green; `SliceTransitionRefined` wired (F5 N/A); no regressions (rename churn washed deletion wins).
- **F5** — Delete orphan `SliceTransitionRefined`: **skipped** — F4 wired the emit path; variant no longer orphaned.
- **F6** — Delete `slice outcome set`: actual **−686** vs predicted −320; all done-when assertions flip cleanly; `cargo make check` green; no regressions (test-block tail drove 2× overshoot).
- **F7** — Drop dead `feature` terminology branch: actual **−43** vs predicted −38; `rg -c 'terminology' validate/` → 0; validate tests green; no regressions (already in working tree; sign did not invert unlike prior F7).
- **T4** — Collapse `pass`/`fail`/`deferred` helpers: actual **−15** vs predicted −15; `rg` done-when → 0; `cargo make check` green; no regressions.
- **T1/T2/T3/T5** — Folded into F3/F4/F2 respectively; no separate post-mortem lines.

### Calibration shape (prior pass)

- **Pure deletions of dead surface** can blow through prediction by 5–7× when orphaned tests/fixtures ride along (F8).
- **Unifications** undershoot 10–56% because helper bodies and doc-blocks absorb LOC.
- **Enum-trim with inline `Diag` rewrites** can invert the sign when `rustfmt` reflow exceeds helper savings (F7).
- **Audit miscounts on callers** — always cross-check bare method names, not just type-qualified forms (T9).
