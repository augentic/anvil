# Code & Skill Review - specify + specify-cli

Top three findings by tier: **F1 Fix init wire drift** (verified wire-contract defect), **F2 Delete package-client trait wrapper** (subtraction plus one operator-path panic removal), **F3 Delete `VersionMode::Preserve`** (dead/test-only init branch).
Total ΔLOC if all land: **approximately -200 LOC**.
Primary non-LOC axes moved: fewer traits/types, fewer branch guards, lower panic surface, lower prompt/call-site burden, fewer duplicate documentation sources.
Top verified defects closed: **2 qualified** (`--hub`/init error documentation drift, `FIRST_PARTY_REGISTRY.parse().expect(...)` on the tool-fetch path). Defect-only net ΔLOC: **0**.
Most likely to break in remediation: **F2** - it touches the resolver test injection path and package fetch runtime boundary.

## Reconnaissance

- `tokei`:
  - `specify`: **648 files**, **87,383 total lines**; Markdown **515 files / 49,801 lines**.
  - `specify-cli`: **446 files**, **64,833 total lines**; Rust **245 files / 47,731 lines**.
- `cargo tree --duplicates` (`specify-cli`): non-empty. First visible duplicate families included `base64 v0.21.7` / `v0.22.1`, `reqwest v0.12.28` / `v0.13.3`, `thiserror v1.0.69` / `v2.0.18`, and `strum v0.27.2` / `v0.28.0`; dominated by `wasmtime` / `wasm-pkg-client` transitives. `Cargo.toml` is frozen for this pass.
- `rg -c '^#\[test\]' crates/ src/ tests/` (`specify-cli`): **512** test functions.
- `rg --files -g '**/mod.rs'` (`specify-cli`): **3** files - `tests/common/mod.rs`, `wasi-tools/vectis/tests/engine_support/mod.rs`, `crates/domain/tests/common/mod.rs`.
- `wc -l docs/standards/*.md AGENTS.md`:
  - `specify`: **556 total**.
  - `specify-cli`: **638 total**.
- Files >500 lines under `crates/` and `src/` (`specify-cli`):
  - Tests: `crates/domain/tests/workspace.rs` **1048**, `crates/domain/tests/finalize.rs` **947**, `crates/domain/tests/registry.rs` **922**, `crates/domain/src/change/plan/core/validate/tests.rs` **594**.
  - Source: `src/commands/plan/create.rs` **966**, `crates/domain/src/discovery/document.rs` **891**, `crates/domain/src/slice/fusion.rs` **839**, `crates/domain/src/adapter/core.rs` **709**, `crates/domain/src/change/plan/core/model.rs` **629**, `crates/domain/src/spec/provenance.rs` **607**, `crates/domain/src/journal.rs` **595**, `crates/tool/src/validate.rs` **520**, `crates/domain/src/adapter/cache/io.rs` **509**.
- `make checks` (`specify`): **passed** - `All checks passed.` Total failures: **0**; first five predicate ids: **none**.
- `cargo make check` (`specify-cli`): **passed** - `Build Done in 172.19 seconds.` First error: **none**.
- `rg -c '\.(unwrap|expect)\(' --glob '!**/tests/**' crates/ src/` (`specify-cli`): summed **701** matching lines.
- `rg -c 'panic!|unreachable!' --glob '!**/tests/**' crates/ src/` (`specify-cli`): summed **48** matching lines.

## Structural Findings

### F1 - Fix init wire drift

**Evidence:** `specify-cli/DECISIONS.md:120-123` pins `specify init` to the `<adapter>` xor `--hub` contract and says the historical `init-requires-adapter-or-hub` envelope is gone from the CLI surface. The shipped clap surface matches that: `specify-cli/src/cli.rs:50-70` documents `--hub`, `conflicts_with = "hub"`, and `required_unless_present = "hub"`.

Current-state grep:

```text
docs/reference/quick-reference.md:21:run `specify init --workspace`
docs/reference/quick-reference.md:70:specify init --workspace
docs/reference/slice-skills/init.md:30:.specify/.cache/<adapter>/
docs/reference/slice-skills/init.md:52:init-requires-adapter-or-hub
docs/explanation/layered-stack.md:26:specify init --workspace
plugins/spec/rules/spec.mdc:44:specify init --workspace
docs/reference/configuration.md:19:workspace shape (`specify init --workspace`)
docs/reference/configuration.md:59:workspace shape
plugins/spec/skills/init/SKILL.md:23:init-requires-adapter-or-hub
```

