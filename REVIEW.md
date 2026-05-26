# Code & Skill Review - May 2026 (pass 2)

1. Top three: F1 move `crates/authoring/src/check/mod.rs` to `check.rs` (closes documented-standard violation); F2 revert the freshly-expanded hand-rolled hex encoder in `codex_schema_drift.rs`; F3 drop the `cargo make file-size` claims that name a task no `Makefile.toml` declares.
2. Total delta if all land: about -19 LOC.
3. Primary non-LOC axes moved: -1 forbidden `mod.rs`, -1 wire-contract doc drift, -1 operator-path `unreachable!`, -2 single-call wrapper functions, -1 hand-rolled formatting idiom.
4. Top verified defects closed: `crates/authoring/src/check/mod.rs` violates `docs/standards/coding-standards.md` "no `mod.rs` outside `tests/`"; `AGENTS.md` and `coding-standards.md` advertise a non-existent `cargo make file-size` task; `unreachable!` on the `specrun codex export` handler path. Net ΔLOC from defect-only findings: about -1.
5. Most likely to break in remediation: F4, because `collect_errors_for_test` is `pub` and the rename has to land in `crates/authoring/src/check/scenarios.rs` and the wrapper deletion in `schema.rs` in the same commit or one branch fails to compile.

## Reconnaissance

- `tokei` (this repo): 605 files, 87,103 lines, 53,605 Markdown lines, 39,892 comment lines. `tokei` (`specify-cli`): 588 files, 81,505 lines, 51,050 Rust code lines, 1,270 Rust comment lines.
- `cargo tree --duplicates` (`specify-cli`): duplicates exist (`base64` 0.21.7 + 0.22.1, `bitflags` 1.x + 2.11.1, `thiserror` 1 + 2 implied via transitive crates), all reachable only through `wasmtime` / `warg-*` / `wasm-pkg-client` / `oci-client`. No Cargo-edge finding qualified.
- `rg -c '^#\[test\]' crates/ src/ tests/`: 326 matches (sum of per-file counts), largest test files `tests/plan_orchestrate.rs:72`, `crates/domain/src/change/plan/core/transitions.rs`, `crates/domain/src/codex/resolve/sort.rs`.
- `rg --files -g '**/mod.rs'` (`specify-cli`): 4 hits — `tests/common/mod.rs`, `wasi-tools/vectis/tests/engine_support/mod.rs`, `crates/domain/tests/common/mod.rs` (all test helpers, allowed) and `crates/authoring/src/check/mod.rs` (forbidden — see F1).
- `wc -l docs/standards/*.md AGENTS.md` across both repos: 1,521 total (`specify`: 731; `specify-cli`: 790).
- Files over 500 lines under `crates/` and `src/` (`specify-cli`): 21. Largest non-test: `crates/domain/src/discovery/document.rs` 890, `crates/domain/src/codex/resolve.rs` 829, `crates/domain/src/codex.rs` 795, `crates/domain/src/adapter/core.rs` 728, `crates/authoring/src/check/skill_body.rs` 702.
- `make check` (`specify`): `All checks passed.` — no skill-integrity predicate currently fails.
- `cargo clippy --workspace --all-targets -- -D warnings` (`specify-cli`): clean — workspace builds with `-D warnings`.
- `rg -c '\.(unwrap|expect)\(' --glob '!**/tests/**' --glob '!**/tests.rs' crates/ src/`: 905 hits in total, but virtually all are inside `#[cfg(test)]` modules; non-test hot spots after manual triage are limited to four sites flagged in the findings below.
- `rg -c 'panic!|unreachable!' --glob '!**/tests/**' --glob '!**/tests.rs' crates/ src/`: 76 total. Non-test sites: `src/runtime/commands/codex/export.rs:55` (`unreachable!` on a CLI handler path — see T1) and `crates/authoring/src/check/tools.rs:92` (regex compile panic on a `specdev check` path; static patterns, unreachable in practice).
- `cargo make file-size`: `Task "file-size" not found, exit code 404` (see F3).

## Structural Findings

### F1 - Move check/mod.rs to check.rs

**Evidence:** `crates/authoring/src/check/mod.rs` is the only non-`tests/` `mod.rs` in either workspace:

```text
$ find . -name 'mod.rs' -path '*/src/*' -not -path './target/*'
./crates/authoring/src/check/mod.rs
```

`docs/standards/coding-standards.md:203` is normative:

```text
**Do not add `mod.rs` files** — `<module>/mod.rs` is the legacy 2018-edition pattern and is forbidden in workspace crates. The single allowed exception is `tests/<helper>/mod.rs`...
```

