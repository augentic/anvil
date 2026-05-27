# Code & Skill Review — May 2026 (pass 3)

1. Top three by tier: **F1** repair RFC-32 archive links (11 `links.unresolved` CI failures); **F2** table-drive `review/run.rs` error-mapping tests (−118 LOC); **F4** drop stale “planned” labels for shipped `specrun review` / `specrun codex export` (doc ↔ DECISIONS drift).
2. Total ΔLOC if all land: about **−181**.
3. Primary non-LOC axes moved: **−1 defect surface** (11 → 0 link failures), **−1 wire-contract doc drift**, **−2 duplicate test suites**, **−1 single-call wrapper**.
4. Top verified defects closed: **F1** (11 predicate failures), **F4** (operator docs contradict shipped CLI). Net ΔLOC from defect-only findings alone: about **−5** (≤ +30 cap). Still open: none that qualify under the pass rules beyond F1/F4.
5. Most likely to break in remediation: **F2** — collapsing ten `map_*_error` tests into one table requires distinct `HintError`/`IndexError`/`RenderError` constructors per row; a missing variant silently drops §D8 coverage.

## Reconnaissance

- `tokei` (`specify-cli`): 631 files, 87,727 lines; Rust 353 files / 62,836 lines (54,970 code). `tokei` (`specify`): 609 files, 88,055 lines; Markdown 505 files / 54,470 lines.
- `cargo tree --duplicates` (`specify-cli`): duplicates (`base64` 0.21.7 + 0.22.1, `reqwest` 0.12 + 0.13, …) all transitive via `wasmtime` / `warg-*` / `wasm-pkg-client`. No direct-edge finding qualified (Cargo.toml frozen).
- `rg -c '^#\[test\]' crates/ src/ tests/` (`specify-cli`): **601** tests (sum of per-file counts).
- `rg --files -g '**/mod.rs'` (`specify-cli`): **5** hits — all under `tests/` or `wasi-tools/vectis/tests/` (allowed). No forbidden `src/**/mod.rs`.
- `wc -l docs/standards/*.md AGENTS.md`: **731** (`specify`) + **803** (`specify-cli`) = **1,534** total.
- Files > 500 lines under `crates/` + `src/` (`specify-cli`): **21** (largest non-test: `discovery/document.rs` 890, `codex/resolve.rs` 829, `codex.rs` 795, `adapter/core.rs` 728, `check/skill_body.rs` 702, `review/run.rs` 678).
- `make check` (`specify`): **FAIL — 11 failures**, all `links.unresolved` (first predicate id: `links.unresolved`). Stale target: `rfcs/rfc-32-standards-enforcement.md` after move to `rfcs/done/`.
- `cargo make check` (`specify-cli`): **pass** (fmt + clippy `-D warnings` + nextest + test-docs, 183.9s).
- `rg -c '\.(unwrap|expect)\(' --glob '!**/tests/**' crates/ src/` (`specify-cli`): **904** (overwhelmingly `#[cfg(test)]` modules co-located in source files; non-test hot paths are sparse).
- `rg -c 'panic!|unreachable!' --glob '!**/tests/**' crates/ src/` (`specify-cli`): **87** (includes `#[cfg(test)]` arms in `codex/export.rs`, static-regex `OnceLock` init in `check/tools.rs`, and table-driven test `panic!` guards — no operator-path `unreachable!` on current `main`).

## Structural Findings

### F1 — Repair RFC-32 archive links

**Evidence:** In-progress move leaves deleted paths referenced. `git status` shows `D rfcs/rfc-32-standards-enforcement.md`, `?? rfcs/done/rfc-32-standards-enforcement.md`. `make check`:

```text
FAIL: links.unresolved: Broken link in AGENTS.md: rfcs/rfc-32-standards-enforcement.md
FAIL: links.unresolved: Broken link in adapters/shared/codex/universal/README.md: ../../../../rfcs/rfc-32-standards-enforcement.md
FAIL: links.unresolved: Broken link in adapters/targets/omnia/briefs/build.md: ../../../../rfcs/rfc-32-standards-enforcement.md#principles
FAIL: links.unresolved: Broken link in adapters/targets/vectis/briefs/build.md: ../../../../rfcs/rfc-32-standards-enforcement.md#principles
FAIL: links.unresolved: Broken link in docs/contributing/checks.md: ../../rfcs/rfc-32-standards-enforcement.md (×3)
FAIL: links.unresolved: Broken link in rfcs/fixtures/rfc-32-seed/README.md: ../../rfc-32-plan.md
FAIL: links.unresolved: Broken link in rfcs/roadmap.md: rfc-32-standards-enforcement.md (×3)
11 check failure(s).
```