**Action:**
1. Replace `specify init --workspace` with `specify init --hub` in `docs/reference/quick-reference.md`, `docs/explanation/layered-stack.md`, `docs/reference/configuration.md`, and `plugins/spec/rules/spec.mdc`.
2. Rename "workspace shape" prose in `docs/reference/configuration.md` to "hub shape" / "registry-only platform hub".
3. Replace `init-requires-adapter-or-hub` in `plugins/spec/skills/init/SKILL.md` and `docs/reference/slice-skills/init.md` with the clap parse-error contract: neither/both exits `2`.
4. Correct the adapter cache path in `docs/reference/slice-skills/init.md` from `.specify/.cache/<adapter>/` to `.specify/.cache/manifests/targets/<adapter>/`.

**Quality delta:** `0 LOC, -1 wire-contract defect cluster, -9 stale init-contract hits, -1 wrong cache-path claim`.

**Net LOC:** affected docs/skill/rule lines **9 stale lines -> 9 corrected lines**.

**Done when:** `rg -n 'specify init --workspace|mutually exclusive with `--workspace`|init-requires-adapter-or-hub|\.specify/\.cache/<adapter>|workspace shape' docs/reference/slice-skills/init.md docs/reference/configuration.md docs/reference/quick-reference.md docs/explanation/layered-stack.md docs/contributing/skills-test-coverage.md plugins/spec/skills/init/SKILL.md plugins/spec/rules/spec.mdc` returns **0**, and `make checks` still prints `All checks passed.`

**Rule?** no - this is an active vocabulary transition, and the prompt explicitly forbids new predicates in this pass.

**Counter-argument:** "Workspace" is still the operator concept for `.specify/workspace/` clones. It loses because `init --workspace` is not a shipped CLI flag; the setup topology is `--hub`, and the workspace noun belongs later under `specify workspace *`.

**Depends on:** none.

### F2 - Delete package-client trait wrapper

**Evidence:** `crates/tool/src/package.rs:87-124` declares `PackageClient`, `WasmPkgClient`, and the only production impl. `crates/tool/src/resolver.rs:224-238` then declares `ClosurePackageClient` solely so tests can pass a closure through the trait. The same file also has a verified operator-path panic surface at `crates/tool/src/package.rs:151-154`:

```text
FIRST_PARTY_REGISTRY.parse().expect("FIRST_PARTY_REGISTRY parses as a Registry")
```

Current-state grep:

```text
crates/tool/src/package.rs:87:pub trait PackageClient
crates/tool/src/package.rs:105:pub struct WasmPkgClient
crates/tool/src/package.rs:117:impl PackageClient for WasmPkgClient
crates/tool/src/package.rs:153:FIRST_PARTY_REGISTRY.parse().expect(...)
crates/tool/src/resolver.rs:226:struct ClosurePackageClient<F>(F)
```

**Action:**
1. Replace `PackageClient` / `WasmPkgClient` with a package-fetch free function in `crates/tool/src/package.rs` that takes `project_dir`, `request`, and `dest_hint`, builds the current-thread runtime, and calls the existing async fetch.
2. Change `resolver::resolve_with` / `stage_and_install` / `acquire_source_bytes` to accept an `impl Fn(&PackageRequest, &Path) -> Result<AcquiredBytes, ToolError>` instead of `&impl PackageClient`.
3. Delete `ClosurePackageClient` and pass the existing test closure directly.
4. Add a small `first_party_registry(package)` helper and use it both where `unwrap_or_else` currently panics and where `load_config` already maps the parse error. This closes the panic while also deleting the duplicate parse/error construction.

**Quality delta:** `~-36 LOC, -1 trait, -1 struct, -1 test wrapper type, -1 operator-path panic surface, -3 trait-bound call sites`.

**Net LOC:** `crates/tool/src/package.rs` + `crates/tool/src/resolver.rs` **781 -> ~745**.

**Architectural impact:** The resolver stops pretending there is a pluggable package-client hierarchy; it keeps one production fetch function and one test closure, which is the smaller cargo/ripgrep-style shape until a second real implementation exists.

**Done when:** `rg 'trait PackageClient|struct WasmPkgClient|ClosurePackageClient|FIRST_PARTY_REGISTRY\.parse\(\)\.expect' crates/tool/src` returns **0**, and `cargo make check` passes.

**Rule?** no - one trait family, not a repeated repo pattern.

**Counter-argument:** The trait names the test seam explicitly. It loses because there is one production implementation and one closure adapter; the wrapper exists only to satisfy abstraction ceremony.