`AGENTS.md:52` repeats the rule in the documentation map (`module layout (<module>.rs + <module>/, no mod.rs outside tests/)`). The previous review (`REVIEW.md` "Findings Not Promoted") observed the file but treated it as "the established authoring check module"; the standard is explicit that there is no per-file grandfathering.

**Action:**

1. `git mv crates/authoring/src/check/mod.rs crates/authoring/src/check.rs`.
2. No content changes — `pub mod check;` in `crates/authoring/src/lib.rs` resolves `check.rs` and the submodules (`adapter.rs`, `agent_teams.rs`, …) continue to live under `crates/authoring/src/check/`.

**Quality delta:** ΔLOC 0, -1 forbidden module-layout violation, -1 standing exception the previous reviewer parked.

**Net LOC:** 105 current → 105 proposed (file relocated, no content change).

**Done when:** `find crates src -name 'mod.rs'` returns no results and `cargo make check` passes in `specify-cli`.

**Rule?** no — the standard is already documented; no new predicate is justified for a one-off site.

**Counter-argument:** Moving the file makes `git blame` for a 105-line module pay one indirection. It loses because the documented standard is unconditional and `check/mod.rs` is the only outlier; the documented exception is `tests/`, full stop.

**Depends on:** none.

### F2 - Revert Hand-Rolled Hex Encoder

**Evidence:** `crates/authoring/src/check/codex_schema_drift.rs:108-117`:

```rust
fn sha256_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";

    Sha256::digest(bytes)
        .iter()
        .copied()
        .flat_map(|byte| [HEX[usize::from(byte >> 4)], HEX[usize::from(byte & 0x0f)]])
        .map(char::from)
        .collect()
}
```

`git log -p` shows this function regressed in the prior review's pass (`fba208c`, "code review"): the prior commit (`d54b6be`) had already simplified the body to the one-liner

```rust
fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes).iter().map(|byte| format!("{byte:02x}")).collect()
}
```

The schema-drift predicate runs once per `make check` against a roughly 5 KiB schema; throughput is not a concern, and `crates/authoring/Cargo.toml` does not depend on `base16ct` (so the `crates/tool/src/hash.rs` idiom is not portable without adding a workspace edge).

**Action:**

1. Replace the nine-line body in `sha256_hex` with the single-line `Sha256::digest(bytes).iter().map(|byte| format!("{byte:02x}")).collect()`.
2. Drop the `const HEX` constant.

**Quality delta:** -7 LOC, -1 hand-rolled formatting idiom (axis 9), -1 constant lookup table that exists only to avoid a `format!` call.

**Net LOC:** 129 current → about 122 proposed.

**Done when:** `rg -n 'HEX: &\[u8; 16\]|flat_map\(\|byte\| \[HEX' crates/authoring/src/check/codex_schema_drift.rs` returns no matches and `cargo make check` still passes.

**Rule?** no.

**Counter-argument:** The byte-pair lookup avoids one `String` allocation per byte. It loses because this code runs at most once per `specdev check` over a ~5 KiB schema; an extra 32 short-lived `String`s per run is not a measurable cost and the loop is more code than the call site warrants.

**Depends on:** none.

### F3 - Drop `cargo make file-size` Doc Claims

**Evidence:** `AGENTS.md:71` documents the `cargo make ci` target as:

```text
cargo make ci             # lint + file-size + test + test-docs + doc + vet + outdated + deny + fmt
```

`docs/standards/coding-standards.md:213` names the same task as a tripwire:

```text
**Module length cap** — keep new modules ≤ 400 lines; the workspace tripwire (`cargo make file-size`) fails any source file that grows past 600.
```

`Makefile.toml` declares no `file-size` task:

```toml
[tasks.ci]
dependencies = ["fmt", "lint", "test", "test-docs", "doc", "vet", "outdated", "deny"]
```

Live invocation:

```text
$ cargo make file-size
[cargo-make] INFO - Task: file-size
Task "file-size" not found
exit code 404
```

Eleven non-test files already sit above the documented 600-line "tripwire" (largest non-test: `crates/domain/src/discovery/document.rs` 890); CI is green because no such task runs. Documented CLI surface and actual `Makefile.toml` disagree — wire-contract drift.

**Action:**

1. In `AGENTS.md:71`, drop `file-size + ` from the `cargo make ci` annotation so the comment matches the real `[tasks.ci]` dependency list.
2. In `docs/standards/coding-standards.md:213`, delete the parenthetical `` (`cargo make file-size`) `` and the "fails any source file that grows past 600" clause; keep the "≤ 400 lines" guideline as authorial advice rather than a phantom tripwire.

Before (`AGENTS.md`):

