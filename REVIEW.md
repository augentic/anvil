# Code & Skill Review — specify + specify-cli

Top three findings by tier: **F1 Delete retired `outcome set` / `journal append` from merge briefs** (wire-contract defect, ~−130 LOC), **F2 Stop double cycle diagnostics on `plan validate`** (wire-contract defect, ~−34 LOC), **F3 Single-pass `specs/` scan in `slice validate`** (~−55 LOC, hot-path I/O).
Total ΔLOC if all land: **approximately −380 LOC**.
Primary non-LOC axes moved: defect surface (nonexistent CLI verbs, duplicate cycle rows), duplicate I/O on validate hot paths, test duplication, check-script duplication.
Top verified defects closed: **3** — merge briefs cite `specify slice outcome set` / `specify slice journal append` (verbs absent from `src/commands/slice/cli.rs`), `specify plan validate` emits two cycle codes for one SCC, execute skill reads retired `phase outcome` from `.metadata.yaml`; defect-only net ΔLOC: **~−134** (F1 + F2; portfolio cap unused).
Most likely to break in remediation: **F2** — `plan.next` currently blocks cycles only through `Plan::validate`'s `dependency-cycle` findings; moving the gate to `doctor::cycle::detect` must preserve the `plan-structural-errors` refusal before `advance_next`.

## Reconnaissance

- `tokei`: **specify** **658** files / **89,907** lines (Markdown **525** / **51,952**; Rust **3** / **120**); **specify-cli** **453** files / **65,244** lines (Rust **250** / **47,814**).
- `cargo tree --duplicates` (`specify-cli`): non-empty — `base64 v0.21.7 / v0.22.1`, `reqwest v0.12.28 / v0.13.3`, multi-version `wasmtime` / `wasm-pkg-client` transitives. `Cargo.toml` frozen for this pass.
- `rg -c '^#\[test\]' crates/ src/ tests/` (`specify-cli`): summed **514** `#[test]` declarations.
- `rg --files -g '**/mod.rs'` (`specify-cli`): **3** files — `tests/common/mod.rs`, `crates/domain/tests/common/mod.rs`, `wasi-tools/vectis/tests/engine_support/mod.rs`.
- `wc -l docs/standards/*.md AGENTS.md`:
  - `specify`: **584 total**.
  - `specify-cli`: **780 total**.
- Files >500 lines under `crates/` and `src/` (`specify-cli`):
  - Tests: `crates/domain/tests/workspace.rs` **1048**, `crates/domain/tests/finalize.rs` **947**, `crates/domain/tests/registry.rs` **922**.
  - Source: `crates/domain/src/discovery/document.rs` **890**, `crates/domain/src/slice/fusion.rs` **843**, `crates/domain/src/adapter/core.rs` **742**, `crates/domain/src/change/plan/core/model.rs` **700**, `crates/domain/src/journal.rs` **659**, `crates/domain/src/spec/provenance.rs` **607**, `crates/tool/src/validate.rs` **520**, `crates/domain/src/adapter/cache/io.rs` **509**, `src/commands/plan/lifecycle.rs` **506**.
- `make check` (`specify`): **passed** — `All checks passed.` Total failures: **0**.
- `cargo make check` (`specify-cli`): **passed** — `[cargo-make] INFO - Build Done in 169.93 seconds.` First error: **none**.
- `rg -c '\.(unwrap|expect)\(' --glob '!**/tests/**' crates/ src/` (`specify-cli`): summed **698** matching lines (dominated by inline `#[cfg(test)]` modules inside source files; **no operator-path panic defect** in `src/commands` outside `#[cfg(test)]`).
- `rg -c 'panic!|unreachable!' --glob '!**/tests/**' crates/ src/` (`specify-cli`): summed **50** matching lines (same test-module inflation).

## Structural Findings

### F1 — Delete retired merge outcome commands