**Depends on:** none.

### F3 - Delete `VersionMode::Preserve`

**Evidence:** `crates/domain/src/init.rs:36-55` carries `InitOptions.version_mode` and the `VersionMode` enum. `src/commands/init.rs:26-33`, `crates/domain/src/init/regular.rs:67`, and `crates/domain/src/init/hub.rs:84` always pass `VersionMode::WriteCurrent` on production paths. `VersionMode::Preserve` appears only in the test at `crates/domain/src/init/regular.rs:280-303`.

Current-state grep:

```text
crates/domain/src/init.rs:37:pub version_mode: VersionMode
crates/domain/src/init.rs:49:pub enum VersionMode
crates/domain/src/init.rs:55:Preserve
crates/domain/src/init.rs:120:pub(crate) fn resolve_version(project_dir: &Path, mode: VersionMode)
crates/domain/src/init/regular.rs:297:version_mode: VersionMode::Preserve
src/commands/init.rs:31:version_mode: VersionMode::WriteCurrent
```

**Action:**
1. Delete `VersionMode` and `InitOptions.version_mode`.
2. Change `resolve_version(project_dir, mode)` to `resolve_version()` returning `env!("CARGO_PKG_VERSION").to_string()`, or inline the two call sites if smaller.
3. Remove `version_mode: VersionMode::WriteCurrent` fields from init call sites and test helpers.
4. Delete `preserve_mode_keeps_existing_pinned_version`.

**Quality delta:** `~-43 LOC, -1 enum, -1 struct field, -1 branch, -1 YAML read on an unreachable mode, -4 caller fields`.

**Net LOC:** `crates/domain/src/init.rs` + `crates/domain/src/init/regular.rs` + `crates/domain/src/init/hub.rs` + `src/commands/init.rs` **1052 -> ~1009**.

**Architectural impact:** Init has one shipped version policy pre-1.0: write the current binary floor. The preserve mode is not exposed by clap, so keeping it only makes tests exercise a product surface operators cannot use.

**Done when:** `rg 'VersionMode|version_mode|Preserve|WriteCurrent' crates/domain/src/init.rs crates/domain/src/init/regular.rs crates/domain/src/init/hub.rs src/commands/init.rs` returns **0**, and `cargo make check` passes.

**Rule?** no.

**Counter-argument:** Re-init preserving a hand-edited version floor is polite. It loses because pre-1.0 explicitly ignores compatibility/migration posture, and the current CLI cannot request the preserve branch anyway.

**Depends on:** none.

### F4 - Cut capture rule to pointers

**Evidence:** `plugins/capture/rules/capture.mdc` is **79 lines** and `alwaysApply: true`. It still says the capture plugin "migrates existing TypeScript services to production-grade Rust WASM components" and "Automate[s] large-scale migrations from TypeScript to Rust WASM" (`:10-12`), while `plugins/capture/README.md:3` says capture consumption lives in the `captures` source adapter and replay verification lives in Omnia build briefs. The rule then repeats generic TypeScript migration advice already owned by `code-typescript` / target briefs.

Current-state grep:

```text
79 plugins/capture/rules/capture.mdc
10:This plugin migrates existing TypeScript services to production-grade Rust WASM components...
12:**Purpose**: Automate large-scale migrations from TypeScript to Rust WASM...
44:### Known Challenges
66:### TypeScript Libraries with No Rust Equivalent
```

**Action:**
1. Keep the frontmatter.
2. Replace the body with a short pointer set: the capture plugin only runs `/capture:wiretapper`; static extraction is `adapters/sources/code-typescript`; runtime capture consumption is `adapters/sources/captures`; replay is in Omnia `build` briefs; the skill body owns operational steps.
3. Delete the architecture diagram, generic TypeScript analysis list, troubleshooting list, and "Rust equivalent" advice.

**Quality delta:** `~-60 LOC, -1 misleading always-applied prompt, -4 duplicated guidance sections, lower prompt/call-site burden`.

**Net LOC:** `plugins/capture/rules/capture.mdc` **79 -> <=20**.

**Architectural impact:** Always-applied plugin rules should route the model to the right source of truth, not restate migration policy that belongs in adapter briefs and target build references.

**Done when:** `wc -l plugins/capture/rules/capture.mdc` reports **<=20**, `rg 'migrates existing TypeScript services|production-grade Rust WASM components|Known Challenges|TypeScript Libraries with No Rust Equivalent' plugins/capture/rules/capture.mdc` returns **0**, and `make checks` passes.

**Rule?** no.

