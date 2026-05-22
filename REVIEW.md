# Code & Skill Review — Subtraction Pass

## Summary

Top three by LOC removed: (F1) delete the hand-maintained `rfcs/rfc-27-synthesis.html` duplicate of the archived RFC markdown (−2006 LOC); (F2) delete the pre-1.0 migration doc tree and stale `migrate-to-2.0.sh` prose still advertising a removed script (−≈165 LOC); (F3) collapse `FusionStatus` into the existing `RequirementStatus` enum (−≈55 LOC, −1 mirror type). If every finding lands: ≈2200 LOC. Non-LOC axes moved most: types (`FusionStatus`, `OverrideTrace`), module edges (retargeted RFC-27 links), call-site burden (one status enum on the fusion wire path). Highest remediation risk: F3 — `fusion.yaml` kebab-case status values and `tests/slice.rs` golden envelopes pin the serde shape byte-for-byte.

## Reconnaissance

- `tokei` (combined): 156 805 lines / 1092 files — specify 85 406 / 606; specify-cli 71 399 / 486. Rust 46 945 + 46969 = 93 914 lines across 284 files.
- `cargo tree --duplicates` (specify-cli): duplicates are entirely transitive from `wasm-pkg-client` (`base64 0.21/0.22`, `pbjson`, `warg-*`, `oci-client`, `reqwest 0.12/0.13`) — not actionable without new `Cargo.toml` edges (frozen).
- `rg -c '^#\[test\]' crates/ src/ tests/` (specify-cli): **584** `#[test]` functions across 42 files; heaviest: `tests/plan_orchestrate.rs` (77), `crates/domain/tests/registry.rs` (50), `tests/slice.rs` (44).
- `rg --files -g '**/mod.rs'` (specify-cli): **3** hits — all test shims (`tests/common/mod.rs`, `crates/domain/tests/common/mod.rs`, `wasi-tools/vectis/tests/engine_support/mod.rs`).
- `wc -l docs/standards/*.md AGENTS.md` (combined): **986** total — specify 348, specify-cli 638.
- Files > 500 lines under `crates/` and `src/` (specify-cli): `tests/plan_orchestrate.rs` 2585; `src/commands/plan/create.rs` 1079; `crates/domain/src/slice/fusion.rs` 976; `crates/domain/tests/workspace.rs` 1041; `crates/domain/tests/finalize.rs` 947; `crates/domain/src/discovery/document.rs` 943; `crates/domain/tests/registry.rs` 922; `crates/domain/src/change/plan/core/model.rs` 841; `crates/domain/src/journal.rs` 765; `crates/domain/src/spec/provenance.rs` 643; `crates/tool/src/validate.rs` 520.
- Prior pass already landed: migration script/fixtures deleted; plan/slice transition collapse (`transitions.rs` 109, `lifecycle.rs` 88); `CmdRunner` trait → borrowed-dyn alias (`cmd.rs` 21); legacy docs-quality predicates removed (`docs_quality.ts` 65); `Counts::from_entries` BTreeMap dance replaced (net 0 LOC — `struct_lit_width = 20` reflow).
- `make checks` (specify): **16 failures**, all broken links to `rfcs/rfc-27-synthesis.md` (file lives at `rfcs/archive/rfc-27-synthesis.md`; only `rfcs/rfc-27-synthesis.html` remains at the old path).
- `rg 'migrate-to-2|migrate_to_2_0' --glob '!rfcs/archive'` (specify): **4 live hits** — `docs/explanation/release-notes.md`, `docs/explanation/decision-log.md`, `REVIEW.md`, `rfcs/rfc-27-synthesis.html` (migration script itself is gone; prose is stale).

## Structural Findings

### F1 — Delete `rfcs/rfc-27-synthesis.html`

