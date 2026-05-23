# Code & Skill Review — specify + specify-cli

Top three by tier: **F1 Delete unemitted post-2.0 journal events** (−30 LOC, −3 enum variants, −3 unemittable wire IDs), **F2 Trim dead `crates/domain/src/adapter.rs` re-exports** (−18 names, −1 module-edge surface, ≥ 2 axes), **F3 Collapse duplicate `Candidate::matches` / `resolves`** (−15 LOC, −1 method, −1 redundant test).
Total ΔLOC if all findings land: **approximately −85 LOC**.
Primary non-LOC axes moved: fewer enum variants, fewer pub-API names, fewer methods, fewer dead documentation-only branches.
Top verified defects closed: **none qualified**; `make checks` passed (`All checks passed.`) and `cargo clippy --workspace --all-targets --all-features -- -D warnings` finished clean. Defect-only net ΔLOC: **0** (under the +30 cap).
Most likely to break in remediation: **F1** because the three `EventKind` variants are documented as a wire-shape lock for post-2.0 emit sites; removing them invites a cosmetic re-add later. Mitigation: the master rule says ignore back-compat pre-1.0; re-adding the closed enum variants when emit sites land is a one-line edit.

## Reconnaissance

- `tokei`:
  - `specify`: 648 files, 87,034 total lines, Markdown 3,104 lines / 82 files (excluding embedded code blocks).
  - `specify-cli`: 455 files, 65,894 total lines, Rust 48,491 lines / 249 files.
- `cargo tree --duplicates` (`specify-cli`): non-empty (`base64 v0.21.7` and `v0.22.1`, `reqwest v0.12.28` and `v0.13.3`, `rustix v0.38.44` and `v1.1.4`, `thiserror v1.0.69` and `v2.0.18`); transitive `wasmtime` / `wasm-pkg-client` chains. `Cargo.toml` is frozen for the pass.
- `cargo +nightly udeps --workspace --all-targets`: **`All deps seem to have been used.`** No dependency-removal finding qualified.
- `rg -c '^#\[test\]' crates/ src/ tests/` (`specify-cli`): 36 files matched (per-file count not aggregated; previous review's 517 figure remains the order of magnitude).
- `rg --files -g '**/mod.rs'`: **3 files** (`crates/domain/tests/common/mod.rs`, `wasi-tools/vectis/tests/engine_support/mod.rs`, `tests/common/mod.rs`).
- `wc -l docs/standards/*.md AGENTS.md`:
  - `specify`: **534 total**.
  - `specify-cli`: **636 total** (excluding `DECISIONS.md`).
- Files >500 lines under `crates/` and `src/` (`specify-cli`):
  - Tests: `crates/domain/tests/workspace.rs` 1041, `crates/domain/tests/finalize.rs` 947, `crates/domain/tests/registry.rs` 922, `crates/domain/src/change/plan/core/validate/tests.rs` 695.
  - Source: `src/commands/plan/create.rs` 966, `crates/domain/src/discovery/document.rs` 908, `crates/domain/src/slice/fusion.rs` 839, `crates/domain/src/adapter/core.rs` 709, `crates/domain/src/journal.rs` 631, `crates/domain/src/change/plan/core/model.rs` 629, `crates/domain/src/spec/provenance.rs` 607, `crates/tool/src/validate.rs` 520, `crates/domain/src/adapter/cache/io.rs` 509.
- `make checks` (`specify`): **passed**, output `All checks passed.` Total failures: **0**; first 5 predicate ids: **none**.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` (`specify-cli`): **passed** (`Finished \`dev\` profile […] in 48.00s`). First error: **none**.
- `rg -c '\.(unwrap|expect)\(' --glob '!**/tests/**' --glob '!**/wasi-tools/**' crates/ src/` (`specify-cli`): **716 matches across 57 files** (previous review reported 731; the −15 came from the F1–F4 deletions in the prior pass).
- `rg -c 'panic!|unreachable!' --glob '!**/tests/**' --glob '!**/wasi-tools/**' crates/ src/` (`specify-cli`): **49 matches across 20 files** (previous review reported 50; −1 came from F1).

## Structural Findings

### F1 — Delete unemitted post-2.0 journal event variants