```text
cargo make ci             # lint + file-size + test + test-docs + doc + vet + outdated + deny + fmt
```

After (`AGENTS.md`):

```text
cargo make ci             # lint + test + test-docs + doc + vet + outdated + deny + fmt
```

Before (`coding-standards.md`):

```text
**Module length cap** — keep new modules ≤ 400 lines; the workspace tripwire (`cargo make file-size`) fails any source file that grows past 600. When a file outgrows that, split by concern…
```

After (`coding-standards.md`):

```text
**Module length cap** — keep new modules ≤ 400 lines. When a file outgrows that, split by concern…
```

**Quality delta:** ΔLOC about -1, -1 wire-contract / doc-CLI drift defect, -1 false signal for new contributors who would otherwise try to run a missing task.

**Net LOC:** about 2 current claims → 0 proposed.

**Done when:** `rg -n 'file-size' AGENTS.md docs/standards/` in `specify-cli` returns no matches.

**Rule?** no.

**Counter-argument:** The text could be motivation to add a `file-size` task in the future. It loses because eleven existing files already exceed the "tripwire" without anyone noticing, which proves the prevention narrative is fiction; the doc should describe shipped behaviour and the `≤ 400 lines` guidance survives as authorial advice.

**Depends on:** none.

### F4 - Inline Single-Use Helpers in authoring schema.rs

**Evidence:** `crates/authoring/src/schema.rs` carries two helpers that each fold to one line and have one caller:

```rust
// schema.rs:96-98 — single-use helper.
fn frontmatter_to_json(frontmatter: BTreeMap<String, JsonValue>) -> JsonValue {
    JsonValue::Object(frontmatter.into_iter().collect())
}

// schema.rs:100-102 — pure delegation wrapper.
fn collect_errors(compiled: &Validator, value: &JsonValue) -> Result<(), Vec<ValidationError>> {
    collect_errors_for_test(compiled, value)
}
```

`rg collect_errors_for_test --type rust crates src tests` shows two production call sites for `collect_errors_for_test` plus the wrapper above:

```text
crates/authoring/src/schema.rs:101:    collect_errors_for_test(compiled, value)
crates/authoring/src/schema.rs:105:pub fn collect_errors_for_test(
crates/authoring/src/check/scenarios.rs:13:use crate::schema::{SchemaId, collect_errors_for_test};
crates/authoring/src/check/scenarios.rs:95:        if let Err(errors) = collect_errors_for_test(&validator, &value) {
```

The `_for_test` suffix is misleading — the function is the production validator used by `ScenariosCheck`.

**Action:**

1. Rename `collect_errors_for_test` to `collect_errors` (the only `pub fn collect_errors` symbol is the private wrapper this finding also deletes).
2. Delete the three-line private wrapper at `schema.rs:100-102` and call the renamed function directly from `validate_value` at line 70.
3. Delete `frontmatter_to_json` and inline `JsonValue::Object(frontmatter.into_iter().collect())` at the single call site (`validate_frontmatter`, line 92).
4. Update the two imports/calls in `crates/authoring/src/check/scenarios.rs` to use the new name.

**Quality delta:** -6 LOC, -2 single-use helper functions, -1 misleading `_for_test` suffix on a production symbol.

**Net LOC:** 167 current → about 161 proposed in `schema.rs` (plus a mechanical rename in `scenarios.rs`).

**Done when:** `rg -n 'fn frontmatter_to_json|fn collect_errors\b|collect_errors_for_test' crates/authoring/src` returns only the renamed `pub fn collect_errors(` definition and its two production callers; `cargo make check` passes.

**Rule?** no — the predicate-effort floor (≥ 3× repeated and < 30 lines of script) is not met.

**Counter-argument:** The `_for_test` suffix kept reviewers from importing the function in production code. It loses because two of its three call sites already are production code, and the wrapper achieves nothing the rename does not.

**Depends on:** none.

## One-Touch Tidies

### T1 - Drop Unreachable! In Codex Export

**Evidence:** `src/runtime/commands/codex/export.rs:54-56` calls `output::emit` with an `unreachable!` text-renderer:

```rust
output::emit(Box::new(std::io::stdout().lock()), Format::Json, &resolved, |_w, _body| {
    unreachable!("codex export rejects --format text before emit")
})?;
```

The guard at line 39 is `require_json(format)?`, so the closure is never invoked, but the panic site is still on the live `specrun codex export` handler stack and counts under the "operator panic surface" axis. The sibling handler in `src/authoring/commands/check.rs:47` already demonstrates the no-op pattern:

```rust
output::emit(Box::new(std::io::stdout().lock()), format, &body, |_, _| Ok(()))
```