- **Evidence**:
  - `wc -l rfcs/rfc-27-synthesis.html` → **2006** lines (`tokei` reports HTML 1414 code / 2006 raw — the only HTML source file in the repo).
  - Canonical normative text: `wc -l rfcs/archive/rfc-27-synthesis.md` → **795** lines.
  - `rg -l 'rfc-27-synthesis\.html' --glob '!REVIEW.md'` → **0** source-tree linkers; the HTML is maintained in parallel (both touched May 23) but nothing references it.
  - Live docs link the missing `rfcs/rfc-27-synthesis.md` path (`make checks` → 16 broken-link failures in `adapters/sources/code-runtime/briefs/*.md`, `plugins/spec/references/synthesis/*.md`, `docs/migration/2.1.md`).
- **Action**:
  1. `rm rfcs/rfc-27-synthesis.html`.
  2. Retarget every live `rfcs/rfc-27-synthesis.md` link to `rfcs/archive/rfc-27-synthesis.md` (8 files per `rg 'rfc-27-synthesis\.md' --glob '!rfcs/archive/**'`).
- **Quality delta**: −2006 LOC, −1 hand-styled CSS surface, −16 broken-link failures.
- **Net LOC**: 2006 → 0 (link retargets are path-string edits, ≈0 net).
- **Done when**: `tokei` reports **HTML 0 files** under `rfcs/` and `make checks` broken-link count drops by 16 (currently 16 total failures).
- **Rule?**: no.
- **Counter-argument**: "HTML is nicer for stakeholders" — loses because no in-tree consumer links to it; render from archived markdown on demand.
- **Depends on**: none.

### F2 — Delete migration doc + stale 2.0 script prose

- **Evidence**:
  - `wc -l docs/migration/2.1.md` → **149** lines; `docs/SUMMARY.md:69` still links it.
  - `glob scripts/migrate*` → **0 files** (2.0 migration apparatus already deleted).
  - `rg -n 'migrate-to-2\.0' docs/explanation/` → release-notes.md:15, decision-log.md:327 — both describe a script that no longer ships.
  - Parent `AGENTS.md`: "2.0 is a hard cut from 1.x"; review brief: "Pre-1.0 — ignore back-compat, migrations, deprecations."
  - `docs/migration/2.1.md` itself says "There is no migration script" — the page duplicates release-notes upgrade guidance.
- **Action**:
  1. `rm docs/migration/2.1.md`; drop the SUMMARY entry at line 69.
  2. Replace the **Migration** paragraph in `docs/explanation/release-notes.md:15` with one sentence: hard cut, no in-tree upgrade script; bump binary + reload plugins.
  3. Trim `docs/explanation/decision-log.md:327` to drop the `migrate-to-2.0.sh` inventory — keep the "hard cut / no aliases" decision, delete the script bullet list.
- **Quality delta**: −≈165 LOC, −1 docs subtree, −2 stale operator instructions.
- **Net LOC**: ≈165 → ≈8 (two one-line replacements).
- **Done when**: `rg 'migrate-to-2|docs/migration' --glob '!rfcs/archive/**'` returns zero hits outside `REVIEW.md`.
- **Rule?**: no.
- **Counter-argument**: "Operators need the 2.1 feature inventory" — loses because `docs/explanation/release-notes.md` and `AGENTS.md` already carry the vocabulary; the migration page repeats them.
- **Depends on**: none.

### F3 — Collapse `FusionStatus` into `RequirementStatus`

- **Evidence**:
  - `crates/domain/src/slice/fusion.rs:83–124` — `FusionStatus` enum mirrors `RequirementStatus` variant-for-variant; comment at :86–89 admits the split exists only because `RequirementStatus` lacked serde.
  - `rg 'FusionStatus' crates/` → **19 hits, all in `fusion.rs`**; `From` impls are consumed only by the `requirement_status_round_trip_via_from_into` test (:671–683).
  - `schemas/slice/fusion.schema.json:71–74` — wire enum `["agreed","unknown","conflict","divergence"]` is byte-identical to `RequirementStatus::as_str()`.
  - `RequirementStatus` today: `crates/domain/src/spec/provenance.rs:101–111` — four variants, no serde.