**Evidence:** `crates/domain/src/journal.rs:241-268` declares `EventKind::SliceBuildFailed`, `EventKind::SliceMergeConflicted`, and `EventKind::PlanTransitionArchived`. `rg 'SliceBuildFailed|SliceMergeConflicted|PlanTransitionArchived'` finds **only** the variant declarations themselves — no constructor, no test, no caller in `src/` or any other domain module. The doc comments concede the gap: each variant says "Emitter sites land post-2.0; the wire shape is locked here." The master rule explicitly ignores pre-1.0 wire-shape concerns.

Current-state grep:

```text
crates/domain/src/journal.rs:241:    #[serde(rename = "slice.build.failed", rename_all = "kebab-case")]
crates/domain/src/journal.rs:242:    SliceBuildFailed {
crates/domain/src/journal.rs:253:    #[serde(rename = "slice.merge.conflicted", rename_all = "kebab-case")]
crates/domain/src/journal.rs:254:    SliceMergeConflicted {
crates/domain/src/journal.rs:264:    #[serde(rename = "plan.transition.archived", rename_all = "kebab-case")]
crates/domain/src/journal.rs:265:    PlanTransitionArchived {
```

**Action:**
1. Delete the three variant blocks at `crates/domain/src/journal.rs:233-268` (each is a doc block + `#[serde]` attr + `Variant { … },` body, ~10 lines each).
2. No test deletions required — `rg` shows zero test references.
3. No call-site rewrites — `rg` shows zero constructors.

Before:

```rust
#[serde(rename = "slice.build.failed", rename_all = "kebab-case")]
SliceBuildFailed {
    slice_name: String,
},
#[serde(rename = "slice.merge.conflicted", rename_all = "kebab-case")]
SliceMergeConflicted {
    slice_name: String,
},
#[serde(rename = "plan.transition.archived", rename_all = "kebab-case")]
PlanTransitionArchived {
    plan_name: String,
},
```

After:

```rust
// No replacement. Re-add the closed variants when /spec:build, /spec:merge,
// or specify plan finalize gain the matching emit sites.
```

**Quality delta:** `−30 LOC, −3 enum variants, −3 unemittable wire IDs, −3 doc-only paragraphs`.

**Net LOC:** `crates/domain/src/journal.rs` **631 → ~601**.

**Done when:** `rg 'SliceBuildFailed|SliceMergeConflicted|PlanTransitionArchived' crates/ src/` returns **0** and `cargo clippy --workspace --all-targets --all-features -- -D warnings` passes.

**Rule?** no.

**Counter-argument:** Locking the wire shape now lets future emit sites land without a sequenced enum migration. It loses because the master rule explicitly drops pre-1.0 back-compat as a justification, the variants are also untested (so the "lock" claim is unverified), and re-adding three closed-enum variants is a 30-line edit when emit sites materialise.

**Depends on:** none.

### F2 — Trim dead `crates/domain/src/adapter.rs` re-exports

**Evidence:** `crates/domain/src/adapter.rs:36-48` re-exports 28 names. A targeted scan across `src/`, `crates/error/`, `crates/tool/`, `wasi-tools/`, and `crates/domain/tests/` finds external callers for only **10** of them: `ADAPTER_FILENAME`, `AdapterLocation`, `Axis`, `ResolvedTargetAdapter`, `SourceAdapter`, `TargetAdapter`, `cache_dir`, `check_axis_unique_for_name`, `CacheLayout`, `SourceOperation`, `cache_read_index`, and `TargetOperation`. The remaining 18 names are either used only inside `crates/domain/src/adapter/` (so the in-module `pub` already covers them) or have no callers at all.

Current-state grep (a sample of the unused names; full list below):

```text
$ rg 'ADAPTERS_DIR|EXTRACTIONS_CACHE_DIR|MANIFESTS_CACHE_DIR|adapter_axis_dir|AdapterToolDeclaration|CacheMode|ResolvedSourceAdapter|CacheFingerprint|CacheIndexEntry|CacheLookup|CacheMissReason|FingerprintRecord|FingerprintSource|FingerprintToolVersion|LookupOutcome|append_index|sha256_file|sha256_prefixed|cache_lookup|cache_write' src/ crates/error/ crates/tool/ wasi-tools/ crates/domain/tests/
(no matches)
```

