# Code & Skill Review — specify + specify-cli

Top three by tier: **F1 Delete topological sorter** (−157 LOC, −1 public method, −1 non-test `expect`), **F2 Delete unused adapter accessors** (−57 LOC, −4 public methods), **F3 Delete unused fusion writer** (−42 LOC, −1 public method, −1 module edge).
Total ΔLOC if all findings land: **approximately −288 LOC**.
Primary non-LOC axes moved: fewer public methods, fewer enum variants / impossible wire values, fewer non-test panic-adjacent matches, fewer module edges.
Top verified defects closed: **none qualified**; `make checks` and `cargo make check` both passed. Defect-only net ΔLOC: **0**.
Most likely to break in remediation: **F1** because removing the unused sorter also deletes tests that look like scheduler coverage; keep the `next_eligible` and `advance_next` tests.

## Reconnaissance

- `tokei`:
  - `specify`: 648 files, 87,105 total lines, 515 Markdown files / 49,647 Markdown lines.
  - `specify-cli`: 455 files, 66,216 total lines, 249 Rust files / 48,723 Rust lines.
- `cargo tree --duplicates` (`specify-cli`): non-empty; examples include `base64 v0.21.7` and `v0.22.1`, `reqwest v0.12.28` and `v0.13.3`, `rustix v0.38.44` and `v1.1.4`, `thiserror v1.0.69` and `v2.0.18`. Cargo edges are frozen for this pass, so no dependency finding qualified.
- `rg -c '^#\[test\]' crates/ src/ tests/` (`specify-cli`): **517 matches across 36 files**.
- `rg --files -g '**/mod.rs'` (`specify-cli`): **3 files**: `crates/domain/tests/common/mod.rs`, `wasi-tools/vectis/tests/engine_support/mod.rs`, `tests/common/mod.rs`.
- `wc -l docs/standards/*.md AGENTS.md`:
  - `specify`: **534 total**.
  - `specify-cli`: **639 total**.
- Files >500 lines under `crates/` and `src/` (`specify-cli`): `crates/tool/src/validate.rs` 520, `crates/domain/src/journal.rs` 656, `crates/domain/src/slice/fusion.rs` 903, `crates/domain/src/discovery/document.rs` 908, `crates/domain/src/spec/provenance.rs` 607, `crates/domain/src/adapter/core.rs` 771, `crates/domain/src/adapter/cache/io.rs` 509, `crates/domain/src/change/plan/core/model.rs` 630, `src/commands/plan/create.rs` 966.
- `make checks` (`specify`): **passed**, output `All checks passed.` Total failures: **0**; first 5 predicate ids: **none**.
- `cargo make check` (`specify-cli`): **passed**, output summary `855 tests run: 855 passed, 2 skipped` and `Build Done in 99.78 seconds.` First error: **none**.
- `rg -c '\.(unwrap|expect)\(' --glob '!**/tests/**' crates/ src/` (`specify-cli`): **731 matches across 57 files**.
- `rg -c 'panic!|unreachable!' --glob '!**/tests/**' crates/ src/` (`specify-cli`): **50 matches across 21 files**.

## Structural Findings

### F1 — Delete topological sorter

**Evidence:** `crates/domain/src/change/plan/core/next.rs:100-140` defines `Plan::topological_order`, but `rg 'topological_order\('` finds only the method and its own tests in the same file. Recon also shows `crates/domain/src/change/plan/core/next.rs` at 374 lines, and the raw panic-adjacent count is 731 because the unused method contains `expect("indegree init covers every node")` at `crates/domain/src/change/plan/core/next.rs:130`.

Current-state grep:

```text
crates/domain/src/change/plan/core/next.rs:100:    pub fn topological_order(&self) -> Result<Vec<&Entry>, Error> {
crates/domain/src/change/plan/core/next.rs:261:            .topological_order()
crates/domain/src/change/plan/core/next.rs:290:        let err = plan.topological_order().expect_err("cycle must surface as Err");
crates/domain/src/change/plan/core/next.rs:310:            .topological_order()
crates/domain/src/change/plan/core/next.rs:322:            .topological_order()
crates/domain/src/change/plan/core/next.rs:372:        assert!(plan.topological_order().is_err(), "cycle should surface from topological_order");
```

**Action:**
1. Delete `Plan::topological_order` from `crates/domain/src/change/plan/core/next.rs`.
2. Delete tests `topo_order_rfc_example`, `topo_order_cycle_errors`, `topo_order_deterministic_tiebreak`, and `next_eligible_with_cycle`.
3. Drop now-unused imports: `std::cmp::Reverse`, `BinaryHeap`, `petgraph::Direction`, `petgraph::algo::{tarjan_scc, toposort}`, and `petgraph::graph::NodeIndex`.
4. Keep `next_eligible_*` and `advance_next_*` tests; those cover the live scheduler.

Before:

```rust
pub fn topological_order(&self) -> Result<Vec<&Entry>, Error> {
    let graph = entry_dependency_graph(&self.entries);
    let idx: HashMap<&str, NodeIndex> = graph.node_indices().map(|n| (graph[n], n)).collect();
    // ...
    let entry = indegree.get_mut(&downstream).expect("indegree init covers every node");
}
```

After:

```rust
// No replacement. `plan next` uses `next_eligible` / `advance_next`.
```

**Quality delta:** `−157 LOC, −1 public method, −1 panic-adjacent non-test match, −5 imports, −1 unused branch family`.

**Net LOC:** `crates/domain/src/change/plan/core/next.rs` **374 → ~217**.

**Done when:** `rg 'topological_order|indegree init covers every node' crates/domain/src/change/plan/core/next.rs` returns **0** and `cargo make check` passes.

**Rule?** no.

**Counter-argument:** A full topological order is useful for future diagnostics. It loses because the live CLI scheduler never calls it, and `doctor/cycle.rs` already owns cycle diagnostics.

**Depends on:** none.

### F2 — Delete unused adapter accessors

**Evidence:** `crates/domain/src/adapter/core.rs:355-366`, `crates/domain/src/adapter/core.rs:428-439`, and `crates/domain/src/adapter/core.rs:456-466` expose `locate` and `brief_path` helpers with no external callers. `rg '(SourceAdapter|TargetAdapter)::locate|\.locate\('` returns **no matches**. `rg '\.brief_path\('` returns only the two wrapper bodies calling the inner helper.

Current-state grep:

```text
crates/domain/src/adapter/core.rs:365:    pub fn brief_path(&self, root_dir: &Path, operation: SourceOperation) -> Option<PathBuf> {
crates/domain/src/adapter/core.rs:438:    pub fn brief_path(&self, root_dir: &Path, operation: TargetOperation) -> Option<PathBuf> {
crates/domain/src/adapter/core.rs:456:    pub fn brief_path(&self, operation: SourceOperation) -> Option<PathBuf> {
crates/domain/src/adapter/core.rs:465:    pub fn brief_path(&self, operation: TargetOperation) -> Option<PathBuf> {
```

**Action:**
1. Delete `SourceAdapter::locate` and `TargetAdapter::locate`.
2. Delete `SourceAdapter::brief_path`, `TargetAdapter::brief_path`, `ResolvedSourceAdapter::brief_path`, and `ResolvedTargetAdapter::brief_path`.
3. Keep private `locate_axis`; it is still used by `resolve`.

Before:

```rust
pub fn brief_path(&self, root_dir: &Path, operation: SourceOperation) -> Option<PathBuf> {
    self.briefs.get(&operation).map(|relative| root_dir.join(relative))
}
```

After:

```rust
// No replacement. Callers already use resolved roots / manifest fields directly.
```