**Action:**

1. Stage the move: `git add rfcs/done/rfc-32-standards-enforcement.md` and stage deletions of `rfcs/rfc-32-standards-enforcement.md` and `rfcs/rfc-32-plan.md`.
2. Bulk-rewrite relative targets — insert `done/` before `rfc-32-standards-enforcement.md` in every checked path above (preserve `#anchors`).
3. In `rfcs/fixtures/rfc-32-seed/README.md:5`, replace `../../rfc-32-plan.md` with `../../done/rfc-32-standards-enforcement.md`.
4. Optionally sweep unchecked `rfcs/rfc-*.md` prose (same stale segment) while the branch is open; link predicate skips most `rfcs/rfc-*` sources but operators still follow those links.

**Quality delta:** ΔLOC ~0, −11 defect surface, −1 broken-archive invariant.

**Net LOC:** link strings ~same length (`done/` offset).

**Done when:** `make check` prints `All checks passed.` (0 failures; was 11).

**Rule?** no — one-off archive move.

**Counter-argument:** Leave RFC-32 at `rfcs/` root for shorter URLs. Loses because the file is already under `rfcs/done/` and CI fails today.

**Depends on:** none.

### F2 — Table-drive review error-map tests

**Evidence:** `src/runtime/commands/review/run.rs:481–660` — ten copy-pasted tests that each construct one error, call `map_index_error` / `map_hint_error` / `map_render_error`, and assert a single `rule_id` or variant. Same file already documents the §D8 table in prose at `:258–371`. Module is **678** lines; test block alone is **~220** lines.

**Action:**

1. Replace individual `#[test] fn …_maps_to_…` functions with one `#[test] fn error_mapping_matches_d8_table()` driven by a `const CASES: &[Case]` slice.
2. Each row: input enum variant (or factory), mapper fn pointer, expected `Error` discriminant + `rule_id`/`code`.
3. Keep the two `parse_slice_tasks_paths` tests separate (different surface).

Before (representative):

```rust
#[test]
fn unsupported_scan_profile_maps_to_validation_exit_2() {
    let err = map_index_error(IndexError::UnsupportedScanProfile(ScanProfile::Framework));
    match err {
        Error::Validation { results } => {
            assert_eq!(results[0].rule_id, "review-unsupported-scan-profile");
        }
        other => panic!("expected Validation, got {other:?}"),
    }
}
```

After (sketch):

```rust
#[test]
fn error_mapping_matches_d8_table() {
    for case in INDEX_CASES.iter().chain(HINT_CASES).chain(RENDER_CASES) {
        let err = (case.map)(case.input());
        assert_eq!(case.rule_id(&err), case.expected_rule_id, "{}", case.label);
    }
}
```

**Quality delta:** −118 LOC, −9 duplicate test functions, −1 maintenance branch per new §D8 row.

**Net LOC:** 678 → ~560 in `review/run.rs`.

**Done when:** `wc -l src/runtime/commands/review/run.rs` ≤ 565 and `cargo make check` passes.

**Rule?** no.

**Counter-argument:** Separate tests give clearer failure names in CI. Loses because the table is closed and one row per variant preserves names via `case.label`; ripgrep/cargo use the same pattern for exit-code tables.

**Depends on:** none.

### F3 — Delete redundant sort unit tests

**Evidence:** `crates/codex/src/rules/resolve/sort.rs:171–203` — `sort_orders_by_severity` and `sort_orders_by_origin` re-prove enum ordering already pinned in `crates/codex/src/rules.rs:544–569` (`severity_ordering_matches_rfc`, `origin_ordering_matches_rfc`). Integration test `build_resolved_codex_emits_versioned_envelope` (`sort.rs:281–312`) already asserts final rule-id order on the wire envelope.

**Action:** Delete `sort_orders_by_severity` and `sort_orders_by_origin` tests only. Keep tests 3–5 (deprecated flag, rule-id tie-break, full tuple) and integration tests 6–8.

**Quality delta:** −35 LOC, −2 duplicate test functions.

**Net LOC:** 396 → ~361 in `sort.rs` test module.

**Done when:** `rg -c 'fn sort_orders_by_(severity|origin)' crates/codex/src/rules/resolve/sort.rs` → 0; `cargo make check` passes.