The same 18 names are unused outside the `adapter/` subtree per `rg --type rust crates/ src/` (matches only in `crates/domain/src/adapter.rs`, `crates/domain/src/adapter/core.rs`, `crates/domain/src/adapter/cache.rs`, `crates/domain/src/adapter/cache/io.rs`).

**Action:**
1. In `crates/domain/src/adapter.rs:36-47`, remove these 18 names from the two `pub use` blocks: `ADAPTERS_DIR`, `EXTRACTIONS_CACHE_DIR`, `MANIFESTS_CACHE_DIR`, `AdapterToolDeclaration`, `CacheMode`, `ResolvedSourceAdapter`, `adapter_axis_dir`, `CacheFingerprint`, `CacheIndexEntry`, `CacheLookup`, `CacheMissReason`, `FingerprintRecord`, `FingerprintSource`, `FingerprintToolVersion`, `LookupOutcome`, `append_index`, `sha256_file`, `sha256_prefixed`, `lookup as cache_lookup`, `write as cache_write`.
2. Leave the `pub` modifiers on the original definitions in `core.rs`, `cache.rs`, and `cache/io.rs` — internal callers reach them via `crate::adapter::core::…` / `crate::adapter::cache::…` paths and the test harness inside the `adapter/` subtree relies on `pub` for cross-module use.
3. Run `cargo clippy --workspace --all-targets --all-features -- -D warnings` to confirm no consumer broke; the 10 surviving re-exports cover every external caller.

Before:

```rust
pub use core::{
    ADAPTER_FILENAME, ADAPTERS_DIR, AdapterLocation, AdapterToolDeclaration, Axis, CacheMode,
    EXTRACTIONS_CACHE_DIR, MANIFESTS_CACHE_DIR, ResolvedSourceAdapter, ResolvedTargetAdapter,
    SourceAdapter, TargetAdapter, adapter_axis_dir, cache_dir, check_axis_unique_for_name,
};
pub use cache::{
    CacheFingerprint, CacheIndexEntry, CacheLayout, CacheLookup, CacheMissReason,
    FingerprintRecord, FingerprintSource, FingerprintToolVersion, LookupOutcome, SourceOperation,
    append_index, lookup as cache_lookup, read_index as cache_read_index, sha256_file,
    sha256_prefixed, write as cache_write,
};
```

After:

```rust
pub use core::{
    ADAPTER_FILENAME, AdapterLocation, Axis, ResolvedTargetAdapter, SourceAdapter, TargetAdapter,
    cache_dir, check_axis_unique_for_name,
};
pub use cache::{CacheLayout, SourceOperation, read_index as cache_read_index};
```

**Quality delta:** `−10 LOC, −18 public-API names, −2 module-edge surfaces, +0 axes broken (every kept name still has at least one external caller)`.

**Net LOC:** `crates/domain/src/adapter.rs` **49 → ~39**; downstream module-edges drop because external callers no longer pull through redundant re-exports.

**Done when:** `rg 'pub use core::|pub use cache::' crates/domain/src/adapter.rs | wc -l` returns **2**, the surviving names match the after-block above verbatim, and `cargo clippy --workspace --all-targets --all-features -- -D warnings` passes.

**Rule?** no.

**Counter-argument:** Re-exports document the module's "public face." It loses because the master rule prefers fewer module-crossing names, the unused 18 hide the 10 that callers actually need, and re-adding a name is one edit when a future caller wants it.

**Depends on:** none.

### F3 — Collapse duplicate `Candidate::matches` / `resolves`

**Evidence:** `crates/domain/src/discovery/candidate.rs:86-103` defines two methods that share the same body — `Candidate::matches` (5 lines) and `Candidate::resolves` (3 lines, calls `matches`). Production callers use only `resolves`; tests cover both surfaces. The doc on `resolves` admits the duplication: `"Kept as a thin alias so call sites can read in the same vocabulary the RFC speaks."`

Current-state grep:

```text
crates/domain/src/discovery/document.rs:151:        let hits: Vec<&Candidate> = self.candidates.iter().filter(|c| c.resolves(token)).collect();
crates/domain/src/discovery/candidate.rs:93:    pub fn matches(&self, needle: &str) -> bool {
crates/domain/src/discovery/candidate.rs:101:    pub fn resolves(&self, token: &str) -> bool {
crates/domain/src/discovery/candidate.rs:102:        self.matches(token)
```

