---
name: vectis-template-updater
description: Fix Vectis scaffold templates and version pins when upstream crate or tooling bumps break a fresh render. Use when Crux, uniffi, Gradle, Xcode, or Android SDK drift breaks the Vectis cap matrix, or when the user mentions template-updater.
argument-hint: "[cli-repo-dir]"
---

# Vectis Template Updater

Close the loop on version bumps. This skill owns the host workflow that used to be hidden behind the old bundled version verifier: inspect live registry data, draft a complete `versions.toml`, render scratch scaffolds with `specify tool run vectis -- scaffold`, run the host build matrix, diagnose drift, edit templates or embedded defaults, and prove the fix.

`vectis` (`scaffold`) is render-only. It may write the core, iOS, or Android template output using embedded defaults or an explicit complete `--version-file`, but it must not query registries, discover SDKs, build Cargo projects, bootstrap Gradle, run Xcode, or validate a cap matrix. Those steps are host-side judgement and command execution owned by this skill.

Version pins remain skill-readable data until another RFC creates a dedicated version tool. The canonical inputs today are the embedded defaults in `<specify-cli>/crates/vectis/embedded/versions.toml` plus an optional complete TOML override passed to `vectis scaffold --version-file`.

## Critical Path (Quick Reference)

1. Copy the embedded `versions.toml` to a scratch proposal file, query live registries from the host, and edit the proposal by hand.
2. Consult [`references/known-drift.md`](references/known-drift.md) before diagnosing a failure.
3. For each cap combo, create an empty scratch Specify project, render core and requested shells with `specify tool run vectis -- scaffold ... --version-file <proposal>`, then run explicit host build commands.
4. Map each reproduced failure to the narrowest template, template registry, capability map, deny-list, or embedded pin edit.
5. Re-run the failing combo immediately after each edit, then run the full cap matrix.
6. Report the proposed pins, failures, diagnoses, edits, verification commands, and unresolved known drift.

## Arguments

| Argument | Required | Description |
|---|---|---|
| `repo-dir` | No | Path to the `specify-cli` checkout where the Vectis templates and embedded defaults live. Defaults to the current working directory. |
| `version-file` | No | Path to a complete proposed `versions.toml`. Defaults to a scratch copy of `{repo-dir}/crates/vectis/embedded/versions.toml`; write proposals there before changing embedded defaults. |
| `caps-matrix` | No | Comma-separated list of `--caps` strings to validate, joined with `\|`. Defaults to `""`, `"http"`, `"http,kv"`, `"http,kv,time,platform,sse"`. |
| `shells` | No | Shells to scaffold per combo during validation. Defaults to `""` (core-only). Use `"ios,android"` when a shell-specific breakage is suspected. |
| `scratch-dir` | No | Directory for scratch scaffolds. Defaults to `$HOME/.cache/vectis/template-updater-<pid>/`. |

## Inputs the skill relies on

All paths below are rooted at the `specify-cli` checkout (`{repo-dir}`); the Vectis templates and render-only WASI crates live in that repo.

- **Current embedded pins** at `{repo-dir}/crates/vectis/embedded/versions.toml`.
- **Template files** at `{repo-dir}/templates/vectis/{core,ios,android}/`, with target-path mapping in each folder's `MANIFEST.md`.
- **Template engine + registries** at `{repo-dir}/crates/vectis/src/scaffold/templates.rs and {repo-dir}/crates/vectis/src/scaffold/templates/{core.rs,ios.rs,android.rs}` (placeholder chain, cap-conditional logic, per-file target paths).
- **Capability map** at `{repo-dir}/crates/vectis/src/scaffold/templates.rs` -- the active list of `--caps` tags and CAP-marker names. When a Crux bump renames or adds a capability, edit this module in lockstep with the `app.rs` template.
- **Host verify recipe** in this skill -- the ordered Cargo, codegen, deny/vet, Gradle, and Xcode commands that replaced the old bundled verify path. Failures from these commands are the primary signal this skill works from.
- **Known drift backlog** at [`references/known-drift.md`](references/known-drift.md) -- the running list of deferred items from chunk 11/12 verification. Start here before diagnosing a new bump; the odds are non-trivial that the failure is one of these.