**Rule?** no.

**Counter-argument:** Unit tests isolate `sort_resolved` from `build_resolved_codex`. Loses because `sort_resolved` is a five-line `sort_by` on enum keys already covered by derived `Ord` tests plus envelope integration.

**Depends on:** none.

### F4 — Drop “planned” for shipped review

**Evidence:** DECISIONS.md documents shipped `specrun review` (`specify-cli/DECISIONS.md:727,755`). Binary exists (`src/runtime/commands/review/run.rs`, `tests/review_run.rs`). Operator docs still say planned:

- `AGENTS.md:45` — `` `specrun review` (planned, RM-10) ``
- `docs/explanation/standards-layer.md:11,24,35` — “future `specrun codex export` and `specrun review`”, “`(planned)`”
- `docs/contributing/checks.md:33` — “`(planned)`”

**Action:** Delete `(planned, RM-10)` / `(planned)` / “future” qualifiers for commands that ship today. Keep RM-10 roadmap pointer only where it names CI wiring, not binary existence. One sentence per file; no new doc files.

Example — `AGENTS.md:45`:

```markdown
`specrun review` is CI-native **standards enforcement**, not a workflow phase
```

**Quality delta:** −5 LOC, −1 wire-contract doc drift, −1 operator confusion axis.

**Net LOC:** ~731 → ~726 in touched `specify` docs.

**Done when:** `rg 'specrun review.*planned|review.*\(planned\)' AGENTS.md docs/` → 0 matches; `make check` still passes after F1.

**Rule?** no.

**Counter-argument:** RM-10 still tracks CI adoption, not CLI presence. Loses because “planned” describes the binary, not consumer rollout; DECISIONS already draws that boundary.

**Depends on:** none.

## One-touch tidies

### T1 — Inline `to_file_fact`

**Evidence:** `crates/codex/src/review/index/files.rs:172–179` — eight-line wrapper with a single call site at `crates/codex/src/review/index.rs:83`.

**Action:** Delete `to_file_fact`; inline struct literal inside the `par_iter().map` closure.

**Quality delta:** −8 LOC, −1 function, −1 module edge.

**Net LOC:** 179 → 171 in `files.rs`; 229 → 237 in `index.rs` (net −8).

**Done when:** `rg 'to_file_fact' crates/codex` → 0; `cargo make check` passes.

**Rule?** no.

**Counter-argument:** Named conversion documents intent. Loses at one call site — field mapping is self-evident.

**Depends on:** none.

### T2 — Drop duplicate severity smoke tests

**Evidence:** `src/authoring/severity.rs:98–108` — individual tests for `codex.namespace-ownership-violation` and `codex.duplicate-rule-id` duplicate coverage in `every_exported_rule_constant_maps_to_a_known_severity` (`:169–201`).

**Action:** Delete the two standalone `#[test]` functions; keep `codex_schema_violation_maps_to_critical` and the aggregate test.

**Quality delta:** −15 LOC, −2 test functions.

**Net LOC:** 204 → ~189 in `severity.rs`.

**Done when:** `rg 'codex_namespace_ownership|codex_duplicate_rule_id' src/authoring/severity.rs` → 0; `cargo make check` passes.

**Rule?** no.

**Counter-argument:** Named tests read better in failure output. Loses because aggregate test already names the constant on failure.

**Depends on:** none.

### T3 — Use shared markdown walk in links check

**Evidence:** `crates/authoring/src/check/links.rs:214–232` duplicates walkdir + `.md` filter logic from `crates/authoring/src/helpers.rs:61–123` (~18 lines). Helpers version already applies `under_symlink`.

**Action:** Delete local `walk_markdown_files`; import `crate::helpers::walk_markdown_files`; call `walk_markdown_files(root, root).unwrap_or_default()` at lines 46 and 148. Drop unused `walkdir::WalkDir` import.

**Quality delta:** −18 LOC, −1 duplicate walk, −1 import edge.

**Net LOC:** 331 → ~313 in `links.rs`.

**Done when:** `rg 'fn walk_markdown_files' crates/authoring/src/check/links.rs` → 0; `make check` passes.

**Rule?** no.

**Counter-argument:** Local version uses `under_symlink(…).unwrap_or(true)` (skip file on metadata error) vs helpers `?` (fail whole walk). Loses if symlink metadata errors are common; in practice they are exceptional and failing closed is preferable to silently skipping link checks.