- **Action**:
  1. Add `Serialize, Deserialize` + `#[serde(rename_all = "kebab-case")]` to `RequirementStatus` in `provenance.rs`.
  2. Change `FusionRequirement.status` to `RequirementStatus`; delete `FusionStatus`, both `From` impls, `fusion_status_round_trip_kebab_case`, and `requirement_status_round_trip_via_from_into`.
  3. Replace `FusionStatus::Agreed` etc. in `sample()` and drift tests with `RequirementStatus::Agreed`.
  4. Move the kebab-case serde pin to `crates/domain/src/spec/provenance/tests.rs` as one loop over the four variants.
- **Quality delta**: −≈55 LOC, −1 enum, −2 `From` impls, −1 module-edge mirror.
- **Net LOC**: fusion.rs ≈976 → ≈920; provenance.rs +≈3.
- **Done when**: `rg 'FusionStatus' crates/` returns **0** and `cargo make check` green.
- **Rule?**: no.
- **Counter-argument**: "Separate wire type insulates the parser from fusion schema churn" — loses because the schema `$ref` already names `requirementStatus` and the variant sets are identical; one enum makes drift impossible.
- **Depends on**: none.

### F4 — Delete `OverrideTrace` reserved field

- **Evidence**:
  - `crates/domain/src/spec/provenance.rs:53–98` — `override_trace: Option<OverrideTrace>` plus `OverrideTrace` struct; doc at :57–67 states `parse_spec_md` **always** leaves it `None` and validation never reads it.
  - `rg 'override_trace|OverrideTrace' crates/ tests/` → **3 hits**: field definition, struct definition, `override_trace: None` in parser (:577).
  - Authoritative trace lives on `fusion.yaml` via `ResolutionTrace` (`crates/domain/src/slice/fusion.rs:192`).
- **Action**:
  1. Delete `OverrideTrace` struct and `Requirement.override_trace` field plus the 15-line doc block.
  2. Drop `override_trace: None` from the parser constructor.
- **Quality delta**: −≈50 LOC, −1 DTO, −1 dead struct field on the hot parse path.
- **Net LOC**: provenance.rs ≈643 → ≈593.
- **Done when**: `rg 'OverrideTrace|override_trace' crates/` returns **0**.
- **Rule?**: no.
- **Counter-argument**: "Refine synthesis will populate it in Change 3.2" — loses because `fusion.yaml` + `ResolutionTrace` already owns that data; duplicating it on `Requirement` adds a second write surface with zero readers.
- **Depends on**: none.

### F5 — Delete journal unit tests duplicated by `tests/journal.rs`

- **Evidence**:
  - `wc -l crates/domain/src/journal.rs` → **765** (production ≈361, `#[cfg(test)]` mod ≈404).
  - `tests/journal.rs` — 13 integration tests with golden fixtures under `tests/fixtures/journal/` covering the same wire shapes end-to-end.
  - Overlap:
    - `journal.rs:378` `plan_transition_reviewed_wire_shape` ↔ `tests/journal.rs:67` `plan_transition_reviewed_emits_journal_event`
    - `journal.rs:398` `plan_amend_divergence_wire_shape` ↔ `tests/journal.rs:110+` divergence amend suite
  - Unit-only tests (cache hit/miss, authority-override, fusion-written, append_batch) **stay** — no integration coverage.
- **Action**: Delete the two overlapping unit tests and their helper assertions (~62 lines); keep the remaining 15 unit tests.
- **Quality delta**: −≈62 LOC, −2 duplicate wire-shape oracles.
- **Net LOC**: journal.rs 765 → ≈703.
- **Done when**: `rg 'plan_transition_reviewed_wire_shape|plan_amend_divergence_wire_shape' crates/` returns **0** and `cargo test -p specify-domain journal::` + `cargo test --test journal` both green.
- **Rule?**: no.
- **Counter-argument**: "Unit tests pin exact JSON strings cheaper than CLI goldens" — loses because `tests/journal.rs` already pins the same strings via normalized goldens; keeping both means two places update on every payload tweak.
- **Depends on**: none.

## One-touch tidies

1. **Delete `SliceSourceBinding` `From` impls** — `crates/domain/src/change/plan/core/model.rs:311–320`; `rg 'SliceSourceBinding::from\(' crates/` → **0**; call sites already use `SliceSourceBinding::Bare("…".into())`. Δ: −12 LOC, −2 trait impls.

