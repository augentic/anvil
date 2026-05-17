# Senior-engineer review — `specify` + `specify-cli` (Pre-1.0, subtraction-biased)

## Summary

- Three findings; if all land: **−540 LOC**, **−3 modules**, **−1 public trait**, **−2 enums**, **−4 structs**, **−2 error discriminants**.
- Top three by LOC: F1 dead detector subsystem (−496), F2 collapse `dispatch_survey`/`SurveyArgs` (−27), T1 drop dead `extract_code` 3rd arm (−7).
- Primary non-LOC axes: types (−5 in F1), branches (−2 error variants in F1), module edges (−3 modules in F1).
- Riskiest in remediation: F1 — RFC-20 calls the detector trait a "deferred extension point". Justification below; fold the prose move into the same diff so AGENTS / SKILLs don't go stale.

## Reconnaissance (current state)

- `tokei`: Rust = **50,429 lines / 281 files**, Markdown = **66,636 / 677**.
- `wc -l crates/domain/src/survey/*.rs` → `detector.rs 108 + registry.rs 41 + merge.rs 66 = 215 LOC`.
- `wc -l crates/domain/tests/survey.rs` → `696`; lines 410–684 are detector tests = **275 LOC**.
- `rg '\bLanguage::' --type rust` outside `survey/detector.rs` and `tests/survey.rs` returns only the unrelated Crux `Language::{Swift,Kotlin,Typescript}` in `templates/vectis/core/codegen.rs`. **Zero production callers of `survey::Language`.**
- `rg 'merge_detector_outputs|DetectorRegistry|with_builtins' --type rust` outside `crates/domain/src/survey/{detector,registry,merge}.rs` and `crates/domain/tests/survey.rs` returns **zero hits**.
- `find plugins -type l` confirms apparent doc duplicates (`plugins/omnia/skills/crate-writer/references/capabilities.md` etc.) are symlinks, not copies — no LOC duplication. Drop that line of inquiry.
- `cargo tree --duplicates` shows `base64 0.21/0.22` and crossings through `warg-*`, `oci-client`, `hyper-util`, `reqwest` 0.12/0.13. All third-party transitive; not actionable here.

---

## Structural findings (ranked by LOC removed)

### F1. Delete the deferred detector subsystem

**Evidence**

- `crates/domain/src/survey.rs:1-24` documents detector / registry / merge as "deferred extension points… v1 ships the registry empty; every legacy-code source flows through the agent-driven `ingest` pipeline."
- `crates/domain/src/survey/registry.rs:25-29` — `with_builtins()` literally returns `Self { detectors: Vec::new() }`. The `iter()` accessor and `impl Debug for dyn Detector + Send + Sync` exist only to satisfy `#[derive(Debug)]` on an empty `Vec`.
- `crates/domain/src/survey/merge.rs:1-66` — module doc-comment: "the `detector-failure` and `detector-id-collision` codes are unreachable through the CLI handler today."
- `crates/domain/src/survey/detector.rs:1-108` — `Detector` trait, `DetectorInput`, `DetectorOutput`, `DetectorError`, and the `Language` enum.
- `git status` already shows all three v1 detector implementations (`detectors/{express,nestjs,bullmq}.rs`), their `tests/detectors_*.rs` integration tests, and their fixtures **deleted on the working tree**, with `crates/domain/src/survey/ingest.rs` newly added — the architectural pivot has already happened.
- `rg` proves zero production call sites for `merge_detector_outputs`, `DetectorRegistry`, `with_builtins`, or `survey::Language` outside the dead-code island.
- `crates/domain/tests/survey.rs` lines 410–684 (275 LOC: `MockDetector`, `merge_detector_*`, `language_serde_round_trip`) exercise nothing the production binary can reach.

**Action**