**Evidence:** RFC-25 retired `PhaseOutcome` and `specify slice outcome set` ([`plugins/spec/references/phase-outcome-contract.md:3`](plugins/spec/references/phase-outcome-contract.md)). The CLI exposes no such verbs — `src/commands/slice/cli.rs` lists only `create`, `validate`, `merge`, `task`, `transition`, `touched-specs`, `overlap`, `drop`. `rg 'specify slice outcome set' adapters/targets` returns **12** hits across the three merge briefs; each also instructs `specify slice journal append`, which is likewise absent from the slice CLI surface.

```text
adapters/targets/omnia/briefs/merge.md:85:specify slice outcome set $SLICE_NAME merge failure \
adapters/targets/vectis/briefs/merge.md:120:specify slice outcome set <slice> merge failure \
adapters/targets/contracts/briefs/merge.md:82:specify slice outcome set <slice> merge failure \
src/commands/slice/cli.rs:8-72   (no outcome / journal subcommands)
```

Merge brief totals: omnia **121**, vectis **169**, contracts **134** lines; § Outcome signalling blocks occupy **~52 / ~82 / ~82** lines respectively.

**Action:**
1. In each of the three merge briefs, delete § Outcome signalling / Outcome contract branches (`success` / `failure` / `deferred` / bash examples for retired verbs).
2. Replace with the pattern already used in [`plugins/spec/skills/merge/SKILL.md:37-47`](plugins/spec/skills/merge/SKILL.md) § Stop hint contract: one paragraph + bullet list (`slice`, `phase`, `failure-kind`, `paths`, `next-action`).
3. Keep only adapter-unique gate prose (Omnia cargo/wasm32, Vectis cap-matrix, Contracts validator modes) under existing pre-merge sections; link `phase-outcome-contract.md` in one line only.
4. In the same pass, fix [`docs/standards/skill-authoring.md:59`](docs/standards/skill-authoring.md) (drop `specify slice outcome set` from the canonical guardrails list) and [`docs/reference/cli/slice.md:134`](docs/reference/cli/slice.md) (remove `PhaseOutcome` stamp prose).

**Quality delta:** `−130 LOC, −1 wire-contract defect, −1 skill-integrity defect, −3 duplicate prose blocks`.

**Net LOC:** merge briefs **424 → ~294** combined; standards/cli doc touch **~−6**.

**Done when:** `rg 'specify slice outcome set|specify slice journal append' adapters/targets` returns **0**; `make check` passes.

**Rule?** no — retired contract, three briefs only.

**Counter-argument:** Journal append examples document the intended failure observability shape. It loses because the verbs do not exist in the binary — agents that follow the brief shell out to commands that fail immediately.

**Depends on:** none.

### F2 — One cycle diagnostic per plan validate

**Evidence:** `Plan::validate` runs `detect_cycles` (`crates/domain/src/change/plan/core/validate.rs:35,97-132`, code `dependency-cycle`). `plan_doctor` then runs the same graph again via `cycle::detect` (`crates/domain/src/change/plan/doctor.rs:167-170`, `doctor/cycle.rs:16-48`, code `cycle-in-depends-on`). Both share `entry_dependency_graph` (`validate.rs:74-90`). On a cyclic plan, `specify plan validate` emits **two** error rows for the same SCC.

```text
crates/domain/src/change/plan/core/validate.rs:35:        results.extend(detect_cycles(&self.entries));
crates/domain/src/change/plan/doctor.rs:170:    out.extend(cycle::detect(&plan.entries));
tests/plan_orchestrate.rs:1588:        .find(|d| d["code"] == "cycle-in-depends-on")
```

`plan.next` blocks only through `plan.validate` (`src/commands/plan/lifecycle.rs:110`), not `plan_doctor`.

**Action:**
1. Delete `detect_cycles` and its call at `validate.rs:35` (**~41 LOC**).
2. `pub use` `doctor::cycle::detect` (change `pub(super)` → `pub` in `doctor/cycle.rs`).
3. In `plan.next`'s `with_state` closure (`lifecycle.rs:109-117`), after loading the plan and before `advance_next`, refuse when `!cycle::detect(&plan.entries).is_empty()` with the existing `plan-structural-errors` envelope.
4. Retarget `crates/domain/src/change/plan/core/validate/tests.rs` cycle tests (`:33-59`) to call `cycle::detect` directly (expect `cycle-in-depends-on`-equivalent non-empty output) instead of `plan.validate`'s `dependency-cycle`.

