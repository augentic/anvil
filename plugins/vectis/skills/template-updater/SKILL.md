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
- **Known drift backlog** at [`references/known-drift.md`](references/known-drift.md) -- the running list of deferred items from prior template verification. Start here before diagnosing a new bump; the odds are non-trivial that the failure is one of these.

## Prerequisites

Before starting, make sure:

1. The `specify-cli` working tree is clean. `git status` shows no unstaged changes. This skill makes small, verifiable edits; mixing them with unrelated WIP will cause the validation matrix to attribute unrelated failures to the bump.
2. `specify` is on `PATH` and supports `specify tool run`. The scratch projects must resolve the Vectis capability declaration that exposes `vectis` (`scaffold`).
3. If local WASI artifacts are under test, build or fetch them before the matrix run and point the scratch project's tool declaration at those component bytes. Do not execute a cached component directly; always run it through `specify tool run vectis -- scaffold`.
4. Platform prerequisites for each shell being validated are present (`xcodegen`, `cargo-swift`, `$ANDROID_HOME` with NDK, Gradle/Xcode as required). Confirm missing SDKs before editing templates; the skill's first action must not be to paper over a missing prereq.

---

## Process

The skill runs a five-step flow: **D1. Detect** breakage, **D2. Diagnose** the failure, **D3. Update** templates / modules / pins, **D4. Validate** the full cap matrix, and **D5. Report**. Each edit is scoped to `{repo-dir}/templates/vectis/**`, `{repo-dir}/crates/vectis/src/scaffold/templates/**`, `{repo-dir}/crates/vectis/embedded/versions.toml`, and (when a new upstream advisory appears) `{repo-dir}/templates/vectis/core/deny.toml`; everything else is off-limits.

The per-step playbook — registry queries, scratch-project scaffolding commands, the symptom→fix-site table for D2, narrow-edit guidance for D3, the full matrix loop for D4, and the report template for D5 — lives in [`references/process.md`](references/process.md). Read it before starting any of D1–D5; it carries the verbatim shell snippets the skill copies into the host workflow.

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
- **Dropping an existing `#[allow(...)]` on a capability type alias or `update()` match.** The render-only baseline intentionally carries `#[allow(dead_code)]` on capability `type` aliases and `#[allow(clippy::match_same_arms)]` on `update()`. These are part of the writer-skill handoff contract; do not touch them unless the writer skills have been updated in lockstep.

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