1. `rm crates/domain/src/survey/{detector,registry,merge}.rs`.
2. In `crates/domain/src/survey.rs`, drop `pub mod detector;`, `pub mod merge;`, `pub mod registry;` and the three matching `pub use` lines.
3. In `crates/domain/tests/survey.rs`, delete from the first `// ── Detector contract ──` banner (line 410) through the closing brace of `language_serde_round_trip` (line 684); keep `assert_has_finding` at line 686+ (also called by surfaces tests). Remove the now-unused `use specify_domain::survey::Language` (line 667).
4. Tighten the module doc-comment on `crates/domain/src/survey.rs:1-7` to "DTOs, validators, sources file, and ingest pipeline." (drop the "deferred extension point" prose).
5. `rg 'DetectorRegistry|merge_detector_outputs|survey::Language' plugins/ docs/ AGENTS.md rfcs/` and update every hit in the same diff (per `docs/standards/coding-standards.md` §Drift audit).

**Quality delta** `−496 LOC, −3 modules, −1 trait (Detector), −2 enums (Language, DetectorError), −2 structs (DetectorInput, DetectorOutput), −1 struct (DetectorRegistry), −2 error discriminants (detector-failure, detector-id-collision), −1 cross-module Debug impl on `dyn Detector`.`

**Net LOC** `survey source 990 + survey tests 696 = 1686  →  990 − 215  +  696 − 275  =  1196` (≈ 30% smaller).

**Done when** `rg --type rust 'Detector|merge_detector_outputs|with_builtins|survey::Language'` returns **0 hits** outside `templates/vectis/core/codegen.rs` (the unrelated Crux `Language` enum). Currently returns ~50.

**Rule?** No. One-shot deletion; nothing recurring to enforce.

**Counter-argument** "RFC-20 §Future mechanical reversion explicitly preserves the trait so a future detector can replace the agent producer for one (language, framework) pair." Loses because pre-1.0 means `git revert` is the time machine — paying 496 LOC of carrying cost forever to save one diff in a hypothetical v2 is the textbook YAGNI tax, and the RFC explicitly says the **artifact contract** doesn't change in either direction (i.e., the DTOs in `dto.rs` are the actual reversion seam, not the trait).

**Depends on** none.

---

### F2. Collapse `dispatch_survey` + `SurveyArgs` into one match arm

**Evidence**

- `src/commands/change.rs:26-93` — the `ChangeAction::Survey { … 7 fields }` arm copies its 7 fields into a private `SurveyArgs` struct, then `dispatch_survey` immediately destructures `SurveyArgs` to match on the same 5 `Option<…>` fields and build a `survey::Form::Single | Batch`.
- `wc -l src/commands/change.rs` = `365`; `SurveyArgs` is used at exactly one site.
- `src/commands/change/survey.rs:21-48` already defines `Form::{Single, Batch}` with the matching shape; the dispatch is the only thing standing between clap and the form.

**Action**

1. Inline the field→`Form` resolution into `commands::change::survey::run` (or a new `survey::Form::resolve(...)` taking the clap fields).
2. In `src/commands/change.rs`, replace the `ChangeAction::Survey { … } => dispatch_survey(...)` arm with a one-liner: `ChangeAction::Survey { source_path, source_key, surfaces, sources, staged, out, validate_only } => survey::run(ctx, source_path, source_key, surfaces, sources, staged, out, validate_only)`.
3. Delete `struct SurveyArgs` (lines 49-57) and `fn dispatch_survey` (lines 59-93).

**Quality delta** `−27 LOC, −1 type (SurveyArgs), −1 fn (dispatch_survey).` Same hand-rolled idiom that ripgrep / jj use: clap arm → handler with named args, no DTO between.

**Net LOC** `change.rs 365 → ~338`.

**Done when** `rg 'SurveyArgs|dispatch_survey' src/` returns **0 hits**. Currently returns 7.

**Rule?** No.

**Counter-argument** "Bundling args in a struct improves readability." Loses by the master rule — readability is unfalsifiable, and the struct exists at one call site for one frame of life. ripgrep / cargo handle 7-field clap arms by passing them through directly.