**Quality delta:** `−34 LOC, −1 wire-contract defect, −1 duplicate algorithm, −1 defect surface (double cycle row)`.

**Net LOC:** `validate.rs` **359 → ~318**; `lifecycle.rs` **506 → ~512** (+6 gate lines).

**Done when:** `rg -n 'dependency-cycle' crates/domain/src/change/plan/core/validate.rs` returns **0** (only doctor/cycle path remains); `cargo test cycle_error cycle-in-depends-on plan_validate_structured_cycle_payload --test plan_orchestrate` passes; cyclic plan JSON validate returns **exactly one** cycle diagnostic.

**Rule?** no — single duplicate check.

**Counter-argument:** Dashboards route `dependency-cycle` separately from `cycle-in-depends-on`. It loses pre-1.0 because both describe the same SCC and the structured `cycle-in-depends-on` payload is strictly more useful.

**Depends on:** none.

### F3 — Single-pass spec.md scan in slice validate

**Evidence:** Three functions each walk `specs/**/*.md`, read, and `parse_spec_md`:

```text
src/commands/slice/validate.rs:120-140  collect_synthesis_tags
src/commands/slice/validate.rs:244-263  collect_spec_req_ids
src/commands/slice/validate.rs:273-303  validate_spec_provenance
```

All three call `collect_spec_files` + `read_to_string` + `provenance::parse_spec_md` independently on the operator hot path (`validate.rs:29-50`).

**Action:**
1. Add a private `struct ScannedSpec { path: PathBuf, parsed: ParsedSpec }` and `fn scan_slice_specs(slice_dir: &Path, source_keys: &BTreeSet<String>) -> Result<(BTreeSet<String>, Vec<(String, RequirementTag)>, Vec<ValidationSummary>)>` that walks once and fans out req-ids, synthesis tags, and provenance summaries.
2. Replace the three call sites in `run`, `collect_fusion_drift_findings` (pass pre-scanned req-ids), and delete `collect_synthesis_tags`, `collect_spec_req_ids`, and the inner loop of `validate_spec_provenance`.

**Quality delta:** `−55 LOC, −2 duplicate walks, lower call-site burden on validate hot path`.

**Net LOC:** `validate.rs` **382 → ~327**.

**Done when:** `rg -c 'fn collect_synthesis_tags|fn collect_spec_req_ids' src/commands/slice/validate.rs` returns **0**; `cargo test --test slice validate_passes_on_clean_fusion_inputs validate_flags_missing_req_id_in_fusion` passes.

**Rule?** no — one handler, three duplicate loops.

**Counter-argument:** Three small functions are easier to read in isolation. It loses because every `slice validate` invocation pays triple parse cost on the same files.

**Depends on:** none.

### F4 — Share evidence YAML path enumerator

**Evidence:** Near-identical `readdir` + `.yaml`/`.yml` filter + sort in:

```text
crates/domain/src/schema.rs:101-117       validate_evidence_dir
crates/domain/src/slice/fusion.rs:417-433 collect_evidence_claim_ids
```

Both run on every `specify slice validate` (`validate.rs:29` then `:204` via fusion drift) — every evidence file is walked twice.

**Action:**
1. Add `pub fn evidence_yaml_paths(slice_dir: &Path) -> Result<Vec<PathBuf>>` to `crates/domain/src/schema.rs` (move the shared loop there).
2. Replace both inline loops with calls to `evidence_yaml_paths`.
3. Optionally piggyback claim-id extraction on the schema-validation read in a follow-up; this finding stops at deduplicating the walk.

**Quality delta:** `−32 LOC, −1 duplicate walk, lower I/O on validate hot path`.

