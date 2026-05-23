# Code & Skill Review - specify + specify-cli

Top three findings by tier: **F1 Rename stale `init-requires-target-or-workspace` kebab** (verified wire-contract defect), **F2 Delete `InitPolicy::CreateMissing` and `default_for_load`** (subtraction plus one operator-path panic removal), **F3 Inline plan/create dedup helpers** (subtraction plus a -1 branch-cluster collapse).
Total ΔLOC if all land: **approximately -85 LOC**.
Primary non-LOC axes moved: fewer enum variants, fewer trait methods, lower panic surface, fewer hand-rolled helpers, fewer match-arm branches.
Top verified defects closed: **1 qualified** (`Error::Diag.code == "init-requires-target-or-workspace"` emitted by `specify-cli` contradicts `specify-cli/docs/init.md:5,49` which documents the kebab as `init-requires-adapter-or-hub`). Defect-only net ΔLOC: **0**.
Most likely to break in remediation: **F2** - it touches the `AtomicYaml` trait surface and the registry-add write path; the helper's atomic-write semantics must be preserved when the one `CreateMissing` caller is inlined.

## Reconnaissance

- `tokei`:
  - `specify`: **647 files**, **87,087 total lines**; Markdown **514 files / 49,526 lines**.
  - `specify-cli`: **446 files**, **64,723 total lines**; Rust **245 files / 47,642 lines**.
- `cargo tree --duplicates` (`specify-cli`): non-empty. `base64 v0.21.7 / v0.22.1`, `reqwest v0.12.28 / v0.13.3`, `bitflags v2.11.1` against `rustix v0.38.44 / v1.1.4`, plus the wider `wasmtime` / `wasm-pkg-client` transitive families. `Cargo.toml` is frozen for this pass.
- `rg -c '^#\[test\]' crates/ src/ tests/` (`specify-cli`): **512** test functions.
- `rg --files -g '**/mod.rs'` (`specify-cli`): **3** files - all under `tests/` trees (`tests/common/mod.rs`, `wasi-tools/vectis/tests/engine_support/mod.rs`, `crates/domain/tests/common/mod.rs`).
- `wc -l docs/standards/*.md AGENTS.md`:
  - `specify`: **555 total**.
  - `specify-cli`: **638 total** (DECISIONS.md adds **624** on top).
- Files >500 lines under `crates/` and `src/` (`specify-cli`):
  - Tests: `crates/domain/tests/workspace.rs` **1048**, `crates/domain/tests/finalize.rs` **947**, `crates/domain/tests/registry.rs` **922**.
  - Source: `src/commands/plan/create.rs` **956**, `crates/domain/src/discovery/document.rs` **891**, `crates/domain/src/slice/fusion.rs` **839**, `crates/domain/src/adapter/core.rs` **709**, `crates/domain/src/change/plan/core/model.rs` **629**, `crates/domain/src/spec/provenance.rs` **607**, `crates/domain/src/journal.rs` **595**, `crates/tool/src/validate.rs` **520**, `crates/domain/src/adapter/cache/io.rs` **509**.
- `make checks` (`specify`): **passed** - `All checks passed.` Total failures: **0**; first five predicate ids: **none**.
- `cargo make check` (`specify-cli`): **passed** - `Build Done in 164.24 seconds.` First error: **none**.
- `rg -c '\.(unwrap|expect)\(' --glob '!**/tests/**' crates/ src/` (`specify-cli`): summed **695** matching lines (filename-`*tests*.rs` files included; production-path count is materially smaller).
- `rg -c 'panic!|unreachable!' --glob '!**/tests/**' crates/ src/` (`specify-cli`): summed **48** matching lines.

## Structural Findings

### F1 - Rename stale `init-requires-*` kebab to match docs

**Evidence:** `specify-cli/docs/init.md:5` says "missing both surfaces as `init-requires-adapter-or-hub`" and `:49` reinforces "refuses the ambiguous shape at the entry point with the `init-requires-adapter-or-hub` discriminant." The CLI binary emits a different kebab from five production-path sites:

```text
crates/domain/src/init.rs:171:                    code: "init-requires-target-or-workspace",
crates/domain/src/init.rs:200:                    code: "init-requires-target-or-workspace",
crates/error/src/error.rs:140:                "init-requires-target-or-workspace" => Some(
crates/domain/src/init/regular.rs:35:        code: "init-requires-target-or-workspace",
crates/domain/src/init/hub.rs:51:            code: "init-requires-target-or-workspace",
```

The leftover `workspace` token predates the Specify 2.0 hub/adapter rename and is the same wire-contract drift the previous review closed in the skill repo (`plugins/spec/skills/init/SKILL.md`, `docs/reference/slice-skills/init.md`).