**Depends on:** none.

### T4 — Delete `resolve_and_filter` one-liner re-export path

**Evidence:** `crates/codex/src/rules/resolve/filter.rs:110–115`:

```rust
pub fn resolve_and_filter(inputs: &ResolveInputs<'_>) -> Result<Vec<ResolvedRuleEntry>, ResolveError> {
    let entries = super::resolve(inputs)?;
    Ok(filter(entries, inputs))
}
```

Single external caller: `sort.rs:69`. `filter.rs` tests call `filter` directly.

**Action:** Inline at `sort.rs:69`:

```rust
let mut entries = filter(super::resolve(inputs)?, inputs);
```

Remove `resolve_and_filter` fn and `pub use` from `resolve.rs:64`. Update `filter.rs` module docs to point callers at the two-step compose.

**Quality delta:** −6 LOC, −1 public fn, −1 indirection.

**Net LOC:** filter.rs 606 → ~600; sort.rs unchanged net.

**Done when:** `rg 'resolve_and_filter' crates/codex` → 0; `cargo make check` passes.

**Rule?** no.

**Counter-argument:** Named compose documents CH-12+13 pipeline. Loses at one call site — `build_resolved_codex` name already documents the pipeline entry.

**Depends on:** none.

## Findings not promoted

| Candidate | Reason dropped |
| --- | --- |
| Merge `compact.rs` / `github.rs` render loops | Different §D6 wire shapes; shared loop adds abstraction without ≥30 LOC deletion. |
| Collapse `ReviewResultVersion` / `WorkspaceModelVersion` | Would need a new generic or macro (+LOC / +type). |
| Deduplicate `EVIDENCE_MAX_BYTES` in `map_finding.rs` | `finding::EVIDENCE_MAX_BYTES` is private; exporting it adds API surface. |
| `cargo tree --duplicates` consolidation | Transitive only; no workspace `Cargo.toml` change allowed. |
| Skill body / fixture edits in working trees | No `make check` skill predicate failure on current tree beyond links. |
| Previous pass F1 (`check/mod.rs` → `check.rs`) | Already landed — `crates/authoring/src/check.rs` exists, no forbidden `mod.rs`. |
| Previous pass F3 (`cargo make file-size`) | No `file-size` references remain in `specify-cli` docs. |

## Post-mortem

- **F1:** actual ΔLOC ~0 link edits (±0 net path strings); done-when flipped cleanly (`make check` 11 → 0 failures); no regressions; optional sweep caught 5 extra stale refs (`decision-log.md`, `rfc-5-tooling.md`, `rfc-18-slm.md`, `rfc-33`, `rfc-34`).
- **F2:** actual ΔLOC −221 in `run.rs` (457 vs predicted ~560; tests extracted to `run_tests.rs` +176); done-when flipped cleanly (457 ≤ 565, `cargo make check` pass); no regressions; +1 §D8 row (`JsonSerialise`) vs 11 original tests.
- **F3:** actual ΔLOC −39 in `sort.rs` (357 vs predicted ~361); done-when flipped cleanly (`rg` 0, `cargo make check` pass); no regressions; first local `cargo make check` hit corrupted `target/` (subagent rebuilt clean).
- **F4:** actual ΔLOC ~0 net (10/10 replace, vs predicted −5); done-when flipped cleanly (`rg` 0, `make check` pass); no regressions; `omnia/briefs/build/review.md` still has `(planned RM-10)` outside scoped paths.
- **T1:** actual ΔLOC −6 net (vs predicted −8); done-when flipped cleanly (`rg` 0, `cargo make check` pass); no regressions.
- **T2:** actual ΔLOC −14 (vs predicted −15); done-when flipped cleanly (`rg` 0, severity 5/5 pass); no regressions; full `cargo make check` blocked transiently by corrupted `target/` (T4 fixed unrelated clippy).
- **T3:** actual ΔLOC −20 in `links.rs` (vs predicted −18); done-when flipped cleanly (`rg` 0, `make check` pass); no regressions.
- **T4:** actual ΔLOC −11 task-only (vs predicted −6); done-when flipped cleanly (`rg` 0, `cargo make check` pass); no regressions; removed unused `ResolveError` import left by inline.

**Final validation:** `make check` (specify) pass; `cargo make check` + `cargo make ci` (specify-cli) pass after sequential `rm -rf target` (parallel subagent builds had raced on shared `target/`).