**Depends on** none.

---

## One-touch tidies

### T1. `extract_code` in `commands/change/survey.rs:283-289` lies about its third arm

**Evidence** `src/commands/change/survey.rs:283-289` returns `"io"` for any non-`Diag`, non-`Validation` `Error`, including `ArtifactNotFound`, `Filesystem`, `BranchPrepareFailed`, `CliTooOld`, `NotInitialized`, `YamlDe`, `YamlSer`. `Error::variant_str(&self) -> String` already exists in `crates/error/src/error.rs:161-174` and returns the truthful kebab discriminant.

**Action** Replace the body of `extract_code` with `err.variant_str()`; widen `RowError.code` (line 115) from `&'static str` to `String`. Inline the helper at its single call site.

**Quality delta** `−7 LOC, −1 hand-rolled match, −1 lying default arm.`

**Done when** `rg 'fn extract_code' src/` returns **0 hits**. Currently returns 1.

**Rule?** No.

**Counter-argument** "`&'static str` is cheaper than `String`." Loses because the surrounding `RowError.detail: String` already heap-allocates; a `String` for `code` is rounding error and the truthful value is worth more.

**Depends on** none.

### T2. Drop the `survey/SKILL.md` line-83 RFC citation in body prose

**Evidence** `plugins/change/skills/survey/SKILL.md:83` reads `See [rfcs/rfc-20-survey.md](../../../../rfcs/rfc-20-survey.md) §"Determinism Policy" for the long form.` `docs/standards/skill-authoring.md:50` (rule 3): "**No RFC citations in skill bodies.** … Mechanically enforced by `checkNoRfcCitationsInSkillBody`." The same RFC appears (correctly) in the `## Reference Documentation` table at line 130.

**Action** Delete the second sentence of the `## Determinism policy` paragraph (line 83); keep the bulleted summary that follows. The reference table at line 130 already carries the link.

**Quality delta** `−1 LOC, −1 standards violation.`

**Done when** `rg '§"Determinism Policy"' plugins/` returns **0 hits**. Currently returns 1.

**Rule?** No — the predicate already exists; this is just one stale violation.

**Counter-argument** "Linking to the RFC inline helps reviewers." Loses — the reference table at the bottom is the canonical location, and skill discovery loads bodies into context where the inline link is dead weight.

**Depends on** none.

---

## Items considered and dropped

- **Duplicate `*.md` references under `plugins/omnia/skills/{crate,test}-writer/`** — `find plugins -type l` confirms they are symlinks. Zero LOC.
- **`base64 0.21` vs `0.22` cargo duplication** — entirely transitive through `warg-*`, `oci-client`, `hyper-util`, `reqwest` 0.12/0.13. Cannot dedupe without dropping `wasm-pkg-client`. Out of scope.
- **`is_kebab` placement in `specify-error`** — eight production call sites across the binary and three workspace crates; the leaf is the right home. Workspace-wide reuse beats the "leaf has only thiserror + saphyr" purity claim in `AGENTS.md`.
- **`SurfaceKind` / `SurfacesDocument` consolidation** — schema closure (`additionalProperties: false`) and the closed enum are load-bearing for the wire contract; no axis moves in the reducing direction.
- **`commands/change.rs:337-364` handler-level tests for `Landing`** — could move to `crates/domain/tests/finalize.rs`, but this is a relocation, not a deletion; net ~0 LOC and no axis improvement that beats taste.
- **`crates/tool/src/{package.rs (504), validate.rs (459), host.rs (376), manifest.rs (360)}`** — read-only spot-checks didn't surface a clean subtraction; each file pays for a real wire-contract surface (wasm-pkg client, JSON Schema validation, wasmtime host, manifest shape). Not worth a finding without a prototype delete.
- **Skill body caps** — every `SKILL.md` is below the 200-line cap; the largest (`plugins/omnia/skills/code-reviewer/SKILL.md`, 163) clears comfortably. No structural skill finding.
