# Code & Skill Review — specify + specify-cli

Top three findings by tier: **F1 Share plan test fixtures across doctor/core** (~−52 LOC), **F2 Replace validate/tests Plan+Entry boilerplate** (~−75 LOC), **F3 Collapse authority-override journal event triple** (~−22 LOC).
Total ΔLOC if all land: **approximately −195 LOC**.
Primary non-LOC axes moved: fewer duplicate test DTO literals, fewer hand-rolled helper fns, fewer branch clusters, lower call-site ceremony in tests.
Top verified defects closed: **none qualified** (0 open from this pass); defect-only net ΔLOC: **0** (portfolio cap unused).
Most likely to break in remediation: **F4** — `emit_override_events` sort-key tuple and set-then-clear dedup logic must stay byte-identical to the batched journal append tests.

## Reconnaissance

- `tokei`:
  - `specify`: **648 files**, **87,122 total lines**; Markdown **515 files / 49,552 lines**.
  - `specify-cli`: **446 files**, **64,600 total lines**; Rust **245 files / 47,549 lines**.
- `cargo tree --duplicates` (`specify-cli`): non-empty. `base64 v0.21.7 / v0.22.1`, `reqwest v0.12.28 / v0.13.3`, `bitflags v2.11.1` against `rustix v0.38.44 / v1.1.4`, plus wider `wasmtime` / `wasm-pkg-client` transitive families. `Cargo.toml` frozen for this pass.
- `rg -c '^#\[test\]' crates/ src/ tests/` (`specify-cli`): **512** test functions.
- `rg --files -g '**/mod.rs'` (`specify-cli`): **3** files — `crates/domain/tests/common/mod.rs`, `tests/common/mod.rs`, `wasi-tools/vectis/tests/engine_support/mod.rs`.
- `wc -l docs/standards/*.md AGENTS.md`:
  - `specify`: **555 total**.
  - `specify-cli`: **638 total** (`DECISIONS.md` adds **624** on top).
- Files >500 lines under `crates/` and `src/` (`specify-cli`):
  - Tests: `crates/domain/tests/workspace.rs` **1048**, `crates/domain/tests/finalize.rs` **947**, `crates/domain/tests/registry.rs` **922**.
  - Source: `src/commands/plan/create.rs` **918**, `crates/domain/src/discovery/document.rs` **891**, `crates/domain/src/slice/fusion.rs` **839**, `crates/domain/src/adapter/core.rs` **709**, `crates/domain/src/change/plan/core/model.rs` **629**, `crates/domain/src/spec/provenance.rs` **607**, `crates/domain/src/journal.rs` **595**, `crates/tool/src/validate.rs` **520**, `crates/domain/src/adapter/cache/io.rs` **509**.
- `make checks` (`specify`): **passed** — `All checks passed.` Total failures: **0**; first five predicate ids: **none**.
- `cargo make check` (`specify-cli`): **passed** — `Build Done in 165.71 seconds.` First error: **none**.
- `rg -c '\.(unwrap|expect)\(' --glob '!**/tests/**' crates/ src/` (`specify-cli`): summed **695** matching lines (includes `#[cfg(test)]` modules co-located in production files; operator-path count is materially smaller).
- `rg -c 'panic!|unreachable!' --glob '!**/tests/**' crates/ src/` (`specify-cli`): summed **48** matching lines.

## Structural Findings

### F1 — Share plan test fixtures with doctor