**Counter-argument:** The current rule gives the model useful context without opening references. It loses because the context is now stale in the one place loaded unconditionally; progressive disclosure through `SKILL.md` and adapter references is cheaper and less wrong.

**Depends on:** none.

### F5 - Delete capture plugin page

**Evidence:** `docs/reference/plugins/capture.md` is **53 lines** and duplicates `plugins/capture/README.md` (**21 lines**) plus `plugins/capture/skills/wiretapper/SKILL.md`. It already drifted: `docs/reference/plugins/capture.md:16-21` documents `/capture:wiretapper <legacy-dir> [app-name <name>]` and `--app-name`, but the skill frontmatter is positional `argument-hint: <legacy-dir> [app-name]` (`plugins/capture/skills/wiretapper/SKILL.md:4`).

Current-state grep:

```text
docs/SUMMARY.md:37:- [Capture](reference/plugins/capture.md)
docs/reference/plugins/capture.md:16:/capture:wiretapper <legacy-dir> [app-name <name>]
docs/reference/plugins/capture.md:21:- `--app-name` - Name for the captured wiretap file.
```

**Action:**
1. Delete `docs/reference/plugins/capture.md`.
2. Remove the `Capture` child page from `docs/SUMMARY.md`.
3. In `docs/reference/plugins/index.md`, either leave the Capture row unlinked or point it at `../../../plugins/capture/README.md`; do not keep a second synopsis page.

**Quality delta:** `~-54 LOC, -1 duplicate docs page, -1 argument-shape drift, -1 documentation source of truth`.

**Net LOC:** `docs/reference/plugins/capture.md` + `docs/SUMMARY.md` + `docs/reference/plugins/index.md` **177 -> ~123**.

**Architectural impact:** Plugin behavior should live with the shipped plugin (`README.md` + `SKILL.md`); a mdBook mirror adds another contract surface and has already gone stale.

**Done when:** `test ! -e docs/reference/plugins/capture.md`, `rg 'reference/plugins/capture.md|app-name <name>|--app-name' docs/reference/plugins docs/SUMMARY.md` returns **0**, and `make checks` passes.

**Rule?** no.

**Counter-argument:** The reference section should have a page for every plugin. It loses because this page is not authoritative and duplicates the plugin artifact users actually install.

**Depends on:** none.

### F6 - Delete impossible odd-arg guard

**Evidence:** `src/commands/plan/cli.rs:57-63`, `:186-192`, and `:200-206` declare every call-site feeding `parse_slice_pair_args` with `num_args = 2`. The helper still carries an odd-length branch at `src/commands/plan/create.rs:238-258` while its own comment says clap prevents it in practice.

Current-state span:

```text
src/commands/plan/create.rs:241:clap's own `num_args = 2` guard prevents
src/commands/plan/create.rs:254:if !raw.len().is_multiple_of(2) {
src/commands/plan/create.rs:257:detail: format!("{flag} expects {value_names}; got an odd number...
```

**Action:**
1. Delete the `if !raw.len().is_multiple_of(2)` block from `parse_slice_pair_args`.
2. Keep the empty-slice validation and typed `T::from_str` error mapping.

**Quality delta:** `~-7 LOC, -1 impossible branch, lower handler noise`.

**Net LOC:** `src/commands/plan/create.rs` **966 -> ~959**.

**Done when:** `rg 'odd number of positional values|future surface changes|is_multiple_of\(2\)' src/commands/plan/create.rs` returns **0**, and `cargo make check` passes.

**Rule?** no.

**Counter-argument:** The guard protects future maintainers if they loosen clap. It loses because the present helper has exactly `num_args = 2` callers; future looseners can re-add validation with the new contract in hand.

**Depends on:** none.

## One-Touch Tidies

None. Everything below the structural bar either failed the quality-axis test or wanted a broader cleanup than this pass allows.

## Post-mortem

- F1 (init wire drift): actual ΔLOC -1 vs predicted 0; done when clean; regressions none.
- F2 (package-client trait wrapper): actual ΔLOC -36 vs predicted -36; done when clean; regressions none.
- F3 (VersionMode::Preserve): actual ΔLOC -64 vs predicted -43; done when clean; regressions none.
- F4 (capture rule pointers): actual ΔLOC -67 vs predicted -60; done when clean; regressions none.
- F5 (capture plugin page): actual ΔLOC -54 vs predicted -54; done when clean; regressions none.
- F6 (impossible odd-arg guard): actual ΔLOC -10 vs predicted -7; done when clean; regressions none.