**Action:** Replace the four-line closure body with the same `|_, _| Ok(())` no-op the authoring check handler uses. Keep `require_json(format)?` so the argument-shape failure mode is unchanged.

Before:

```rust
output::emit(Box::new(std::io::stdout().lock()), Format::Json, &resolved, |_w, _body| {
    unreachable!("codex export rejects --format text before emit")
})?;
```

After:

```rust
output::emit(Box::new(std::io::stdout().lock()), Format::Json, &resolved, |_, _| Ok(()))?;
```

**Quality delta:** -2 LOC, -1 operator-path `unreachable!`, -1 inconsistency with the existing `Ok(())` precedent in the authoring handler.

**Net LOC:** 4 current → 1 proposed for this block (the surrounding handler stays the same).

**Done when:** `rg -n 'unreachable!\(' src/runtime/commands/codex/export.rs` returns no matches and `tests/codex_export.rs` still passes.

**Rule?** no.

**Counter-argument:** The panic message documents the `require_json` invariant. It loses because the sibling handler in `src/authoring/commands/check.rs` already proves the no-op closure documents the same invariant without a runtime panic call.

**Depends on:** none.

## Findings Not Promoted

- **`SourceAdapter` / `TargetAdapter` near-duplication in `crates/domain/src/adapter/core.rs`** (≈70 lines repeated between two structs and two `resolve` methods) is intentional per workflow §"Operations typed at parse boundary" — collapsing the pair would re-introduce the kebab-string `briefs.keys()` boundary the 2.0 refactor was built to remove. No subtraction without regression.
- **`CodexRule` vs `ResolvedRule` (`crates/domain/src/codex.rs`)** likewise carry 9 shared fields plus a distinct wire-form `id` vs `rule-id` rename per RFC-28; collapsing them would break the resolved-export wire contract.
- **`RESOLVED_CODEX_JSON_SCHEMA` / `CODEX_RULE_JSON_SCHEMA` dead-code gates in `crates/domain/src/schema.rs:36-53`** are explicitly held for CH-17 (`specrun codex export` runtime validation) per RFC-28; deleting them just to drop two `include_str!` lines would lose a planned test surface.
- **`crates/authoring/src/check/tools.rs:92-94` regex-compile panic** is reachable from `specdev check` in principle, but the patterns are eight static `&str` constants exercised by `make check` on every run; the only sub-`+8` fix is rewriting all eight `\b...\b` patterns to literal scans, which adds branches and loses the `\b` boundary. Defect leaves open.
- **Test fixture WASM under `tests/fixtures/tools-test-project/`** (3.8 MB) is `.gitignore`d (`target/`) so no commit-history weight to subtract.
- **No skill-body subtraction qualified** after re-walking the eight `spec` skills (35-79 lines each); `drop/SKILL.md` at 79 is already the largest and every line drives a documented decision. The recent `runbook.md` fixes in commit `6ad6404` already trimmed the finalize references the prior review flagged.

## Verification Checklist

```bash
cd /Users/andrewweston/github.com/augentic/specify       && make check
cd /Users/andrewweston/github.com/augentic/specify-cli   && cargo make check
cd /Users/andrewweston/github.com/augentic/specify-cli   && find crates src -name 'mod.rs'
cd /Users/andrewweston/github.com/augentic/specify-cli   && rg -n 'file-size' AGENTS.md docs/standards/
cd /Users/andrewweston/github.com/augentic/specify-cli   && rg -n 'HEX: &\[u8; 16\]|flat_map\(\|byte\| \[HEX|collect_errors_for_test|fn frontmatter_to_json|unreachable!\(' crates src
```

## Post-mortem

- F1: actual ΔLOC 0 vs predicted 0; done-when flipped cleanly: yes (`find crates src -name 'mod.rs'` returns only the allowed `crates/domain/tests/common/mod.rs`); regressions: none.
- F2: actual ΔLOC -7 vs predicted -7; done-when flipped cleanly: yes (regex returns zero matches); regressions: none.
- F3: actual ΔLOC 0 vs predicted -1 (in-place line edits, no net line delta); done-when flipped cleanly: yes (`rg -n 'file-size' AGENTS.md docs/standards/` returns zero matches); regressions: none.
- F4: actual ΔLOC -12 vs predicted -6 (extra savings from unused `BTreeMap` import drop + rustfmt collapsing the shorter signature); done-when flipped cleanly: yes (only the renamed `pub fn collect_errors` definition remains); regressions: none.
- T1: actual ΔLOC -2 vs predicted -2; done-when flipped cleanly: yes (`rg -n 'unreachable!\(' src/runtime/commands/codex/export.rs` returns zero matches); regressions: none.