2. **Trim refine Step 5 fusion duplication** — `plugins/spec/skills/refine/SKILL.md:64–71` restates block grammar owned by `plugins/spec/references/synthesis/fusion.md`; replace the eight bullet lines with `Follow the block grammar in [fusion.md](../../references/synthesis/fusion.md).` Δ: −≈10 LOC body.

3. **Retarget stale `plugins/change` refs** — `adapters/sources/code-typescript/references/semantic-search.md:270,346` cite deleted `plugins/change/skills/analyze/SKILL.md`; point at `plugins/rt/skills/wiretapper/SKILL.md` §*Cloning a source tree* (the inlined clone snippet that replaced analyze). Δ: −0 LOC, −2 broken institutional paths; `rg 'plugins/change' adapters/` → 0.

4. **Delete stale Phase 1/2 comment in `src/output.rs`** — lines 54–67 document RFC-27 discriminants "landing in Phase 2"; wiring is live. Delete the 14-line table. Δ: −14 LOC.

5. **Delete `requirement_status_round_trip_via_from_into` when F3 lands** — subsumed by F3; listed here only if F3 is deferred. Δ: −12 LOC.

6. **Inline `cli_patch` at its two call sites** — `src/commands/plan/create.rs:24–30` is a 7-line helper used twice in the same file; inline the `match` and drop the fn. Δ: −≈4 LOC, −1 helper. (Borderline; land only if touching `create.rs` anyway.)

7. **Drop `SliceAuthorityOverride::is_empty`** — `model.rs:252–255`; single caller `validate.rs:256` can use `authority_override.by_kind.is_empty()` directly. Δ: −≈6 LOC, −1 method.

8. **Tighten `docs/explanation/decision-log.md` Layer 1/2 vocabulary** — lines 28–29 still name retired `/spec:define` and `/spec:extract` as live Layer 1 verbs; one-line rewrite to `/spec:refine` / `/spec:build` / `/spec:merge`. Δ: −≈4 LOC, −2 stale verb names in operator-adjacent prose (decision-log is allow-listed for RFC citations but not for dead verbs).

## Dropped findings

- **Delete all `journal.rs` unit tests (~404 LOC)** — would orphan cache-hit/miss, authority-override, and append_batch coverage with no integration replacement. Kept.
- **Promote recurring `Error::Diag` codes to typed variants** — `rg 'code: ?"' crates/ src/` shows hundreds of unique discriminants; promotion adds enum + `Exit::from` rows for net `+` LOC. Kept.
- **Delete `Patch<T>` three-way enum** — 18 LOC including `apply`; call sites in `amend.rs` (13 uses) would need a clumsier `Option<Option<T>>` or flag pair. Kept.
- **Collapse `change/plan/core/` into one file** — 8 modules ≈2800 LOC combined; violates file-size norm in `docs/standards/architecture.md`. Kept.
- **Delete `docs/migration/2.1.md` without retargeting RFC-27 links** — F1 and F2 must land together or `make checks` stays red. Not independent.
- **Move `discovery/document.rs` inline tests to `tests/`** — adds a file; forbidden unless net-negative. Kept.
- **Dedupe `PLATFORM_V2_PLAN` / `RFC_EXAMPLE_YAML`** — `tests/plan_orchestrate.rs:123` extended fixture differs from `test_support.rs:12` (extra description lines, 9 vs 3 slices in `model.rs` test copy); merging changes pinned golden shapes. Kept.

## Post-mortem

One line per applied finding: actual ΔLOC vs predicted, did the "done when" assertion flip cleanly, did anything regress.