**Action:**
1. Replace `init-requires-target-or-workspace` with `init-requires-adapter-or-hub` in `crates/domain/src/init/regular.rs:35`, `crates/domain/src/init/hub.rs:51`, and the hint-match arm in `crates/error/src/error.rs:140`.
2. Update the two test assertions in `crates/domain/src/init.rs:171` and `:200` to match.
3. Rephrase the docstring in `crates/domain/src/init.rs:86` and the test comment in `tests/init.rs:127` to use the new spelling.
4. Adjust the historical note in `DECISIONS.md:266` to call out that 2.0 settled on the documented spelling.

**Quality delta:** `0 LOC, -1 wire-contract defect, -5 stale kebab emit sites`.

**Net LOC:** affected files **0 net change** (five string literals rename in place; test/doc strings round-trip).

**Architectural impact:** Defect-only finding. The kebab is the wire contract; emitting a token that contradicts `docs/init.md` breaks any operator or CI that filters JSON envelopes by `error: "init-requires-adapter-or-hub"`.

**Done when:** `rg -nF 'init-requires-target-or-workspace' crates/ src/ tests/` returns **0**, `rg -nF 'init-requires-adapter-or-hub' crates/ src/ tests/ docs/` returns **≥ 6**, and `cargo make check` passes.

**Rule?** no - one-time vocabulary alignment, no recurring pattern to police.

**Counter-argument:** The kebab is unreachable from the shipped clap surface (clap intercepts the empty / both-set cases) so the drift is academic. It loses because `docs/init.md` advertises this kebab to operators and downstream tooling, and the domain-level `init()` function is a public library entry point exercised directly by tests; the surfaced kebab must match the documented contract.

**Depends on:** none.

### F2 - Delete `InitPolicy::CreateMissing` and `default_for_load`

**Evidence:** Exactly one production caller passes `InitPolicy::CreateMissing` (`src/commands/registry/add.rs:40`); every other call site already uses `RequireExisting`. The optional trait method exists solely to feed that single caller, and its `None` arm is a live operator-path panic:

```text
src/commands/registry/add.rs:40:        with_state::<Registry, _, _>(ctx.layout(), InitPolicy::CreateMissing, move |registry| {
crates/domain/src/config/atomic.rs:29:    fn default_for_load() -> Option<Self> {
crates/domain/src/config/atomic.rs:55:pub enum InitPolicy {
crates/domain/src/config/atomic.rs:100:        (None, InitPolicy::CreateMissing) => S::default_for_load().expect(
crates/domain/src/config/atomic.rs:121:    fn default_for_load() -> Option<Self> {  // Registry
crates/domain/src/config/atomic.rs:149:    fn default_for_load() -> Option<Self> {  // ProjectConfig, returns None
crates/domain/src/change/plan/core/io.rs:24:    fn default_for_load() -> Option<Self> {  // Plan, returns None
```

`InitPolicy::RequireExisting` is the only branch actually exercised by the seven non-Registry call sites under `src/commands/plan/*.rs`, `src/commands/slice/merge.rs`, and `src/commands/registry/remove.rs`.