**Net LOC:** `schema.rs` **250 → ~258** (+8 helper); `fusion.rs` **843 → ~825**; net **−32**.

**Done when:** `rg -n 'eq_ignore_ascii_case\("yaml"\)' crates/domain/src/slice/fusion.rs` returns **0**; `cargo test collect_evidence_claim_ids -p specify-domain && cargo test --test slice` pass.

**Rule?** no — two call sites, one loop.

**Counter-argument:** Fusion and schema validation are separate concerns. It loses because the filter/sort logic is byte-identical and the operator path executes it back-to-back.

**Depends on:** none.

### F5 — Table-drive journal wire-shape tests

**Evidence:** Five near-identical append-one-event-read-one-line tests in `crates/domain/src/journal.rs:495-614` (`slice_extract_cache_hit_wire_shape`, `cache_miss`, `fusion_written`, `replay_completed`, `plan_amend_authority_override`). Each is **~15–20 LOC** of tempdir + `append_batch` + substring asserts. Integration coverage for CLI-driven emits lives in `tests/journal.rs` (**482 LOC**, golden fixtures) but does **not** cover cache-hit/miss wire bytes (`rg cache tests/journal.rs` → **0**).

**Action:**
1. Replace the five tests with one table-driven `event_wire_shapes_match_contract` test: `&[(EventKind, &[&str])]` rows asserting required JSON substrings.
2. Keep `append_batch_empty_slice_is_no_op` and `no_snake_case_fields_or_values_leak_to_wire` unchanged.

**Quality delta:** `−65 LOC, −4 test functions, −4 duplicate setup blocks`.

**Net LOC:** `journal.rs` **659 → ~594**.

**Done when:** `rg -c '_wire_shape' crates/domain/src/journal.rs` drops from **5** to **0**; `rg -c 'fn event_wire_shapes' crates/domain/src/journal.rs` returns **≥ 1**; `cargo test -p specify-domain journal::` passes.

**Rule?** no — one module, five copies.

**Counter-argument:** Separate tests give clearer failure names. It loses because the setup is identical and a table row pinpoints the failing event kind in the assertion output.

**Depends on:** none.

### F6 — Trim fusion drift unit tests covered by integration

**Evidence:** `crates/domain/src/slice/fusion.rs:633-788` carries six granular `detect_drift_flags_*` / sort-stability unit tests. End-to-end drift ordering and message shape are already exercised in `tests/slice.rs:622-771` (`validate_passes_on_clean_fusion_inputs`, `validate_flags_missing_req_id_in_fusion`, `validate_flags_contributing_claim_not_found`, …).

**Action:**
1. Delete `detect_drift_flags_missing_fusion_entry`, `detect_drift_flags_extra_fusion_entry`, `detect_drift_flags_contributing_claim_with_no_evidence_row`, `detect_drift_flags_contributing_claim_with_missing_source_file`, `detect_drift_flags_orphan_evidence_claim`, and `detect_drift_findings_sort_byte_stable` from the inline `#[cfg(test)] mod tests`.
2. Keep `round_trips_through_yaml`, `validates_against_embedded_schema`, `load_reports_schema_failure_for_hand_edited_file`, and one representative `detect_drift_clean_inputs_yield_no_findings` smoke.

**Quality delta:** `−95 LOC, −6 redundant test functions, lower maintenance surface in 843-LOC file`.

**Net LOC:** `fusion.rs` **843 → ~748**.

**Done when:** `rg -c 'fn detect_drift_flags_' crates/domain/src/slice/fusion.rs` drops from **6** to **0**; `cargo test -p specify-domain fusion:: && cargo test --test slice fusion` pass.

**Rule?** no — integration tests already pin the operator-visible shape.

**Counter-argument:** Unit tests catch drift logic regressions faster. It loses because `tests/slice.rs` already drives `specify slice validate` through the same drift codes with real on-disk fixtures.

**Depends on:** none.

### F7 — Wire `skill_body.ts` through `walkSkillFiles`