- **F1** — actual −2006 vs predicted −2006 (on target; HTML was orphan on disk, git already deleted in `483344b`); link retargets 0 net across 5 files (10 path edits); `tokei rfcs/` HTML 0 files; broken links 16→6 (−10 synthesis cleared; 6 pre-existing `rfc-27-plan`/`acceptance-matrix` remain, not F1 scope); no regressions.
- **F2** — actual −159 vs predicted ≈165 (−4%); deleted `docs/migration/2.1.md` + SUMMARY entry, replaced stale `migrate-to-2.0.sh` prose with hard-cut one-liner; `rg 'migrate-to-2|docs/migration'` clean outside REVIEW.md; `make checks` 2 pre-existing `rfc-27-plan` link failures only; no regressions.
- **F3** — actual −56 vs predicted ≈55 (on target); `FusionStatus` deleted, `RequirementStatus` gains kebab-case serde; `rg 'FusionStatus' crates/` 0; goldens byte-stable (no regen); `cargo make check` green; auxiliary folded-YAML assertion fixes in cache-meta/context tests (pre-existing drift, not wire-shape); no regressions.
- **F4** — actual −44 vs predicted ≈50 (−12%); deleted `OverrideTrace` + `Requirement.override_trace`; `rg 'OverrideTrace|override_trace' crates/` 0; `cargo make check` green; no regressions.
- **F5** — actual −64 vs predicted ≈62 (+3%); deleted duplicate journal wire-shape unit tests; `rg 'plan_transition_reviewed_wire_shape|plan_amend_divergence_wire_shape' crates/` 0; 13 domain + 13 integration journal tests pass; `cargo make check` green; no regressions.
- **T1** — actual −9 net (−12 in `model.rs`) vs predicted −12; deleted `SliceSourceBinding` `From` impls; 11 test sites needed explicit `SliceSourceBinding::Bare` (audit missed implicit `.into()`); `rg 'SliceSourceBinding::from\(' crates/` 0; `cargo make check` green; no regressions.
- **T2** — actual −7 vs predicted ≈10; Step 5 fusion grammar bullets replaced with single `fusion.md` link; `make checks` 2 pre-existing `rfc-27-plan` failures only; no regressions.
- **T3** — actual 0 net vs predicted 0; retargeted 2 `plugins/change` refs in `semantic-search.md` → wiretapper §Step 0; `rg 'plugins/change' adapters/` 0; `make checks` 2 pre-existing failures only; no regressions.
- **T4** — actual −15 vs predicted −14; deleted stale Phase 1/2 comment table in `src/output.rs`; `cargo make check` green; no regressions.
- **T5** — subsumed by F3; no separate work.
- **T6** — actual +1 vs predicted −4 (sign flip; three call sites not two, inline duplicates `match`); `cli_patch` inlined in `create.rs`; `cargo make check` green after T7 landed; no regressions.
- **T7** — actual −2 net vs predicted −6; dropped `SliceAuthorityOverride::is_empty`; serde elision moved to module-local `slice_authority_override_is_empty`; `cargo make check` green; no regressions.
- **T8** — actual 0 net vs predicted −4; Layer 1/2 verbs updated to `/spec:refine`/`build`/`merge`; `make checks` 2 pre-existing failures only; no regressions.

### Final deep validation

`cargo make check` green in specify-cli after all CLI items. `make checks` in specify: 2 pre-existing broken links to `rfcs/rfc-27-plan.md` (unchanged since F1/F2; not introduced this pass). `cargo make ci` in specify-cli: lint + test + test-docs + doc + vet + outdated green; **deny failed** on pre-existing `RUSTSEC-2026-0149` (`wasmtime-wasi` 44.0.1 → needs ≥44.0.2) — supply-chain advisory unrelated to subtraction changes.

### Totals across this session

Structural (F1–F5): predicted ≈2327 LOC removed, actual **−2329** (F1 −2006 on target; F2–F5 within ±12%). Tidies (T1–T4, T6–T8; T5 subsumed): predicted ≈−46 net, actual **−28** (T6 sign-flip +1 from three-site inline). **Grand total ≈−2357 LOC.**

Calibration shape (this pass):
- **Pure deletions** (F1, F2, F4, F5, T4): hit ±12% of prediction; F1 baseline miscounted 6 non-synthesis broken links.
- **Unifications** (F3): on prediction (−56 vs −55); wire byte-stable without golden regen.
- **Audit miscounts on callers** (T1 implicit `.into()`, T6 third call site): net LOC can undershoot or flip sign when grep counts explicit forms only.
- **Serde-adjacent method deletion** (T7): predicted −6, actual −2 — `skip_serializing_if` needs a replacement helper, not a bare field check.