## Prerequisites

Before starting, make sure:

1. The `specify-cli` working tree is clean. `git status` shows no unstaged changes. This skill makes small, verifiable edits; mixing them with unrelated WIP will cause the validation matrix to attribute unrelated failures to the bump.
2. `specify` is on `PATH` and supports `specify tool run`. The scratch projects must resolve the Vectis capability declaration that exposes `vectis` (`scaffold`).
3. If local WASI artifacts are under test, build or fetch them before the matrix run and point the scratch project's tool declaration at those component bytes. Do not execute a cached component directly; always run it through `specify tool run vectis -- scaffold`.
4. Platform prerequisites for each shell being validated are present (`xcodegen`, `cargo-swift`, `$ANDROID_HOME` with NDK, Gradle/Xcode as required). Confirm missing SDKs before editing templates; the skill's first action must not be to paper over a missing prereq.

---

## Process

The skill runs a five-step flow (Detect → Diagnose → Update → Validate → Report). Steps D1--D5 below correspond one-to-one. Each edit the skill makes to the repo is scoped to `{repo-dir}/templates/vectis/**`, `{repo-dir}/crates/vectis/src/scaffold/templates/**`, `{repo-dir}/crates/vectis/embedded/versions.toml`, and (when a new upstream advisory appears) `{repo-dir}/templates/vectis/core/deny.toml`. All other paths are off-limits -- especially host orchestration, `vectis` (`scaffold`) command behavior, and `specify` CLI entrypoints.

### D1. Detect breakage

1. Prepare a proposed pin file. If the operator did not supply `version-file`, copy `{repo-dir}/crates/vectis/embedded/versions.toml` to `{scratch-dir}/proposed-versions.toml`.
2. Query the live registries from the host and edit the proposed pin file directly:

   ```bash
   # Examples only; use the registry source that owns each coordinate.
   cargo search crux_core --limit 5
   cargo search uniffi --limit 5
   curl -fsSL "https://dl.google.com/dl/android/maven2/com/android/tools/build/gradle/maven-metadata.xml" \
     > "{scratch-dir}/agp-maven-metadata.xml"
   curl -fsSL "https://services.gradle.org/versions/all" \
     > "{scratch-dir}/gradle-versions.json"
   ```

   Record old → proposed values in `{scratch-dir}/pin-diff.md`. Registry queries are not WASI tools and are not performed by `vectis` (`scaffold`).
3. For each cap combo, scaffold deterministically so the diagnosis phase has a reproducible scratch project. Create an empty Specify project for each combo so `specify tool run` can resolve the Vectis tool declaration:

   ```bash
   dir={scratch-dir}/combo-<N>
   rm -rf "$dir"
   mkdir -p "$dir"
   cd "$dir"
   specify init https://github.com/augentic/specify/capabilities/vectis

   specify tool run vectis -- scaffold core ScratchApp \
     --caps "<caps-combo>" \
     --version-file "{version-file}" \
     > "$dir/scaffold-core.json"

   # Optional shell renders when the invocation requested shells.
   specify tool run vectis -- scaffold ios ScratchApp \
     --caps "<caps-combo>" \
     --version-file "{version-file}" \
     > "$dir/scaffold-ios.json"
   specify tool run vectis -- scaffold android ScratchApp \
     --caps "<caps-combo>" \
     --android-package com.vectis.scratchapp \
     --version-file "{version-file}" \
     > "$dir/scaffold-android.json"
   ```

   Omit the shell render blocks unless `shells` includes that target.
4. Run the host verify recipe in the scratch project and record logs by combo and step:

   ```bash
   cd "$dir"
   cargo check --workspace > "$dir/cargo-check.log" 2>&1
   cargo test --workspace > "$dir/cargo-test.log" 2>&1
   cargo clippy --all-targets -- -D warnings > "$dir/cargo-clippy.log" 2>&1
   cargo deny check > "$dir/cargo-deny.log" 2>&1
   cargo vet > "$dir/cargo-vet.log" 2>&1
   cargo run -p shared --bin codegen -- kotlin > "$dir/codegen-kotlin.log" 2>&1
   cargo run -p shared --bin codegen -- swift > "$dir/codegen-swift.log" 2>&1
   ```

   When shells are rendered, add the platform steps the templates expose (`make xcode` / `xcodebuild` for iOS and Gradle wrapper bootstrap / `assembleDebug` for Android). These commands are intentionally host workflow steps, not `vectis` (`scaffold`) behavior.