**Evidence:** `walkSkillFiles()` is defined at `scripts/checks/_shared.ts:134-147` ("Used by every skill-body discipline predicate") but never imported. `skill_body.ts` re-declares `PLUGINS_DIR` + `walk(...)` in **seven** exported predicates (`:32`, `:117`, `:190`, `:238`, `:342`, `:419`, `:451`).

```text
rg 'const PLUGINS_DIR = join\(REPO_ROOT, "plugins"\)' scripts/checks/skill_body.ts
→ 7 matches
rg 'walkSkillFiles' scripts/checks/
→ 1 match (_shared.ts definition only)
```

**Action:**
1. Import `walkSkillFiles` from `./_shared.ts` in `skill_body.ts`.
2. Replace each inline `walk(PLUGINS_DIR, { match: [/SKILL\.md$/], …})` loop with `for (const path of await walkSkillFiles())`.
3. Delete the seven redundant `PLUGINS_DIR` declarations and duplicate traversal blocks.

**Quality delta:** `−52 LOC, −6 duplicate walks, lower check-script maintenance`.

**Net LOC:** `skill_body.ts` **524 → ~472**.

**Done when:** `rg 'const PLUGINS_DIR' scripts/checks/skill_body.ts` returns **0**; `make check` passes.

**Rule?** no — dead helper already documented as the canonical walk.

**Counter-argument:** Inline walks are self-contained per predicate. It loses because the traversal rules (symlink skip) must stay identical and `_shared.ts` already owns them.

**Depends on:** none.

## One-Touch Tidies

### T1 — Fix execute skill outcome drift

**Evidence:** [`plugins/spec/skills/execute/SKILL.md:40`](plugins/spec/skills/execute/SKILL.md) says phase skills "reads their phase outcome from `.metadata.yaml`" — contradicts [`phase-outcome-contract.md:3-5`](plugins/spec/references/phase-outcome-contract.md). Critical Path step 4 (`:15`) and § Stop conditions (`:50-56`) restate the same three terminal cases verbatim.

**Action:**
1. Line 40: replace with "reads slice lifecycle from `.metadata.yaml` and phase exit codes; not an on-disk outcome field."
2. Delete § Stop conditions (`:50-58`); CP step 4 already links [`stop-conditions.md`](plugins/spec/skills/execute/references/stop-conditions.md).

**Quality delta:** `−10 LOC, −1 wire-contract defect, −1 duplicate section`.

**Net LOC:** `execute/SKILL.md` **72 → ~62**.

**Done when:** `rg 'phase outcome' plugins/spec/skills/execute/SKILL.md` returns **0**; `make check` passes.

**Rule?** no.

**Counter-argument:** Stop conditions inline aid skimming. It loses because the reference file is the single writer and duplication already drifted once.

**Depends on:** none.

### T2 — Delete ghost outcome-restatement predicate claim

**Evidence:** [`docs/standards/skill-authoring.md:51`](docs/standards/skill-authoring.md) claimed mechanical enforcement for the phase-outcome one-line link rule, but no matching predicate exists under `scripts/` (and this pass does not add one).

**Action:** Delete the false "Mechanically enforced by …" sentence from rule 4; keep the one-line link rule itself.

**Quality delta:** `−1 LOC, −1 doc/implementation drift`.

**Net LOC:** `skill-authoring.md` **154 → 153**.

**Done when:** no repo references to the retired predicate name; `make check` passes.

**Rule?** no.

**Counter-argument:** The predicate should be implemented instead. It loses because this pass forbids new xtask predicates; deleting the false claim is the smallest honest fix.

**Depends on:** F1 (merge brief cleanup makes the ghost claim moot).

### T3 — Delete stale checks.md §2

**Evidence:** [`docs/contributing/checks.md:27-29`](docs/contributing/checks.md) documents "### 2. Stale claims" pointing at `scripts/check.ts`, but `rg 'Stale claims|checkStale' scripts/checks/` returns **0** — no predicate implements it.

**Action:** Delete §2 and renumber subsequent sections.

**Quality delta:** `−4 LOC, −1 doc drift`.

