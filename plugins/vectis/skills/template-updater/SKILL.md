---
name: template-updater
description: Fix Vectis CLI templates and version pins when upstream crate or tooling bumps break a freshly scaffolded project. Use when `vectis update-versions --verify` reports a failing cap-matrix combo, when a Crux/uniffi/Gradle release has introduced template drift, or when the user mentions template-updater.
---

# Vectis Template Updater

Close the loop on version bumps. When `vectis update-versions` proposes new
crate pins but the scratch scaffold produced from those pins no longer compiles
(or `cargo clippy --all-targets -- -D warnings`, `cargo deny check`, `cargo
vet`, `codegen swift`, `codegen kotlin`, iOS `xcodebuild`, or Android
`assembleDebug` fails), this skill diagnoses the breakage, edits the right
template files + template modules, and proves the fix by re-running the full
cap matrix.

The deterministic machinery (scaffold, verify, registry queries, atomic
writes) lives in `vectis` itself. What remains is judgement work: reading
compiler errors, mapping them to the upstream changelog, deciding whether the
fix is a template edit, a conditional in `crates/vectis-cli/src/templates/`,
an `embedded/versions.toml` pin tweak, or a parser update.

This skill is invoked after `vectis update-versions` proposes or writes new
pins. It never runs the version query itself -- that is the CLI's job.

## Arguments

| Argument | Required | Description |
|---|---|---|
| `repo-dir` | No | Path to the `specify` checkout. Defaults to the current working directory. |
| `version-file` | No | Path to the `versions.toml` carrying the proposed pins. Defaults to `crates/vectis-cli/embedded/versions.toml` (the embedded defaults). |
| `caps-matrix` | No | Comma-separated list of `--caps` strings to validate, joined with `\|`. Defaults to the same four combos `vectis update-versions --verify` uses: `""`, `"http"`, `"http,kv"`, `"http,kv,time,platform,sse"`. |
| `shells` | No | Shells to scaffold per combo during validation. Defaults to `""` (core-only, mirroring `update-versions --verify`). Use `"ios,android"` when a shell-specific breakage is suspected. |
| `scratch-dir` | No | Directory for scratch scaffolds. Defaults to `$HOME/.cache/vectis/template-updater-<pid>/`. |

## Inputs the skill relies on

- **Current embedded pins** at `{repo-dir}/crates/vectis-cli/embedded/versions.toml`.
- **Template files** at `{repo-dir}/templates/vectis/{core,ios,android}/`, with
  target-path mapping in each folder's `MANIFEST.md`.
- **Template engine + registries** at `{repo-dir}/crates/vectis-cli/src/templates/{mod.rs,core.rs,ios.rs,android.rs}` (placeholder chain, cap-conditional logic, per-file target paths).
- **Add-shell parser** at `{repo-dir}/crates/vectis-cli/src/add_shell/parser.rs` -- the only place where capability crate names (`crux_http`, `crux_http::sse`, `crux_kv`, `crux_time`, `crux_platform`) are hard-coded outside the templates. When a Crux bump renames a capability crate, this file must be edited in lockstep with the `app.rs` template.
- **Verify pipeline** at `{repo-dir}/crates/vectis-cli/src/verify/{core,ios,android}.rs` -- the ordered build/check steps per assembly. The JSON emitted by `vectis verify` lists each step by name; failures include the first N lines of combined stdout/stderr and are the primary signal this skill works from.
- **Known drift backlog** at [`references/known-drift.md`](references/known-drift.md) -- the running list of deferred items from chunk 11/12 verification. Start here before diagnosing a new bump; the odds are non-trivial that the failure is one of these.

## Prerequisites

Before starting, make sure:

1. The repo's working tree is clean. `git status` shows no unstaged changes.
   This skill makes small, verifiable edits; mixing them with unrelated WIP
   will cause the validation matrix to attribute unrelated failures to the
   bump.
2. `cargo build --release -p vectis-cli` has been run and `./target/release/vectis --help` works. The skill invokes the release binary throughout.
3. Platform prerequisites for each shell being validated (`xcodegen`, `cargo-swift ≥ 0.10`, `$ANDROID_HOME` with NDK, Gradle 8.x on `PATH`). Re-run `vectis verify` on a known-good scaffold if unsure -- the skill's first action must not be to paper over a missing prereq.