**Quality delta:** `−57 LOC, −6 public methods, −4 wrapper call sites, −1 public API surface`.

**Net LOC:** `crates/domain/src/adapter/core.rs` **771 → ~714**.

**Done when:** `rg 'pub fn (locate|brief_path)' crates/domain/src/adapter/core.rs` returns **0** and `cargo make check` passes.

**Rule?** no.

**Counter-argument:** These are convenient for future skill-side brief loading. It loses because no current Rust path uses them, and pre-1.0 public API compatibility is explicitly out of scope.

**Depends on:** none.

### F3 — Delete unused fusion writer

**Evidence:** `crates/domain/src/slice/fusion.rs:231-233` defines `FusionIndex::write_atomic`, but `rg 'write_atomic\(&path\)|pub fn write_atomic|crate::slice::atomic' crates/domain/src/slice/fusion.rs` shows only the method, its private tests, and its `atomic` import. The shipped refine skill states the skill body is the writer: `plugins/spec/skills/refine/SKILL.md:62` says, `There is no specify slice fusion write verb — the skill body is the writer`.

Current-state grep:

```text
crates/domain/src/slice/fusion.rs:28:use crate::slice::atomic;
crates/domain/src/slice/fusion.rs:231:    pub fn write_atomic(&self, path: &Path) -> Result<()> {
crates/domain/src/slice/fusion.rs:617:        original.write_atomic(&path).expect("write");
crates/domain/src/slice/fusion.rs:619:        original.write_atomic(&path).expect("re-write");
crates/domain/src/slice/fusion.rs:632:        let err = bad.write_atomic(&path).expect_err("schema must reject");
```

**Action:**
1. Delete `FusionIndex::write_atomic`.
2. Delete tests `write_atomic_then_load_round_trips_byte_stable` and `write_atomic_rejects_schema_invalid_index`.
3. Remove `use crate::slice::atomic;`.

Before:

```rust
pub fn write_atomic(&self, path: &Path) -> Result<()> {
    self.validate()?;
    atomic::yaml_write(path, self)
}
```

After:

```rust
// No replacement. The CLI validates and reads fusion.yaml; refine writes it.
```

**Quality delta:** `−42 LOC, −1 public method, −1 module edge, −2 writer-only tests`.

**Net LOC:** `crates/domain/src/slice/fusion.rs` **903 → ~861**.

**Done when:** `rg 'write_atomic\(&path\)|pub fn write_atomic|crate::slice::atomic' crates/domain/src/slice/fusion.rs` returns **0** and `cargo make check` passes.

**Rule?** no.

**Counter-argument:** A typed writer is safer than agent-authored YAML. It loses because the shipped workflow explicitly assigns writing to the skill body and this method is only tested against itself.

**Depends on:** none.

### F4 — Delete impossible ClearAll action

**Evidence:** `AuthorityOverrideAction::ClearAll` is serializable but not emitted by the CLI. `src/commands/plan/create.rs:385-403` expands `--clear-authority-overrides` into one `AuthorityOverrideAction::Clear` event per existing kind, and the integration test at `tests/plan_orchestrate.rs:2113-2163` asserts every emitted event has `"action":"clear"`. `rg 'AuthorityOverrideAction::ClearAll|clear-all'` finds only the enum docs and an artificial unit test in `crates/domain/src/journal.rs`.

Current-state grep:

```text
crates/domain/src/journal.rs:226:        /// is [`AuthorityOverrideAction::ClearAll`].
crates/domain/src/journal.rs:313:pub enum AuthorityOverrideAction {
crates/domain/src/journal.rs:601:                action: AuthorityOverrideAction::ClearAll,
crates/domain/src/journal.rs:608:        assert!(line.contains(r#""action":"clear-all""#));
```

