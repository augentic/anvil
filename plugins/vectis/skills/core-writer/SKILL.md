---
name: vectis-core-writer
description: Generate or update a Rust Crux shared crate from Specify artifacts. Use when a Specify slice has pending Crux core tasks; not for platform shells (`ios-writer` / `android-writer`) or test scaffolding (`test-writer`).
argument-hint: <slice-dir> <feature-name> [project-dir]
---

# Crux Core Application Generator

> **Vectis deterministic tooling runs through declared Specify tools.** Scaffold rendering is `specify tool run vectis -- scaffold ...`; validation is `specify tool run vectis -- validate ...`. Scaffolding is render-only: host verification remains skill-owned and must return step evidence (`name`, `passed`, and a failure snippet on error).

## Critical Path

1. Read Specify artifacts (`{slice-dir}/specs/<feature>/spec.md` + `{slice-dir}/design.md`); extract App name, Model, Events, ViewModel/Page/Route, capabilities, and API shapes.
2. Detect mode from `{project-dir}/shared/src/app.rs`: missing → run `specify tool run vectis -- scaffold core ...` plus explicit Cargo verification, then enter Update Mode; present → start Update Mode immediately.
3. Build an implementation inventory of existing types and diff it against the artifact-derived target — Added / Removed / Modified / Unchanged — per category in dependency order (capabilities → views → domain → model → events → api → logic).
4. Apply structural edits to `app.rs` (domain types → Page/ViewModel/Route → Model → Event/Effect → imports + `Cargo.toml` for new capabilities).
5. Apply logic edits to `update()` and `view()` (per-Event match arms, business rules, model-to-ViewModel mapping for new pages); consult `references/crux-command-api.md` and `references/crux-capabilities.md`.
6. Run `cargo check` as a sanity gate; full clippy / test / regression runs at the orchestration level via test-writer + the unified verify-repair loop.
7. Final diff review against [`rules.md`](rules.md) — never regenerate a file from scratch; preserve helpers, comments, custom capability modules, and `Cargo.lock`.

## Orientation

The core writer generates or updates a buildable Crux core (`shared` crate) for a multi-platform application. The core contains all business logic, state management, and side-effect orchestration. No shell code (iOS, Android, Web) is generated — separate skills handle those.

The skill reads Specify artifacts (spec + design) rather than a standalone spec file, and maps artifact content to Crux constructs (Model, Event, ViewModel/Page/Route, Effect, capabilities). The artifacts always represent the **full desired state** of the application, not a partial diff.

When an existing project is detected, the skill operates in **update mode**: it compares the artifacts against the current implementation and makes targeted edits. When no project exists yet, the skill runs `specify tool run vectis -- scaffold core {AppName} [--caps {caps}]` to render the workspace, shared crate, and toolchain files using the active Vectis version pins. The declared scaffold tool is the single source of truth for Cargo manifests, `rust-toolchain.toml`, `.gitignore`, `ffi.rs`, `codegen.rs`, and the `lib.rs`/`app.rs` skeleton, but it does not run Cargo or inspect the host. Once the scaffold exists and the explicit host checks pass, the skill switches to update mode.

The skill can also be invoked as a **repair sub-agent** from the verify-repair loop (`mode: repair` with error output) to apply the minimum change to fix reported errors without re-running the full create/update process.

See [`references/runbook.md`](references/runbook.md) for input-artifact mapping, capability detection, mode-detection rules, the Create / Update step bodies (1–3 / U1–U8), Repair mode, error tables, and the verification checklist.

## Reference Documentation

Consult these references during generation. Do not deviate from the patterns they describe.

| Reference | Purpose |
|---|---|
| [`references/runbook.md`](references/runbook.md) | Input artifacts, capability detection, mode detection, Create/Update/Repair step bodies, artifact-to-code/update-change pointers, error table, verification checklist |
| [`references/crux-app-pattern.md`](references/crux-app-pattern.md) | App trait, Model, Event, ViewModel (enum), Page management, Route/Navigate pattern, Effect type conventions |
| [`references/crux-command-api.md`](references/crux-command-api.md) | Command creation, chaining, combining, async context |
| [`references/crux-capabilities.md`](references/crux-capabilities.md) | HTTP and KV capability APIs |
| [`references/crux-custom-capabilities.md`](references/crux-custom-capabilities.md) | Building custom Operation + capability (SSE example) |
| [`references/crux-testing-patterns.md`](references/crux-testing-patterns.md) | Testing effects, events, resolving requests |
| [`references/artifact-to-code-mapping.md`](references/artifact-to-code-mapping.md) | Full table mapping each Specify artifact section to its code construct, target file, and diff indicators |
| [`references/update-change-patterns.md`](references/update-change-patterns.md) | Checklist of which code elements each common change pattern touches |
| [`rules.md`](rules.md) | Ten-rule preservation contract (helpers, test utilities, `ffi.rs`, custom capability modules, `Cargo.lock`, doc comments, `#[allow(...)]` attributes) |

## Guardrails

- **ALWAYS read [`rules.md`](rules.md) first** for the preservation contract and platform-level normative facts before editing any generated file.
- **NEVER hand-edit Cargo dependency versions in a generated project** (the Vectis scaffold owns version pins so `crux_core`'s bundled `uniffi_bindgen` matches the runtime `uniffi` crate); **NEVER define a `Capabilities` struct** (the 0.17 API uses `Effect` directly as an enum with `#[effect(facet_typegen)]`); **NEVER call `crux_core::cli::run()`** (use `crux_core::type_generation::facet::TypeRegistry` instead).
- **NEVER write tests in this skill** (test-writer owns them) and **NEVER generate shell code** (iOS / Android / Web are separate skills). **ALWAYS return `Command<Effect, Event>` from `update()`**, **ALWAYS mark Event enums `#[repr(C)]`**, and **ALWAYS generate SSE inline as a custom capability** (not a published crate).