5. Record the first failing command per combo in `{scratch-dir}/failures.md` with the cap string, assembly, command, exit code, and first actionable error line.

### D2. Diagnose the failure

For each distinct error the skill collected in D1, pick one of these paths and commit to it before starting to edit. Do not speculatively rewrite templates that are not covered by at least one reproduced failure.

1. **Look it up first.** Check [`references/known-drift.md`](references/known-drift.md). If the failure matches a listed item (uniffi/cargo-swift decoupling, AGP 9.x + Gradle 9.x, new RUSTSEC advisories in the full-caps combo, `facet_generate` req-string cosmetic), follow the playbook there -- it is already tied to a concrete fix path.
2. **Match the error to the upstream crate.** `unresolved module path shared::ffi` / `cannot find type RustBuffer in scope` → uniffi or cargo-swift. `error[E0432]: unresolved import crux_core::Render` → crux_core rename or capability-crate rename. `RUSTSEC-YYYY-NNNN` in a `cargo deny check` failure → supply-chain advisory. `setFileMode(Integer)` in a Gradle trace → rust-android-gradle vs Gradle major. `unresolved reference: something` under Kotlin → Android library API drift (Compose BOM, Koin, Ktor).
3. **Read the relevant changelog.** Prefer upstream release notes over crates.io's rendered README (they are more reliably updated). Canonical sources: `https://github.com/redbadger/crux/releases` (crux_*), `https://github.com/mozilla/uniffi-rs/blob/main/CHANGELOG.md` (uniffi / uniffi_bindgen), `https://github.com/mozilla/rust-android-gradle/releases` (rust-android-gradle), `https://github.com/rustsec/advisory-db/` (RUSTSEC). Use the WebFetch tool.
4. **Map the changelog entry to a concrete file or module.** The table below covers the common rename-class breakages. If the failure is structural (e.g. uniffi 0.31 decoupling from crux_core::cli::bindgen), see [`references/known-drift.md`](references/known-drift.md) for the pre-scoped rewrite task.

| Symptom | Likely fix site |
|---|---|
| `unresolved import crux_*` or renamed `Effect::*` / `Event::*` variant | `<specify-cli>/templates/vectis/core/app.rs` (type aliases, match arms); `<specify-cli>/templates/vectis/core/ffi.rs` if the public FFI type renamed; `<specify-cli>/crates/vectis/src/scaffold/templates.rs` if a capability tag or CAP marker changed |
| `cannot find type RustBuffer in scope` in generated Swift / `import sharedFFI` failing | `<specify-cli>/templates/vectis/core/codegen.rs` (when the bindgen call signature changes) or the per-developer `cargo-swift` install (per-machine prereq, not a template pin) |
| Kotlin `codegen kotlin` fails with `unresolved module path shared::ffi` | [`references/known-drift.md`](references/known-drift.md) §1 (`uniffi` / `crux_core::cli::bindgen` coupling) -- structural rewrite |
| `cargo deny check` `unmaintained`/`vulnerable` for a Crux transitive dep | `<specify-cli>/templates/vectis/core/deny.toml` `[advisories] ignore`; include a one-line rationale comment pointing at the transitive chain |
| `cargo clippy -- -D warnings` fires a new lint in render-only baseline | `<specify-cli>/templates/vectis/core/workspace-cargo.toml` (add a `#[allow(...)]` at lint group priority, or a scoped `#[allow]` in the specific template file). Prefer the scoped form unless the lint is project-wide. |
| AGP + Gradle `setFileMode(Integer)` trace | [`references/known-drift.md`](references/known-drift.md) §2 (AGP 9.x vs rust-android-gradle 0.9.6) -- pin max-version or drive upstream update |
| Android Kotlin unresolved reference after Compose BOM or Koin or Ktor bump | `<specify-cli>/templates/vectis/android/libs.versions.toml` (pin rollback / new artifact name) and/or `<specify-cli>/templates/vectis/android/Core.kt` (API call site inside `<<<CAP:http>>>` block) |
| iOS `xcodebuild` fails with missing type after cargo-swift bump | `<specify-cli>/templates/vectis/ios/Core.swift` or the per-cap Swift arm block; check the regenerated `shared.swift` in the scaffold to see the new bindgen output |