**Net LOC:** `checks.md` **218 → ~214**.

**Done when:** `rg 'Stale claims' docs/contributing/checks.md` returns **0**; `make check` passes.

**Rule?** no.

**Counter-argument:** The check may be planned. It loses because contributors currently believe CI enforces something that does not run.

**Depends on:** none.

### T4 — Collapse validate JSON-schema boilerplate

**Evidence:** Identical serialize → `validate_value` → filter `Fail` → `Error::Validation` pattern:

```text
crates/domain/src/schema.rs:52-67         validate_plan
crates/domain/src/slice/fusion.rs:174-189 FusionIndex::validate
```

**Action:**
1. Add `fn validate_serialisable<T: Serialize>(value: &T, schema: &str, rule_id: &str, rule: &str) -> Result<(), Error>` beside `validate_value` in `schema.rs` (**~10 LOC**).
2. Replace both call-site blocks with one-liners.

**Quality delta:** `−14 LOC net, −1 duplicate branch cluster`.

**Net LOC:** combined **−14**.

**Done when:** `rg -c 'filter\(\|s\| s\.status == ValidationStatus::Fail\)' crates/domain/src/slice/fusion.rs crates/domain/src/schema.rs` drops from **2** to **0**; `cargo test validates_against_embedded_schema -p specify-domain` passes.

**Rule?** no — two sites only.

**Counter-argument:** Inline validation is explicit about which schema failed. It loses because the filter/collect/error path is byte-identical.

**Depends on:** none.

## Post-mortem

- **F1:** actual ΔLOC **−175** vs predicted **−130** (8 files; contracts verifier collateral required for `adapters/targets` grep); done-when flipped cleanly (`rg` → 0, `make check` pass); no regressions.
- **F2:** actual ΔLOC **−31** vs predicted **−34** (+34/−65; amend/create cycle gates added beyond action list after `amend_rejects_cycle` failed); done-when flipped cleanly (`dependency-cycle` gone, orchestrate tests + `cargo make check` pass); no regressions.
- **F3:** actual ΔLOC **−23** vs predicted **−55** (382→359; scan helper structure offset savings); done-when flipped cleanly (removed fns gone, slice tests + `cargo make check` pass); no regressions.
- **F4:** actual ΔLOC **−13** vs predicted **−32** (+32/−45; helper doc/errors block offset savings); done-when flipped cleanly (fusion yaml filter gone, domain + slice tests + `cargo make check` pass); no regressions.
- **F5:** actual ΔLOC **−18** vs predicted **−65** (659→642; table inlines full EventKind payloads); done-when mostly clean (`fn event_wire_shapes` present, journal tests + `cargo make check` pass; `_wire_shape` count stays 1 because new test name contains substring); no regressions.
- **F6:** actual ΔLOC **−170** vs predicted **−95** (843→673; sort-stability fixture was larger than estimated; orphan test never existed); done-when flipped cleanly (`detect_drift_flags_` → 0, fusion + slice tests + `cargo make check` pass); no regressions.
- **F7:** actual ΔLOC **−56** vs predicted **−52** (524→468); done-when flipped cleanly (`PLUGINS_DIR` → 0, `make check` pass); no regressions.
- **T1:** actual ΔLOC **−10** vs predicted **−10**; done-when flipped cleanly (`phase outcome` → 0, `make check` pass); no regressions.
- **T2:** actual ΔLOC **−1** vs predicted **−1** (skill-authoring only; REVIEW.md T2 block reworded so repo-wide grep → 0); done-when flipped cleanly; no regressions.
- **T3:** actual ΔLOC **−5** vs predicted **−4** (218→213; renumber 3–16→2–13); done-when flipped cleanly (`Stale claims` → 0, `make check` pass); no regressions.
- **T4:** actual ΔLOC **−4** code-only vs predicted **−14** (+12 incl. helper doc; explicit serialise_code/label params preserved byte-identical errors); done-when flipped cleanly (filter pattern → 0, schema test + `cargo make check` pass); no regressions.