---

## Process

The skill runs the five-step RFC flow (Detect → Diagnose → Update → Validate → Report). Steps D1--D5 below correspond one-to-one. Each edit the skill makes to the repo is scoped to `templates/vectis/**`, `crates/vectis-cli/src/templates/**`, `crates/vectis-cli/src/add_shell/parser.rs`, `crates/vectis-cli/embedded/versions.toml`, and (when a new upstream advisory appears) `templates/vectis/core/deny.toml`. All other paths are off-limits -- nothing under `crates/vectis-cli/src/{init,verify,update_versions,prerequisites,main}.rs` should be touched by this skill (those are the CLI's orchestration, not its templates).

### D1. Detect breakage

1. Record the baseline. Capture the output of `./target/release/vectis update-versions --dry-run --verify --version-file {version-file}` to `{scratch-dir}/baseline.json`. The `verification.combos[]` array tells you which combos fail and on which step.
2. For each failing combo, re-scaffold deterministically so the diagnosis phase has a reproducible scratch project:

   ```bash
   dir={scratch-dir}/combo-<N>
   rm -rf "$dir"
   ./target/release/vectis init ScratchApp \
     --dir "$dir" \
     --caps "<caps-combo>" \
     --shells "<shells>" \
     --version-file {version-file}
   ./target/release/vectis verify --dir "$dir" > "$dir/verify.json" 2>&1 || true
   ```

   Only re-run the combos that failed in step 1; passing combos are a regression gate that runs unchanged at D4.
3. Parse each `verify.json`. The first `assemblies.*.steps[]` entry with `passed: false` is the root failure for that combo. Record the assembly, step name, and error snippet.

### D2. Diagnose the failure

For each distinct error the skill collected in D1, pick one of these paths and
commit to it before starting to edit. Do not speculatively rewrite templates
that are not covered by at least one reproduced failure.

1. **Look it up first.** Check [`references/known-drift.md`](references/known-drift.md). If the failure matches a listed item (uniffi/cargo-swift decoupling, AGP 9.x + Gradle 9.x, new RUSTSEC advisories in the full-caps combo, `facet_generate` req-string cosmetic), follow the playbook there -- it is already tied to a concrete fix path.
2. **Match the error to the upstream crate.** `unresolved module path shared::ffi` / `cannot find type RustBuffer in scope` → uniffi or cargo-swift. `error[E0432]: unresolved import crux_core::Render` → crux_core rename or capability-crate rename. `RUSTSEC-YYYY-NNNN` in a `cargo deny check` failure → supply-chain advisory. `setFileMode(Integer)` in a Gradle trace → rust-android-gradle vs Gradle major. `unresolved reference: something` under Kotlin → Android library API drift (Compose BOM, Koin, Ktor).
3. **Read the relevant changelog.** Prefer upstream release notes over crates.io's rendered README (they are more reliably updated). Canonical sources: `https://github.com/redbadger/crux/releases` (crux_*), `https://github.com/mozilla/uniffi-rs/blob/main/CHANGELOG.md` (uniffi / uniffi_bindgen), `https://github.com/mozilla/rust-android-gradle/releases` (rust-android-gradle), `https://github.com/rustsec/advisory-db/` (RUSTSEC). Use the WebFetch tool.
4. **Map the changelog entry to a concrete file or module.** The table below covers the common rename-class breakages. If the failure is structural (e.g. uniffi 0.31 decoupling from crux_core::cli::bindgen), see [`references/known-drift.md`](references/known-drift.md) for the pre-scoped rewrite task.

| Symptom | Likely fix site |
|---|---|
| `unresolved import crux_*` or renamed `Effect::*` / `Event::*` variant | `templates/vectis/core/app.rs` (type aliases, match arms); `templates/vectis/core/ffi.rs` if the public FFI type renamed; `crates/vectis-cli/src/add_shell/parser.rs` (`classify_cap_path`) if a capability crate renamed |
| `cannot find type RustBuffer in scope` in generated Swift / `import sharedFFI` failing | `templates/vectis/core/codegen.rs` (when the bindgen call signature changes) or the per-developer `cargo-swift` install (per-machine prereq, not a template pin) |
| Kotlin `codegen kotlin` fails with `unresolved module path shared::ffi` | [`references/known-drift.md`](references/known-drift.md) §1 (`uniffi` / `crux_core::cli::bindgen` coupling) -- structural rewrite |
| `cargo deny check` `unmaintained`/`vulnerable` for a Crux transitive dep | `templates/vectis/core/deny.toml` `[advisories] ignore`; include a one-line rationale comment pointing at the transitive chain |
| `cargo clippy -- -D warnings` fires a new lint in render-only baseline | `templates/vectis/core/workspace-cargo.toml` (add a `#[allow(...)]` at lint group priority, or a scoped `#[allow]` in the specific template file). Prefer the scoped form unless the lint is project-wide. |
| AGP + Gradle `setFileMode(Integer)` trace | [`references/known-drift.md`](references/known-drift.md) §2 (AGP 9.x vs rust-android-gradle 0.9.6) -- pin max-version or drive upstream update |
| Android Kotlin unresolved reference after Compose BOM or Koin or Ktor bump | `templates/vectis/android/libs.versions.toml` (pin rollback / new artifact name) and/or `templates/vectis/android/Core.kt` (API call site inside `<<<CAP:http>>>` block) |
| iOS `xcodebuild` fails with missing type after cargo-swift bump | `templates/vectis/ios/Core.swift` or the per-cap Swift arm block; check the regenerated `shared.swift` in the scaffold to see the new bindgen output |

5. Write a one-line diagnosis per failure in a scratchpad. Do not combine
diagnoses from unrelated combos. If two combos fail for the same root cause
(e.g. the `sse` cap introduces a new RUSTSEC advisory in both the
`http,kv,time,platform,sse` combo and a future one), treat it as one fix.

### D3. Update templates / modules / pins

Apply the fix, making each edit as narrow as possible.

- Edit the **template file** when the change is visible in a scaffolded
  project (a renamed `use`, a new `#[allow(...)]`, a changed build flag, a
  new advisory in `deny.toml`). Every template file is `include_str!`-ed
  verbatim by `crates/vectis-cli/src/templates/{core,ios,android}.rs`, so
  editing the file is sufficient -- do not touch the module unless the set of
  files shipped, their predicates, or their target paths has changed.
- Edit the **template module** (`crates/vectis-cli/src/templates/core.rs` etc.) only when: a new file is shipped / removed; an existing file's target path changes; a file's `IncludeWhen::{Always, AnyOf(&[Capability])}` predicate changes; or a new placeholder is introduced and must be substituted. Respect the superstring-first substitution order (`__APP_NAME_LOWER__` before `__APP_NAME__`, `__ANDROID_PACKAGE_PATH__` before `__APP_NAME_LOWER__` in path segments). Any new template file must also be listed in the corresponding `templates/vectis/{core,ios,android}/MANIFEST.md`; the parity test `templates::core::tests::registry_matches_rfc_core_file_count` (and its iOS/Android siblings) enforces this.
- Edit `crates/vectis-cli/src/add_shell/parser.rs` when a capability crate is
  renamed upstream. The parser keys off the **RHS crate root**, not the alias
  name in the user's `app.rs`. Nested matches (e.g. `crux_http::sse::Sse`)
  must be tried **before** the bare-crate match, otherwise a capability's
  successor will be mis-tagged as the parent crate.
- Edit `crates/vectis-cli/embedded/versions.toml` when the fix is a pin bump.
  Preserve the multi-line rationale comments already in that file -- they
  capture the uniffi/cargo-swift + AGP/Gradle pairing rules and should only
  be edited when the rule itself changes, not when an ordinary pin moves.
- **Do not** edit anything under
  `crates/vectis-cli/src/{init,verify,update_versions,prerequisites,main}.rs`
  from this skill. If the fix requires orchestration changes, stop and flag
  it -- that is a CLI change, not a template update, and belongs in a new
  RFC-6 chunk follow-up.

After each atomic edit:

1. Re-run the single combo that reproduced the failure:

   ```bash
   cargo build --release -p vectis-cli   # only if you touched vectis-cli source
   rm -rf "{scratch-dir}/combo-<N>"
   ./target/release/vectis init ScratchApp --dir "{scratch-dir}/combo-<N>" \
     --caps "<caps>" --shells "<shells>" --version-file {version-file}
   ./target/release/vectis verify --dir "{scratch-dir}/combo-<N>"
   ```
2. If the step that was failing now passes, move on. If a *later* step now
   fails, keep the edit and continue diagnosing -- do not revert unless the
   later failure is clearly caused by your change (not merely unmasked by it).
3. If the fix required an `embedded/versions.toml` bump, also run
   `cargo test -p vectis-cli` -- the embedded-defaults unit tests pin a
   snapshot of the versions file that will need updating in the same commit.

### D4. Validate the full cap matrix

Once every failing combo passes individually, run the whole matrix to
catch unintended regressions:

```bash
./target/release/vectis update-versions --dry-run --verify --version-file {version-file}
```

The output must show `verification.passed: true` and every entry in
`verification.combos[].passed` must be `true`. If a previously-passing combo
now fails, the fix in D3 was too broad -- narrow it (prefer scoped `#[allow]`,
an additional cap-conditional branch, or a predicate refinement over a
template-wide change).

If `shells` was set to `"ios,android"` in the arguments, also run the shell
matrix combo-by-combo for the caps that include those shells -- `update-versions --verify` is core-only by design; shell regressions surface at this step only.

Finally, re-run the repo's own gates:

```bash
cargo build --release -p vectis-cli
cargo clippy --release -p vectis-cli --all-targets -- -D warnings
cargo test -p vectis-cli
make checks
```

All four must be green before the fix is considered valid.

### D5. Report

Produce a structured report in Markdown with these sections. The orchestrator
(`/vectis:template-updater`) copies this verbatim into the commit message or
PR body.

1. **Trigger** -- what bump prompted the run, with the specific pin diffs
   from `update-versions --dry-run`.
2. **Failures reproduced** -- one bullet per combo × step that failed, with
   the first line of the compiler/linker/cargo error.
3. **Diagnoses** -- one paragraph per distinct root cause, citing the
   upstream changelog entry or RUSTSEC advisory that motivates the fix.
4. **Changes** -- file-by-file list of edits with a one-line rationale per
   file. Cite the edit using the repo-relative path; do not paste large
   diffs (the commit itself carries that).
5. **Verification** -- confirmation that each failing combo now passes, the
   full matrix is green, and `make checks` + `cargo test -p vectis-cli` both
   passed. Include the final `vectis update-versions --dry-run --verify`
   JSON's `verification.passed` line.
6. **Known drift still unresolved** -- anything listed in
   [`references/known-drift.md`](references/known-drift.md) that the
   current bump did not exercise and therefore was not fixed. Do not
   invent new items; promote a known item out of the backlog only when a
   reproduced failure in this run proves it is fixed.

---

## Worked example -- `crux_core 0.17 → 0.18` renames `Effect::Render` to `Effect::View`

This example walks the five-step flow for a hypothetical, mechanical rename.

**D1. Detect.** `vectis update-versions --dry-run` proposes `crux_core: 0.17.0 → 0.18.0`. `vectis update-versions --dry-run --verify --version-file /tmp/proposed.toml` reports:

```json
{ "verification": { "passed": false, "combos": [
  { "caps": "", "passed": false, "verify": { "assemblies": { "core": {
    "passed": false,
    "steps": [{"name": "cargo check", "passed": false,
      "error": "error[E0432]: unresolved import `crux_core::render::Render`\n  ..."}]
  }}}}
] } }
```

All four combos fail on the same `cargo check` step; only render-only is needed to diagnose.

**D2. Diagnose.** `references/known-drift.md` does not list this. The symptom is "renamed `crux_core` export". Fetch `https://github.com/redbadger/crux/releases/tag/crux_core-0.18.0`: the changelog entry says `Render` has been renamed to `View` for consistency with the new capability taxonomy. The render-only baseline's `Effect` enum in `templates/vectis/core/app.rs` imports `Render` and uses it in the `Effect::Render(RenderOperation)` variant; the iOS `processEffect` switch matches on `.render`; the Android switch matches on `Effect.Render`. Three file touch-points; no parser change (Render is a capability but its crate root is still `crux_core::render` -- just the trait/type renamed within it).

**D3. Update.** Edits (each atomic, each verified before the next):

- `templates/vectis/core/app.rs`: `use crux_core::render::Render` → `use crux_core::render::View`; `Render` type alias (if present) → `View`; `Effect::Render(_)` cap-marker arm → `Effect::View(_)`. Run `vectis init ScratchApp --dir /tmp/combo-0 --caps "" --version-file /tmp/proposed.toml && vectis verify --dir /tmp/combo-0` -- `cargo check` now passes; `codegen swift` now fails because `Core.swift`'s `.render` arm is stale.
- `templates/vectis/ios/Core.swift`: `case .render:` → `case .view:`. Re-run verify -- iOS passes.
- `templates/vectis/android/Core.kt`: `is Effect.Render ->` → `is Effect.View ->`. Re-run verify -- Android passes.

No template-module edit (no new files, no new placeholders, no predicate change). No parser edit (`classify_cap_path` matches `crux_core::render` as a whole-crate-level concern which survives the in-crate rename). No `embedded/versions.toml` edit other than the new `crux_core` pin itself (which `update-versions` writes, not this skill).

**D4. Validate.** `vectis update-versions --dry-run --verify --version-file /tmp/proposed.toml` now reports `verification.passed: true` with all four combos green. `cargo test -p vectis-cli` passes (the embedded-defaults snapshot was not touched -- this was a pin the CLI *proposes*, not one it *embeds*, until the user actually runs `update-versions` without `--dry-run`). `make checks` passes.

**D5. Report.** One paragraph summarising the rename, a three-bullet list of template edits, and the confirmation that all combos pass. The commit message subject is `templates: crux_core 0.18.0 rename Render → View`.

---

## Anti-patterns (do not do these)

- **Editing verify or init orchestration.** If a cap's pipeline "should" pick up a new step, the edit belongs in `crates/vectis-cli/src/verify/*.rs`. That is a CLI change that needs its own RFC-6 chunk; this skill must stop and flag it rather than silently expand scope.
- **Silencing a new advisory without understanding it.** `RUSTSEC-*` IDs added to `templates/vectis/core/deny.toml`'s `[advisories] ignore` list must have (a) a rationale comment naming the transitive chain that forces the advisory, and (b) no known safe upgrade path. A one-line `# upstream unmaintained` with no chain is not acceptable.
- **Speculative rewrites.** Do not edit a template file that is not covered by at least one reproduced failure. Templates that look "stylistically outdated" are out of scope for this skill -- they belong in a separate refactor.
- **Changing the placeholder order.** `templates::mod.rs::substitute_placeholders` substitutes superstrings first (`__APP_NAME_LOWER__` before `__APP_NAME__`; `__ANDROID_PACKAGE_PATH__` before `__APP_NAME_LOWER__`). Adding a new placeholder always means slotting it into this chain in superstring-first order -- never appending.
- **Dropping an existing `#[allow(...)]` on a capability type alias or `update()` match.** The render-only baseline intentionally carries `#[allow(dead_code)]` on capability `type` aliases and `#[allow(clippy::match_same_arms)]` on `update()`. These are the writer-skill's handover contract from chunk 12; do not touch them unless the writer skills have been updated in lockstep.

## Reference documentation

| File | Purpose |
|---|---|
| [`references/known-drift.md`](references/known-drift.md) | The running backlog of deferred fix items from chunks 11/12. Always consult before diagnosing. |
| `crates/vectis-cli/embedded/versions.toml` | Embedded default pins plus the multi-line rationale comments explaining the uniffi/cargo-swift and AGP/Gradle pairing rules. |
| `templates/vectis/{core,ios,android}/MANIFEST.md` | Source→target path mapping per template. The registry tests pin file counts; update the manifest when you add or remove a template file. |
| `crates/vectis-cli/src/templates/{mod.rs,core.rs,ios.rs,android.rs}` | Template engine (placeholder substitution, cap-conditional markers, path substitution for `__APP_NAME__` / `__APP_NAME_LOWER__` / `__ANDROID_PACKAGE_PATH__`) and per-assembly file registries. |
| `crates/vectis-cli/src/add_shell/parser.rs` | AST classifier for capability crates. Must be updated in lockstep with any Crux capability-crate rename. |
| `crates/vectis-cli/src/verify/{core,ios,android}.rs` | The ordered build/check steps this skill's Detect phase interprets. Do not edit from this skill. |
| `rfcs/rfc-6-vectis-bootstrap.md` § Template Maintenance | The RFC's narrative motivation for this skill. |