**Action:**
1. Inline `Registry::load_or_default` semantics in `src/commands/registry/add.rs`: `let mut registry = Registry::load(&ctx.project_dir)?.unwrap_or_else(|| Registry { version: 1, projects: Vec::new() }); … yaml_write(&Registry::path(&ctx.project_dir), &registry)?;`. Keep the existing validate-shape gates inside the mutation block.
2. Delete `InitPolicy` outright. Change `with_state`'s signature to `fn with_state<S, B, F>(layout, missing_kind: &'static str, f: F)` and drop the match-on-policy in favour of a single `S::load(layout)?.ok_or_else(|| Error::ArtifactNotFound { kind: missing_kind, path })?`. Update the six remaining call sites from `InitPolicy::RequireExisting("plan.yaml")` to `"plan.yaml"`.
3. Delete `AtomicYaml::default_for_load` (trait default + Registry / ProjectConfig / Plan impls).
4. Delete the `with_state_creates_default_when_absent` test and adjust `with_state_propagates_closure_error_and_skips_write` to seed `registry.yaml` first (or drop it; the closure-error path is already covered by integration tests).

**Quality delta:** `~-50 LOC, -1 enum, -1 enum variant, -1 trait method, -1 production-path expect() panic, -2 match arms`.

**Net LOC:** `crates/domain/src/config/atomic.rs` + `crates/domain/src/change/plan/core/io.rs` + `src/commands/registry/add.rs` **~447 → ~395**.

**Architectural impact:** `AtomicYaml` becomes a pure shape contract (`path` + `load`); creation policy stops being an interface concern and moves to the one caller that needs it. Cargo's `git2::Config` follows the same shape — load existing or fail, with creation explicit at the call site.

**Done when:** `rg -nF 'InitPolicy|default_for_load' crates/ src/` returns **0**, `rg -nF '\.expect\("AtomicYaml::load' crates/` returns **0**, and `cargo make check` passes.

**Rule?** no - one trait family, not a repo-wide pattern.

**Counter-argument:** The `CreateMissing` policy keeps `registry add` symmetric with the other mutation helpers. It loses because that symmetry costs an Option-typed trait method, a two-variant policy enum, an operator-path panic guard, and a dedicated unit test, all to factor out five lines of struct-literal construction in one handler.

**Depends on:** none.

### F3 - Inline plan/create dedup helpers and flatten the unknown-slice walk

**Evidence:** `src/commands/plan/create.rs:268-309` carries two single-call-site helpers that wrap `Iterator::collect` and a three-loop walk that re-tests the same membership predicate:

```text
src/commands/plan/create.rs:268:fn dedup_sets(sets: &[(String, ClaimKind, String)]) -> BTreeMap<(String, ClaimKind), String> {
src/commands/plan/create.rs:278:fn dedup_clears(clears: &[(String, ClaimKind)]) -> BTreeSet<(String, ClaimKind)> {
src/commands/plan/create.rs:289:fn refuse_unknown_slices(
src/commands/plan/create.rs:293:    let known: BTreeSet<&str> = plan.entries.iter().map(|e| e.name.as_str()).collect();
src/commands/plan/create.rs:294:    for (slice, _) in set_map.keys() { if !known.contains(slice.as_str()) { return Err(unknown_slice_err(plan_name, slice)); } }
src/commands/plan/create.rs:299:    for (slice, _) in clear_set       { if !known.contains(slice.as_str()) { return Err(unknown_slice_err(plan_name, slice)); } }
src/commands/plan/create.rs:304:    for slice in clear_all_set         { if !known.contains(slice.as_str()) { return Err(unknown_slice_err(plan_name, slice)); } }
```

Each of `dedup_sets` / `dedup_clears` has exactly one caller (`mutate_authority_overrides` at `:427-429`), and both bodies are one `iter().cloned().[map(…)].collect()`.

**Action:**
1. Inline the two helpers at `mutate_authority_overrides` (`:427`-`:429`): `let set_map: BTreeMap<_, _> = sets.iter().cloned().map(|(s, k, v)| ((s, k), v)).collect(); let clear_set: BTreeSet<_> = clears.iter().cloned().collect();`. Delete `dedup_sets` and `dedup_clears`.
2. Replace the three near-identical loops in `refuse_unknown_slices` with one chained iterator: `let unknown = set_map.keys().map(|(s, _)| s.as_str()).chain(clear_set.iter().map(|(s, _)| s.as_str())).chain(clear_all_set.iter().map(String::as_str)).find(|s| !known.contains(s)); if let Some(slice) = unknown { return Err(unknown_slice_err(plan_name, slice)); } Ok(())`.

**Quality delta:** `~-22 LOC, -2 helper fns, -1 branch cluster (three loops → one find)`.

**Net LOC:** `src/commands/plan/create.rs` **956 → ~934**.

**Done when:** `rg -nF 'fn dedup_sets|fn dedup_clears' src/commands/plan/create.rs` returns **0**, `rg -nF 'for (slice, _) in set_map.keys()' src/commands/plan/create.rs` returns **0**, and `cargo make check` passes.

**Rule?** no - localised to one handler.

**Counter-argument:** Named helpers document intent. They lose because each "helper" is a one-line iterator chain inlined at one site; the function name is longer than the body, and `refuse_unknown_slices`'s three-loop shape obscures a single `find`-on-chain.

**Depends on:** none.

## One-Touch Tidies

### T1 - Drop unused `_value_names` parameter on `parse_slice_pair_args`

**Evidence:** The helper carries a leading-underscore parameter that exists only to be ignored, and every caller pays the line of ceremony:

```text
src/commands/plan/create.rs:245:    raw: &[String], flag: &'static str, _value_names: &str,
src/commands/plan/create.rs:556:            "<slice> <kind>=<key>",
src/commands/plan/create.rs:760:            "<slice> <kind>=<key>",
src/commands/plan/create.rs:768:            "<slice> <kind>",
```

**Action:**
1. Remove the `_value_names: &str` parameter from `parse_slice_pair_args` (`:244-246`).
2. Drop the three `<slice> …` string arguments at `:556`, `:760`, `:768`.

**Quality delta:** `-4 LOC, -1 unused parameter, lower call-site burden`.

**Net LOC:** `src/commands/plan/create.rs` **956 → 952**.

**Done when:** `rg -nF '_value_names' src/commands/plan/create.rs` returns **0**, `rg -nF '"<slice> <kind>=<key>"' src/commands/plan/create.rs` returns **0**, and `cargo make check` passes.

**Rule?** no.

**Counter-argument:** The token documents the value-name shape at each invocation. It loses because the helper does nothing with the value-name string and the closed `T::from_str` impl already shapes the diagnostic; the would-be documentation is dead bytes.

**Depends on:** none.