**Action:**
1. Delete `AuthorityOverrideAction::ClearAll`.
2. Delete `plan_amend_authority_override_clear_all_elides_optional_fields` from `crates/domain/src/journal.rs`.
3. Adjust the optional-field comments so `claim_kind` is absent only for future actions, or simply remove the ClearAll-specific sentence.
4. Keep `tests/plan_orchestrate.rs:2113-2163`; it is the live behavior.

Before:

```rust
pub enum AuthorityOverrideAction {
    Set,
    Clear,
    ClearAll,
}
```

After:

```rust
pub enum AuthorityOverrideAction {
    Set,
    Clear,
}
```

**Quality delta:** `−25 LOC, −1 enum variant, −1 impossible wire value, −1 unit test for un-emitted behavior`.

**Net LOC:** `crates/domain/src/journal.rs` **656 → ~631**.

**Done when:** `rg 'ClearAll|"clear-all"' crates/domain/src/journal.rs` returns **0** and `cargo make check` passes.

**Rule?** no.

**Counter-argument:** `clear-all` preserves which flag the operator typed. It loses because current integration behavior deliberately records per-kind `clear` events, and carrying a never-emitted wire value makes consumers handle a ghost branch.

**Depends on:** none.

## One-Touch Tidies

### T1 — Inline one-use fusion path

**Evidence:** `crates/domain/src/slice/fusion.rs:303-305` defines `fusion_path`, and `rg 'fusion_path\('` finds one production caller at `src/commands/slice/validate.rs:198` plus its own test at `crates/domain/src/slice/fusion.rs:899-901`.

Current-state grep:

```text
src/commands/slice/validate.rs:198:    let fusion_path = fusion::fusion_path(slice_dir);
crates/domain/src/slice/fusion.rs:303:pub fn fusion_path(slice_dir: &Path) -> PathBuf {
crates/domain/src/slice/fusion.rs:900:        let p = fusion_path(Path::new("/proj/.specify/slices/my-slice"));
```

**Action:**
1. Replace the caller with `let fusion_path = slice_dir.join("fusion.yaml");`.
2. Delete `fusion_path` and its unit test.

Before:

```rust
let fusion_path = fusion::fusion_path(slice_dir);
```

After:

```rust
let fusion_path = slice_dir.join("fusion.yaml");
```

**Quality delta:** `−7 LOC, −1 public function`.

**Net LOC:** `crates/domain/src/slice/fusion.rs` **903 → ~897**, `src/commands/slice/validate.rs` **no net change**.

**Done when:** `rg 'fusion_path' crates/domain/src/slice/fusion.rs src/commands/slice/validate.rs` returns **0** and `cargo make check` passes.

**Rule?** no.

**Counter-argument:** A path helper avoids misspelling `"fusion.yaml"`. It loses because there is one caller and the helper has more test code than behavior.

**Depends on:** none.

## Notes

No shipped Skill integrity defect qualified: `make checks` passed with **0** failures. I did not recommend dependency deduplication because `Cargo.toml` / `Cargo.lock` are frozen for this pass, and the duplicate tree is dominated by transitive `wasmtime` / `wasm-pkg-client` edges.

## Post-mortem

- F1: actual ΔLOC -171 vs predicted -157; done-when flipped cleanly: yes (no matches; rg exit 1); regressions: final CI initially caught stale rustdoc link, fixed; validation: cargo make check and cargo make doc passed.
- F2: actual ΔLOC -62 vs predicted -57; done-when flipped cleanly: yes (no matches); regressions: none; validation: cargo make check passed.
- F3: actual ΔLOC -46 vs predicted -42; done-when flipped cleanly: yes (no matches; rg exit 1); regressions: none; validation: cargo make check passed.
- F4: actual ΔLOC -25 vs predicted -25; done-when flipped cleanly: yes (no matches; rg exit 1); regressions: none; validation: cargo make check passed.
- T1: actual ΔLOC -17 vs predicted -7; done-when flipped cleanly: yes (no matches; rg exit 1); regressions: none; validation: cargo make check passed.