5. Write a one-line diagnosis per failure in a scratchpad. Do not combine diagnoses from unrelated combos. If two combos fail for the same root cause (e.g. the `sse` cap introduces a new RUSTSEC advisory in both the `http,kv,time,platform,sse` combo and a future one), treat it as one fix.

### D3. Update templates / modules / pins

Apply the fix, making each edit as narrow as possible.

- Edit the **template file** when the slice is visible in a scaffolded project (a renamed `use`, a new `#[allow(...)]`, a changed build flag, a new advisory in `deny.toml`). Every template file is `include_str!`-ed verbatim by `<specify-cli>/crates/vectis/src/scaffold/templates/{core,ios,android}.rs`, so editing the file is sufficient -- do not touch the module unless the set of files shipped, their predicates, or their target paths has changed.
- Edit the **template module** (`<specify-cli>/crates/vectis/src/scaffold/templates/core.rs` etc.) only when: a new file is shipped / removed; an existing file's target path changes; a file's `IncludeWhen::{Always, AnyOf(&[Capability])}` predicate changes; a capability tag / CAP marker changes in `templates/mod.rs`; or a new placeholder is introduced and must be substituted. Respect the superstring-first substitution order (`__APP_NAME_LOWER__` before `__APP_NAME__`, `__ANDROID_PACKAGE_PATH__` before `__APP_NAME_LOWER__` in path segments). Any new template file must also be listed in the corresponding `<specify-cli>/templates/vectis/{core,ios,android}/MANIFEST.md`; the parity test `templates::core::tests::registry_matches_rfc_core_file_count` (and its iOS/Android siblings) enforces this.
- Edit `<specify-cli>/crates/vectis/embedded/versions.toml` when the accepted fix is a default pin bump. Preserve the multi-line rationale comments already in that file -- they capture the uniffi/cargo-swift + AGP/Gradle pairing rules and should only be edited when the rule itself changes, not when an ordinary pin moves. The scratch proposal remains a complete `--version-file` input until the fix is proven.
- **Do not** edit host orchestration or WASI command behavior from this skill (`<specify-cli>/crates/vectis/src/scaffold.rs`, `<specify-cli>/src/main.rs`, or any future helper that tries to rebuild registry query / verify orchestration). If the fix requires a new command surface, stop and flag it -- that is a separate CLI or WASI-tool change, not a template update.

After each atomic edit:

1. Re-run the single combo that reproduced the failure:

   ```bash
   rm -rf "{scratch-dir}/combo-<N>"
   # Repeat D1's scratch project bootstrap and `specify tool run vectis -- scaffold`
   # render for this combo, then rerun the exact host command that failed.
   ```
2. If the step that was failing now passes, move on. If a *later* step now fails, keep the edit and continue diagnosing -- do not revert unless the later failure is clearly caused by your change (not merely unmasked by it).
3. If the fix required an `embedded/versions.toml` bump, also run the `vectis` (`scaffold`) unit tests from `{repo-dir}` -- the embedded-defaults tests parse the versions file and must pass with the new defaults.

### D4. Validate the full cap matrix

Once every failing combo passes individually, run the whole matrix to catch unintended regressions (from `{repo-dir}`):

```bash
# Pseudocode: run the same explicit render + host verify flow for every combo.
for caps in "" "http" "http,kv" "http,kv,time,platform,sse"; do
  dir="{scratch-dir}/matrix-${caps//,/}-core"
  rm -rf "$dir"
  mkdir -p "$dir"
  cd "$dir"
  specify init https://github.com/augentic/specify/capabilities/vectis
  specify tool run vectis -- scaffold core ScratchApp \
    --caps "$caps" \
    --version-file "{version-file}"
  cargo check --workspace
  cargo test --workspace
  cargo clippy --all-targets -- -D warnings
  cargo deny check
  cargo vet
  cargo run -p shared --bin codegen -- kotlin
  cargo run -p shared --bin codegen -- swift
done
```