`rg '\.matches\(' crates/ src/` outside the file returns 0 production hits; `rg '\.resolves\(' crates/ src/` outside the file returns 1 production hit (line 151 above). Tests in `candidate.rs:222-249` exercise both surfaces redundantly (the `resolves_is_alias_for_matches` test exists solely to prove the alias is still an alias).

**Action:**
1. Delete `Candidate::matches` (lines 86-95) — its 1-line body is the entire `resolves` body once expanded.
2. Rename `resolves`'s body so it carries the merged comment/expansion: `self.id == needle || self.aliases.contains(needle)`.
3. Drop the test `resolves_is_alias_for_matches` (lines 239-250) and rename the surviving `matches_resolves_id_then_aliases` test (lines 222-236) to `resolves_id_then_aliases`, replacing each `candidate.matches(…)` call with `candidate.resolves(…)`.
4. No change to `discovery/document.rs:151` — it already uses `c.resolves(token)`.

Before:

```rust
pub fn matches(&self, needle: &str) -> bool {
    self.id == needle || self.aliases.contains(needle)
}

pub fn resolves(&self, token: &str) -> bool {
    self.matches(token)
}
```

After:

```rust
pub fn resolves(&self, token: &str) -> bool {
    self.id == token || self.aliases.contains(token)
}
```

**Quality delta:** `−15 LOC, −1 method, −1 alias-only test, −1 internal idiom (a method calling its own twin)`.

**Net LOC:** `crates/domain/src/discovery/candidate.rs` **318 → ~303**.

**Done when:** `rg 'fn matches\(|\.matches\(' crates/domain/src/discovery/` returns **0** (the method and its callers are gone) and `cargo clippy --workspace --all-targets --all-features -- -D warnings` passes.

**Rule?** no.

**Counter-argument:** Two names lets one read like RFC prose (`resolves`) and the other like Rust convention (`matches`). It loses because there is exactly one production caller, the alias test exists only to assert the duplication is still in place, and the merged surface is the one the RFC already speaks.

**Depends on:** none.

## One-Touch Tidies

### T1 — Inline `discovery_path()`

**Evidence:** `crates/domain/src/discovery/document.rs:638-640` defines `pub fn discovery_path(project_dir: &Path) -> PathBuf { project_dir.join("discovery.md") }`. The only production caller is `Layout::discovery_path` at `crates/domain/src/config.rs:210-211`, which is itself the wrapper every other handler uses. The standalone helper plus the re-export in `crates/domain/src/discovery.rs:18` is two indirection levels for one `.join("discovery.md")`.

Current-state grep:

```text
crates/domain/src/discovery.rs:18:    Discovery, DiscoveryAliasCollision, ResolveError as DiscoveryResolveError, discovery_path,
crates/domain/src/config.rs:210:    pub fn discovery_path(&self) -> PathBuf {
crates/domain/src/config.rs:211:        crate::discovery::discovery_path(self.project_dir)
crates/domain/src/discovery/document.rs:638:pub fn discovery_path(project_dir: &Path) -> PathBuf {
```

**Action:**
1. Replace `Layout::discovery_path`'s body with `self.project_dir.join("discovery.md")`.
2. Delete `pub fn discovery_path` and its 7-line doc block from `crates/domain/src/discovery/document.rs`.
3. Drop the `discovery_path` name from the `pub use` block in `crates/domain/src/discovery.rs:17-19`.

Before:

```rust
pub fn discovery_path(&self) -> PathBuf {
    crate::discovery::discovery_path(self.project_dir)
}
```

After:

```rust
pub fn discovery_path(&self) -> PathBuf {
    self.project_dir.join("discovery.md")
}
```

**Quality delta:** `−10 LOC, −1 public function, −1 module-edge re-export`.

**Done when:** `rg 'fn discovery_path|crate::discovery::discovery_path' crates/domain/src/discovery/document.rs crates/domain/src/discovery.rs` returns **0** and `cargo clippy --workspace --all-targets --all-features -- -D warnings` passes.

**Rule?** no.

**Counter-argument:** A path helper avoids spelling `"discovery.md"` twice. It loses because `Layout::discovery_path` is already the single shared writer/reader spelling, and the helper has more rustdoc than behavior.