**Evidence:** `crates/domain/src/change/plan/doctor/tests.rs:13-41` re-declares `change`, `change_with_deps`, and `plan_with` — byte-for-byte mirrors of `crates/domain/src/change/plan/core/test_support.rs:77-114`. `change_with_deps` in test_support duplicates the full `Entry { … }` literal instead of delegating to `change` (doctor's copy already delegates at `:28-31`). Recon: doctor/tests.rs **458** lines; duplicated block **29** lines.

```text
crates/domain/src/change/plan/doctor/tests.rs:13:fn change(name: &str, status: Status) -> Entry {
crates/domain/src/change/plan/doctor/tests.rs:34:fn plan_with(changes: Vec<Entry>) -> Plan {
crates/domain/src/change/plan/core/test_support.rs:86:pub(super) fn change(name: &str, status: Status) -> Entry {
crates/domain/src/change/plan/core/test_support.rs:101:pub(super) fn change_with_deps(name: &str, status: Status, deps: &[&str]) -> Entry {
```

**Action:**
1. In `crates/domain/src/change/plan/core.rs`, after `mod test_support;`, add `#[cfg(test)] pub(crate) use test_support::{change, change_with_deps, plan_with_changes};`.
2. Slim `change_with_deps` in `test_support.rs` to four lines: `let mut e = change(name, status); e.depends_on = deps.iter().map(|s| (*s).to_string()).collect(); e`.
3. Delete `change`, `change_with_deps`, and `plan_with` from `doctor/tests.rs`; import `use crate::change::plan::core::{change, change_with_deps, plan_with_changes};` and rename call sites from `plan_with` → `plan_with_changes`. Keep `plan_with_sources` (doctor-only).

**Quality delta:** `−52 LOC, −3 duplicate fns, −2 duplicate Entry-literal sites, −1 module edge (shared fixture home)`.

**Net LOC:** `doctor/tests.rs` + `test_support.rs` + `core.rs` **~573 → ~521**.

**Done when:** `rg -n 'fn change\(|fn plan_with\(|fn change_with_deps' crates/domain/src/change/plan/doctor/tests.rs` returns **0**, `rg -n 'pub\(crate\) use test_support' crates/domain/src/change/plan/core.rs` returns **≥ 1**, and `cargo make check` passes.

**Rule?** no — one duplicated fixture island, not a repo-wide pattern.

**Counter-argument:** Doctor tests stay self-contained without reaching into `core/`. It loses because the fixtures are already maintained in `test_support` for six other `core/` test modules; a third copy will drift on the next `Entry` field addition.

**Depends on:** none.

### F2 — Replace validate/tests Plan+Entry boilerplate

**Evidence:** `crates/domain/src/change/plan/core/validate/tests.rs` already imports `change` and `plan_with_changes` (`:9`) but **eight** registry / project tests still hand-build full `Plan { … entries: vec![Entry { … }] }` blocks (`:179-195`, `:212-228`, `:254-270`, `:299-315`, `:337-353`, `:379-395`, `:407-423`, `:433-449`). Each block is **14–18 lines** where `let mut e = change("a", Status::Pending); e.project = …; let plan = plan_with_changes(vec![e]);` is **4 lines**.

**Action:**
1. For each of the eight tests, replace the manual `Plan` + `Entry` literal with `change` / `plan_with_changes`, mutating only the fields under test (`project`, `target`, `sources`, etc.).
2. Do not touch tests that already mutate `change()` outputs (e.g. `:35-41` cycle fixture) or need `RFC_EXAMPLE_YAML`.

**Quality delta:** `−75 LOC, −8 duplicate Entry literals, lower call-site burden in tests`.

**Net LOC:** `validate/tests.rs` **595 → ~520**.

**Done when:** `rg -c 'entries: vec!\[Entry \{' crates/domain/src/change/plan/core/validate/tests.rs` drops from **8** to **0**, and `cargo make check` passes.

**Rule?** no.

**Counter-argument:** Explicit struct literals make each test's inputs obvious at a glance. It loses because `change()` defaults are stable and field overrides are one line; the 14-line blocks obscure the one field each test cares about.

**Depends on:** F1 (optional — validate/tests already sees `change` via `super::super::test_support`; no hard dependency).

### F3 — Use fixtures in plan io tests

**Evidence:** `crates/domain/src/change/plan/core/io.rs:98-99` imports only `RFC_EXAMPLE_YAML`; three tests still spell full `Entry { … }` literals inside `Plan { … }` (`:144-155`, `:201-213`, `:252-264`) plus two empty-plan literals (`:115-120`, `:240-245`) that match `plan_with_changes(vec![])`.

**Action:**
1. Extend the test import to `use super::super::test_support::{RFC_EXAMPLE_YAML, change, plan_with_changes};`.
2. Replace `entries: vec![Entry { … }]` with `plan_with_changes(vec![change("only-entry", Status::Pending)])` (adjust name/status per test).
3. Replace empty `Plan { name: "init", … entries: vec![] }` with `plan_with_changes(vec![])` and mutate `.name` when the test asserts on disk content.

**Quality delta:** `−40 LOC, −5 duplicate Entry literals, −2 duplicate Plan shells`.

**Net LOC:** `io.rs` **275 → ~235**.

**Done when:** `rg -c 'Entry \{' crates/domain/src/change/plan/core/io.rs` drops from **3** to **0** (inside `mod tests`), and `cargo make check` passes.

**Rule?** no.

**Counter-argument:** io tests are self-describing integration fixtures. It loses because the literals carry no extra signal — every field except `name`/`status` is default.

**Depends on:** none.

### F4 — Collapse authority-override journal triple

**Evidence:** `src/commands/plan/create.rs:299-371` — `emit_override_events` repeats the same `journal::Event::new(… PlanAmendAuthorityOverride { … })` shape three times (set loop `:311-323`, per-kind clear `:333-345`, clear-all `:351-363`). Only `action`, `claim_kind`, and `source_key` vary.

**Action:**
1. Inside `emit_override_events`, add a closure (not a free function) `let mut record = |slice, action, claim_kind, source_key| { … }` that pushes `(sort_key, journal::Event::new(…))` onto `pending`.
2. Replace the three `journal::Event::new` blocks with `record(...)` calls; keep the existing continue/skip guards in each loop unchanged.

**Quality delta:** `−22 LOC, −2 branch clusters (three struct-literal copies → one closure), lower call-site burden`.

**Net LOC:** `create.rs` **918 → ~896**.

**Done when:** `rg -c 'PlanAmendAuthorityOverride' src/commands/plan/create.rs` stays **3** (one closure body + two loop call patterns), `rg -c 'journal::Event::new' src/commands/plan/create.rs` inside `emit_override_events` drops from **3** to **1**, and `cargo make check` passes.

**Rule?** no — single handler, three copies only.

**Counter-argument:** Three explicit blocks document the set vs clear vs clear-all payloads. It loses because the payload shape is identical except for three fields already named at each call site; the closure keeps those names at the call.

**Depends on:** none.

## One-Touch Tidies

### T1 — Drop triplicate orchestrated-mode prose

**Evidence:** The same "legacy reviewer auto-creates a Specify change" paragraph appears verbatim in three briefs:

```text
adapters/targets/vectis/briefs/build.md:65
adapters/targets/vectis/briefs/build/ios/review.md:21
adapters/targets/vectis/briefs/build/android/review.md:21
```

Parent `build.md` § Consolidate review findings already covers orchestrated `design_findings` handling (`:60-65`).

**Action:**
1. Delete the entire `## Orchestrated mode` section (`:19-21`) from `adapters/targets/vectis/briefs/build/ios/review.md` and `adapters/targets/vectis/briefs/build/android/review.md`.
2. Add one bullet to each review brief's Pipeline step 5 (Synthesis): "Return classified `design_findings` per [build.md](adapters/targets/vectis/briefs/build.md) § Consolidate review findings."

**Quality delta:** `−6 LOC, −2 duplicate prose blocks`.

**Net LOC:** ios + android review briefs **58 → 52** combined.

**Done when:** `rg -nF 'legacy "reviewer auto-creates a Specify change"' adapters/targets/vectis/briefs/build/` returns **1** (parent brief only), and `make checks` passes.

**Rule?** no.

**Counter-argument:** Platform reviewers should carry standalone context. It loses because they already link the parent build brief for synthesis; triplicate prose is the drift class `make checks` brief caps were written to prevent.

**Depends on:** none.

### T2 — Delete `context.rs` `diag` one-liner wrapper

**Evidence:** `src/commands/context.rs:42-47` defines `diag(code, detail)` — a two-field `Error::Diag` wrapper. Grep shows **zero** call sites outside the definition (handlers use `Error::Diag` directly or `error_from_fence`).

**Action:**
1. Delete `fn diag` at `:42-47`.

**Quality delta:** `−6 LOC, −1 dead fn`.

**Net LOC:** `context.rs` **204 → 198**.

**Done when:** `rg -n 'fn diag' src/commands/context.rs` returns **0**, and `cargo make check` passes.

**Rule?** no.

**Counter-argument:** The helper standardises future context errors. It loses because it has zero callers today — YAGNI until a second site appears.

**Depends on:** none.

## Post-mortem

- **F1:** actual ΔLOC **−30** vs predicted **−52** (re-export + `#[expect(clippy::redundant_pub_crate)]` + `pub` visibility widened helpers offset deletion); done-when flipped cleanly (doctor duplicate fns 0, `pub(crate) use test_support` ≥1); `cargo make check` passed; no regressions — initial compile failed until helpers were `pub` not `pub(super)`.
- **F2:** actual ΔLOC **−110** vs predicted **−75** (cascade removed unused `Entry`/`Lifecycle` imports per prior F2 pattern); done-when flipped cleanly (`entries: vec![Entry {` 8→0); `cargo make check` passed; no regressions.
- **F3:** actual ΔLOC **−52** vs predicted **−40** (unused imports removed with literals); done-when flipped cleanly (`Entry {` in io tests 3→0); `cargo make check` passed; no regressions.
- **F4:** actual ΔLOC **−23** vs predicted **−22** (`create.rs` 918→900); done-when mostly clean (`journal::Event::new` in `emit_override_events` 3→1; `PlanAmendAuthorityOverride` file-wide 4→2 because `plan add` site remains — REVIEW "stays 3" miscounted the separate handler); `cargo make check` passed; no regressions — RFC-27 §D3 journal ordering/payload tests green.
- **T1:** actual ΔLOC **−6** vs predicted **−6** (ios + android review briefs); done-when flipped cleanly (legacy phrase 1 hit, parent `build.md` only); `make checks` passed after fixing broken `../../build.md` link in REVIEW action text; no regressions.
- **T2:** actual ΔLOC **−2** net vs predicted **−6** (`context.rs` −7 but 5 callers existed in `check.rs`/`generate.rs`, not zero — inlined `Error::Diag` at call sites); done-when flipped cleanly (`fn diag` 0); `cargo make check` passed; no regressions — REVIEW "zero callers" claim was stale.