Every combo must pass. If a previously-passing combo now fails, the fix in D3 was too broad -- narrow it (prefer scoped `#[allow]`, an additional cap-conditional branch, or a predicate refinement over a template-wide change).

If `shells` was set to `"ios,android"` in the arguments, also run the shell matrix combo-by-combo for the caps that include those shells. Render each shell with `specify tool run vectis -- scaffold ios ...` or `-- android ...`, then run the host platform commands. Shell regressions surface here because scaffold remains render-only.

Finally, re-run the `specify-cli` repo's own gates from `{repo-dir}`:

```bash
cargo build -p specify-vectis --target wasm32-wasip2
cargo clippy -p specify -p specify-vectis --all-targets -- -D warnings
cargo test -p specify-vectis
cargo test --workspace
```

All four must be green before the fix is considered valid. (`{repo-dir}` is the `specify-cli` checkout; the integration tests around `specify tool run vectis -- scaffold` should exercise the same render-only path this skill drives.)

### D5. Report

Produce a structured report in Markdown with these sections. The orchestrator (`/vectis:template-updater`) copies this verbatim into the commit message or PR body.

1. **Trigger** -- what bump prompted the run, with the specific pin diffs from `{scratch-dir}/pin-diff.md`.
2. **Failures reproduced** -- one bullet per combo × step that failed, with the first line of the compiler/linker/cargo error.
3. **Diagnoses** -- one paragraph per distinct root cause, citing the upstream changelog entry or RUSTSEC advisory that motivates the fix.
4. **Changes** -- file-by-file list of edits with a one-line rationale per file. Cite the edit using the path relative to the `specify-cli` checkout; do not paste large diffs (the commit itself carries that).
5. **Verification** -- confirmation that each failing combo now passes, the full matrix is green, and `cargo test --workspace` + `cargo test -p specify-vectis` both passed. Include the exact `specify tool run vectis -- scaffold` and host build commands that made up the matrix.
6. **Known drift still unresolved** -- anything listed in [`references/known-drift.md`](references/known-drift.md) that the current bump did not exercise and therefore was not fixed. Do not invent new items; promote a known item out of the backlog only when a reproduced failure in this run proves it is fixed.

---

## Worked example -- `crux_core 0.17 → 0.18` renames `Effect::Render` to `Effect::View`

This example walks the five-step flow for a hypothetical, mechanical rename.

**D1. Detect.** Host registry inspection proposes `crux_core: 0.17.0 → 0.18.0` in `/tmp/proposed-versions.toml`. The full cap matrix renders via `specify tool run vectis -- scaffold core ScratchApp --caps <combo> --version-file /tmp/proposed-versions.toml`; every combo then fails the explicit host `cargo check --workspace` step with:

```text
error[E0432]: unresolved import `crux_core::render::Render`
  ...
```

All four combos fail on the same `cargo check` step; only render-only is needed to diagnose.

**D2. Diagnose.** `references/known-drift.md` does not list this. The symptom is "renamed `crux_core` export". Fetch `https://github.com/redbadger/crux/releases/tag/crux_core-0.18.0`: the changelog entry says `Render` has been renamed to `View` for consistency with the new capability taxonomy. The render-only baseline's `Effect` enum in `<specify-cli>/templates/vectis/core/app.rs` imports `Render` and uses it in the `Effect::Render(RenderOperation)` variant; the iOS `processEffect` switch matches on `.render`; the Android switch matches on `Effect.Render`. Three file touch-points; no parser change (Render is a capability but its crate root is still `crux_core::render` -- just the trait/type renamed within it).

**D3. Update.** Edits (each atomic, each verified before the next):

- `<specify-cli>/templates/vectis/core/app.rs`: `use crux_core::render::Render` → `use crux_core::render::View`; `Render` type alias (if present) → `View`; `Effect::Render(_)` cap-marker arm → `Effect::View(_)`. Re-render the failing combo with `specify tool run vectis -- scaffold core ScratchApp --caps "" --version-file /tmp/proposed-versions.toml` and rerun `cargo check --workspace`; it now passes, while `cargo run -p shared --bin codegen -- swift` exposes stale Swift effect names.
- `<specify-cli>/templates/vectis/ios/Core.swift`: `case .render:` → `case .view:`. Re-render iOS and run the iOS host build command; it passes.
- `<specify-cli>/templates/vectis/android/Core.kt`: `is Effect.Render ->` → `is Effect.View ->`. Re-render Android and run the Android host build command; it passes.