**Depends on:** none.

### T2 — Drop unused `Discovery::candidates()` accessor

**Evidence:** `crates/domain/src/discovery/document.rs:121-124` declares `pub fn candidates(&self) -> &[Candidate]`. `rg '\.candidates\(\)' crates/ src/ tests/` returns **no matches** (only an internal rustdoc reference at line 58). The accessor is unused.

Current-state grep:

```text
$ rg '\.candidates\(\)|Discovery::candidates' crates/ src/ tests/
crates/domain/src/discovery/document.rs:58:    /// pure prose ([`Discovery::candidates`] is empty and the heading
```

**Action:**
1. Delete `pub fn candidates(&self) -> &[Candidate] { &self.candidates }` and its doc block (~5 lines).
2. Update the rustdoc cross-reference at line 58 to mention `self.candidates` directly or drop the parenthetical.

**Quality delta:** `−5 LOC, −1 unused public accessor`.

**Done when:** `rg 'pub fn candidates' crates/domain/src/discovery/document.rs` returns **0** and `cargo clippy --workspace --all-targets --all-features -- -D warnings` passes.

**Rule?** no.

**Counter-argument:** A read accessor is conventional. It loses because `Discovery::resolve_candidate` and `check_alias_collisions` already own every read path, the field is private only in spirit (the surrounding methods touch it directly), and rustdoc cross-refs survive without it.

**Depends on:** none.

### T3 — Drop dead `complete` lifecycle prose from drop SKILL.md

**Evidence:** `crates/domain/src/slice/lifecycle.rs:23-34` defines the closed `LifecycleStatus` enum with five variants: `Refining`, `Refined`, `Built`, `Merged`, `Dropped`. `plugins/spec/skills/drop/SKILL.md:38` and `:93` both reference a non-existent `complete` status — the matching `if status == "complete"` arm in the skill body can never fire because no slice carries that string on disk. (`make checks` passes — there is no skill predicate that catches this content drift, so this finding qualifies on subtraction, not on closing a verified defect.)

Current-state grep:

```text
plugins/spec/skills/drop/SKILL.md:38:   - `complete`: warn that the slice appears ready to merge normally — `/spec:merge` may be the intended action.
plugins/spec/skills/drop/SKILL.md:93:- Warn if the slice is already `complete`, since `/spec:merge` may be the intended action.
crates/domain/src/slice/lifecycle.rs:23:pub enum LifecycleStatus {
```

**Action:**
1. In `plugins/spec/skills/drop/SKILL.md:38`, replace the `complete` case with `built`: `` - `built`: warn that the slice is ready for `/spec:merge`. ``
2. In `plugins/spec/skills/drop/SKILL.md:93`, replace `already complete` with `already built`.
3. Re-run `make checks` to confirm the predicate set still passes.

Before:

```text
- `complete`: warn that the slice appears ready to merge normally — `/spec:merge` may be the intended action.
…
- Warn if the slice is already `complete`, since `/spec:merge` may be the intended action.
```

After:

```text
- `built`: warn that the slice is ready for `/spec:merge`.
…
- Warn if the slice is already `built`, since `/spec:merge` may be the intended action.
```

**Quality delta:** `−0 LOC, −1 unreachable case, −1 doc drift between skill body and the closed Rust enum`.

**Net LOC:** `plugins/spec/skills/drop/SKILL.md` **96 → 96** (rename only; the rule forbids rename-only edits unless the rename unblocks a deletion in the same finding — here it eliminates one unreachable-case branch from the skill body's lifecycle ladder).

**Done when:** `rg '\bcomplete\b' plugins/spec/skills/drop/SKILL.md` returns **0** and `make checks` still prints `All checks passed.`

**Rule?** no.

**Counter-argument:** Operators may type "complete" as a synonym; better to keep the warning. It loses because the on-disk `.metadata.yaml.status` field is the closed `LifecycleStatus` enum (kebab-cased to `refining | refined | built | merged | dropped`); `complete` cannot appear there, and the skill's text falsely implies a lifecycle state that does not exist.

**Depends on:** none.

### T4 — Drop unused `CandidateAliases::new()` constructor

**Evidence:** `crates/domain/src/discovery/candidate.rs:55-58` defines `pub const fn new() -> Self { Self { names: Vec::new() } }`. The struct already derives `Default` (line 45). The only caller is the `sample()` test helper at line 302; every other test or constructor uses `CandidateAliases::from_iter(...)` or `Default::default()` indirectly via `#[derive(Default)]`.

Current-state grep:

```text
crates/domain/src/discovery/candidate.rs:56:    pub const fn new() -> Self {
crates/domain/src/discovery/candidate.rs:302:            aliases: CandidateAliases::new(),
```

**Action:**
1. Delete `pub const fn new()` (4 lines including doc).
2. Replace the single test use at line 302 with `CandidateAliases::default()` (or omit the field — `Default` covers it via the test helper's struct-update syntax).

**Quality delta:** `−5 LOC, −1 redundant constructor (collapses to derive(Default))`.

**Done when:** `rg 'CandidateAliases::new' crates/domain/src/discovery/candidate.rs` returns **0** and `cargo clippy --workspace --all-targets --all-features -- -D warnings` passes.

**Rule?** no.

**Counter-argument:** A `const fn new()` reads more idiomatically than `default()` at struct-update sites. It loses because `#[derive(Default)]` already gives every caller the same value, and rust's idiom (e.g. `BTreeMap::new` vs `BTreeMap::default`) does not require declaring both.

**Depends on:** none.

## Post-mortem

<!-- One line per applied finding: actual ΔLOC vs predicted, did the "done when" assertion flip cleanly, did anything regress. -->
- F1 — actual ΔLOC -36 vs predicted −30; done-when grep flipped cleanly; no regression (`cargo make check` green).
- F2 — actual ΔLOC -6 vs predicted −10; done-when grep flipped with caveat: kept `CacheMode` (internal `crate::adapter::CacheMode` import in `adapter/cache/io.rs:34` REVIEW missed) and `TargetOperation` re-export untouched (its existing `pub use operation::TargetOperation;` line lives outside both trimmed blocks and has many external callers); no regression (`cargo make check` green).
- F3 — actual ΔLOC -22 vs predicted −15; done-when grep flipped cleanly; no regression (`cargo make check` green).
- T1 — actual ΔLOC -13 vs predicted −10; done-when grep flipped cleanly; no regression (`cargo make check` green).
- T2 — actual ΔLOC -6 vs predicted −5; done-when grep flipped cleanly; no regression (`cargo make check` green).
- T3 — actual ΔLOC 0 vs predicted 0; done-when grep flipped cleanly; no regression (`make checks` + `cargo make check` green).
- T4 — actual ΔLOC -6 vs predicted −5; done-when grep flipped with caveat: REVIEW missed 2 callers in `document.rs` test module — swapped to `default()` in same dispatch; no regression (`cargo make check` green).
- F2 follow-up — `cargo make ci` (rustdoc `-D warnings`) caught 5 broken intra-doc links the trim invalidated (`crate::adapter::CacheIndexEntry`, `crate::adapter::CacheFingerprint` ×2, `[adapter_axis_dir]`, `[ADAPTERS_DIR]`); fixed by repointing to `crate::adapter::cache::*` paths and dropping links to private items; +0 LOC; `cargo make ci` now green end-to-end.

## Notes

I deliberately did **not** flag the entire RFC-27 §D8 cache lookup / write surface (`crates/domain/src/adapter/cache/io.rs:173-323`) even though `rg 'SliceExtractCacheHit|SliceExtractCacheMiss'` shows zero emit sites and the matching `lookup` / `write` / `append_index` helpers have no production callers either. F2 trims the dead re-exports in the same direction without disturbing the on-disk RFC contract or the round-trip tests; the actual lookup/write functions are tested end-to-end inside the `adapter/cache/io.rs` test module and are wired up to land emit sites in the next change. The smaller F2 finding takes the subtraction win without re-implementing D8 later.

I also did not flag dependency deduplication — `cargo udeps` reports no unused workspace deps, and the four duplicate transitive trees (`base64`, `reqwest`, `rustix`, `thiserror`) are dominated by `wasmtime` / `wasm-pkg-client` chains the workspace has no upgrade authority over inside this pass.

`make checks` (`specify`) and `cargo clippy --workspace --all-targets --all-features -- -D warnings` (`specify-cli`) both passed at the start of the pass; no Skill-integrity or CI predicate defect qualified.