No template-module edit (no new files, no new placeholders, no predicate change). No parser edit (`classify_cap_path` matches `crux_core::render` as a whole-crate-level concern which survives the in-crate rename). The proposal file carries the new `crux_core` value until the full matrix proves it should become the embedded default.

**D4. Validate.** All four core combos render through `specify tool run vectis -- scaffold core ... --version-file /tmp/proposed-versions.toml` and pass the host Cargo/codegen/deny/vet commands. The shell matrix passes for iOS and Android when requested. `cargo test -p specify-vectis` and `cargo test --workspace` pass before promoting the proposed `crux_core` pin into the embedded defaults.

**D5. Report.** One paragraph summarising the rename, a three-bullet list of template edits, and the confirmation that all combos pass. The commit message subject is `templates: crux_core 0.18.0 rename Render → View`.

---

## Anti-patterns (do not do these)

- **Expanding `vectis` (`scaffold`) beyond render-only.** If a cap's pipeline "should" pick up a new registry query, SDK check, or build step, keep it in the host workflow or flag a separate RFC. Do not hide host behavior behind `specify tool run`.
- **Silencing a new advisory without understanding it.** `RUSTSEC-*` IDs added to `<specify-cli>/templates/vectis/core/deny.toml`'s `[advisories] ignore` list must have (a) a rationale comment naming the transitive chain that forces the advisory, and (b) no known safe upgrade path. A one-line `# upstream unmaintained` with no chain is not acceptable.
- **Speculative rewrites.** Do not edit a template file that is not covered by at least one reproduced failure. Templates that look "stylistically outdated" are out of scope for this skill -- they belong in a separate refactor.
- **Changing the placeholder order.** `templates::mod.rs::substitute_placeholders` substitutes superstrings first (`__APP_NAME_LOWER__` before `__APP_NAME__`; `__ANDROID_PACKAGE_PATH__` before `__APP_NAME_LOWER__`). Adding a new placeholder always means slotting it into this chain in superstring-first order -- never appending.
- **Dropping an existing `#[allow(...)]` on a capability type alias or `update()` match.** The render-only baseline intentionally carries `#[allow(dead_code)]` on capability `type` aliases and `#[allow(clippy::match_same_arms)]` on `update()`. These are the writer-skill's handover contract from chunk 12; do not touch them unless the writer skills have been updated in lockstep.

## Reference documentation

| File | Purpose |
|---|---|
| [`references/known-drift.md`](references/known-drift.md) | The running backlog of deferred fix items from chunks 11/12. Always consult before diagnosing. |
| `<specify-cli>/crates/vectis/embedded/versions.toml` | Embedded default pins plus the multi-line rationale comments explaining the uniffi/cargo-swift and AGP/Gradle pairing rules. |
| `<specify-cli>/templates/vectis/{core,ios,android}/MANIFEST.md` | Source→target path mapping per template. The registry tests pin file counts; update the manifest when you add or remove a template file. |
| `<specify-cli>/crates/vectis/src/scaffold/templates.rs and <specify-cli>/crates/vectis/src/scaffold/templates/{core.rs,ios.rs,android.rs}` | Template engine (placeholder substitution, cap-conditional markers, capability tags, path substitution for `__APP_NAME__` / `__APP_NAME_LOWER__` / `__ANDROID_PACKAGE_PATH__`) and per-assembly file registries. |
| `<specify-cli>/crates/vectis/src/scaffold.rs` | Render-only command contract, app-name validation, collision refusal, and `--version-file` parsing. Do not expand host behavior here from this skill. |
| `<specify-cli>/tests/vectis_tool.rs` | Integration tests for `specify tool run vectis -- scaffold` through the declared WASI tool path. |
| `rfcs/rfc-16-wasi-vectis.md` § Tool Scope | Current design narrative for keeping scaffold render-only and moving registry / matrix behavior into host-owned skill workflows. |
