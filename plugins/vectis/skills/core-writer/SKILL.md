---
name: vectis-core-writer
description: Generate or update a Rust Crux shared crate from Specify artifacts. Use when a Specify slice has pending Crux core tasks; not for platform shells (`ios-writer` / `android-writer`) or test scaffolding (`test-writer`).
argument-hint: "<slice-dir>"
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

Generate or update a buildable Crux core (`shared` crate) for a multi-platform application. The core contains all business logic, state management, and side-effect orchestration. No shell code (iOS, Android, Web) is generated -- separate skills handle those.

When an existing project is detected, the skill operates in **update mode**: it compares the Specify artifacts against the current implementation and makes targeted edits rather than regenerating from scratch.

When no project exists yet, the skill runs `specify tool run vectis -- scaffold core {AppName} [--caps {caps}]` to render the workspace, shared crate, and toolchain files using the active Vectis version pins. The declared scaffold tool is the single source of truth for Cargo manifests, `rust-toolchain.toml`, `.gitignore`, `ffi.rs`, `codegen.rs`, and the `lib.rs`/`app.rs` skeleton, but it does not run Cargo or inspect the host. Once the scaffold exists and the explicit host checks pass, this skill switches to **update mode** and layers feature-specific changes over the generated baseline.

## Arguments

| Argument | Required | Description |
|---|---|---|
| `slice-dir` | **Yes** | Path to the active Specify slice directory (`.specify/slices/<change>/`). |
| `feature-name` | **Yes** | Spec folder name under `{slice-dir}/specs/` identifying the feature to generate. |
| `project-dir` | No | Directory to create the project in. Defaults to current directory. |

## Input Artifacts

The skill reads from Specify artifacts rather than a standalone spec file:

- **Spec**: `{slice-dir}/specs/{feature-name}/spec.md` -- behavioral requirements using `### Requirement:` / `#### Scenario:` format. The skill reads the **core requirements** (main body of the spec). Platform-specific sections (e.g. `## iOS Shell Requirements`) are not relevant to core generation and are ignored.
- **Design**: `{slice-dir}/design.md` -- domain model, capabilities, API contracts, and technical design decisions.

The skill maps artifact content to Crux code constructs:

| Artifact Section | Maps to |
|---|---|
| **Spec** -- Requirements with feature-related scenarios | Shell-facing Event variants and `update()` match arm logic |
| **Spec** -- Requirements about views/pages | `ViewModel` enum variants, per-page view structs, `Page` enum variants |
| **Spec** -- Scenario conditions and validation rules | Validation logic in `update()` |
| **Design** -- Domain Model | Model fields, supporting types (domain structs/enums) |
| **Design** -- Capabilities | Effect variants and capability crates |
| **Design** -- API Contracts | HTTP call sites and response types |

If a required section is missing or too vague, ask **one** clarifying question before proceeding.

## Derived Arguments

The following are inferred from the Specify artifacts. Do **not** prompt for them unless the artifacts are too ambiguous to proceed.

| Derived | How to infer | Example |
|---|---|---|
| **App struct name** | PascalCase noun from the design overview or feature name | `TodoApp`, `NoteEditor`, `Counter` |
| **Model** | Internal state fields from Design Domain Model section | `todos: Vec<Todo>`, `filter: Filter` |
| **Event** | User actions from spec Requirements + internal callback variants from Capabilities + `Navigate(Route)` | `AddTodo(String)`, `Fetched(Result<...>)`, `Navigate(Route)` |
| **ViewModel variants** | One variant per view from spec Requirements about views/pages | `ViewModel::Loading`, `ViewModel::TodoList(TodoListView)` |
| **Page (internal)** | Internal enum mirroring ViewModel variants, tracked in Model | `Page::Loading`, `Page::TodoList` |
| **Route (shell-facing)** | Navigable views from spec Requirements (excludes internal states) | `Route::TodoList`, `Route::Settings` |
| **Per-page view structs** | Display data for each view from spec Requirements about UI | `TodoListView { items, count }`, `ErrorView { message }` |
| **Capabilities** | Explicitly listed in Design Capabilities section (see below) | Render + HTTP + KV |

### Capability Detection

Always include **Render**. Add others based on the Design Capabilities section:

| Capability | Artifact indicators |
|---|---|
| **HTTP** (`crux_http`) | HTTP capability listed in Design, or API Contracts section present |
| **Key-Value** (`crux_kv`) | Key-Value storage listed in Design Capabilities |
| **SSE / Streaming** (custom) | Server-Sent Events listed in Design Capabilities |
| **Time** (`crux_time`) | Timer / Time listed in Design Capabilities |
| **Platform** (`crux_platform`) | Platform detection listed in Design Capabilities |

If the design describes effects not covered by published capabilities, generate a custom capability module following the pattern in `references/crux-custom-capabilities.md`.

## Mode Detection

The skill operates in one of two modes depending on whether an existing project is found:

- **Create Mode** -- used when `{project-dir}/shared/src/app.rs` does **not** exist. The skill invokes `specify tool run vectis -- scaffold core` to render the baseline, runs explicit Cargo verification, then proceeds directly into Update Mode to apply feature-specific changes from the Specify artifacts.
- **Update Mode** -- used when `{project-dir}/shared/src/app.rs` **does** exist. Reads the existing code, diffs it against the artifacts, and makes targeted edits (steps U1--U8 below).

The Specify artifacts always represent the **full desired state** of the application, not a partial diff. In update mode the skill compares the full artifacts against the existing implementation to determine what changed.

Detection rule: check for the file `{project-dir}/shared/src/app.rs`. If the file exists, switch to update mode. If not, run:

```bash
cd {project-dir}
specify tool run vectis -- scaffold core {AppName} [--caps {detected-caps}]
cargo fmt --check
cargo check
cargo clippy --all-targets -- -D warnings
cargo test
```

`{AppName}` is the derived App struct name (see Derived Arguments). `{detected-caps}` is the comma-separated list from Capability Detection (e.g. `http,kv`; omit the flag when only Render is needed). If the scaffold command fails, report the tool's structured output to the user and stop -- do **not** attempt a manual scaffold as a fallback. If a host verification command fails, return a verification object with the failed step's `name`, `passed: false`, and the relevant stderr/stdout snippet, then stop.

If the scaffold and host checks succeed, switch to Update Mode (the just-scaffolded project is the "existing implementation" Update Mode diffs against). The baseline emitted by `vectis` (`scaffold`) is a render-only scaffold with type aliases for each selected capability and placeholder `update()` arms; Update Mode fills in domain types, Model fields, Event/ViewModel variants, and real handler logic derived from the Specify artifacts.

### Repair mode

This skill may be invoked as a **repair sub-agent** from the verify-repair loop. In repair mode the skill receives:

- `mode: repair` (not `create` or `update`)
- The full compiler or test error output
- The repair discipline constraints (minimum change, scoped diff)

When invoked in repair mode:

1. Read `app.rs` and any files referenced in the error output.
2. Diagnose the root cause from the error output.
3. Apply the minimum change to fix the reported errors.
4. Do **not** re-read the full reference documentation or re-run the complete create/update process. The repair is scoped to the errors provided.
5. Return the list of files modified and the fix applied.

## Process: Create Mode

Use this process when no existing project is found at `{project-dir}`. The scaffold tool owns all render-only boilerplate (workspace manifest, `shared/Cargo.toml`, `rust-toolchain.toml`, `.gitignore`, `clippy.toml`, `ffi.rs`, `codegen.rs`, `lib.rs`, and a render-only `app.rs` skeleton with type aliases for each selected capability). This skill's Create-Mode responsibilities are: (1) read the Specify artifacts to derive the App name and capability set, (2) invoke the scaffold tool, (3) run host checks, (4) switch to Update Mode.

### 1. Read and analyze the Specify artifacts

Read the spec at `{slice-dir}/specs/{feature-name}/spec.md` and the design at `{slice-dir}/design.md`. Extract core requirements from the main body of the spec (stop before any `## ... Shell Requirements` or `## Design System Requirements` sections):
- The core concept and app name (from the design overview or feature name)
- State that needs to be tracked (from **Design Domain Model** -> Model)
- Actions the user can take (from **Spec Requirements** with feature scenarios -> shell-facing Event variants)
- Side-effects needed (from **Design Capabilities** -> Effect variants and internal Event variants)
- What the UI needs to show (from **Spec Requirements** about views/pages -> per-page view structs)
- Distinct screens/pages (from **Spec Requirements** about views -> ViewModel enum variants, internal Page enum)
- API shapes (from **Design API Contracts** -> HTTP call sites and response types)
- Validation and constraints (from **Spec Scenarios** with conditions -> logic in `update()`)

If a required section is missing or too vague to determine Model and Events, ask **one** clarifying question. Use `[unknown]` tokens for anything genuinely ambiguous rather than guessing.

### 2. Invoke the scaffold tool and host checks

Derive `{AppName}` (see Derived Arguments § App struct name) and `{caps}` (see Capability Detection; comma-separated, lowercase, in artifact order). Then run:

```bash
cd {project-dir}
specify tool run vectis -- scaffold core {AppName} [--caps {caps}]
cargo fmt --check
cargo check
cargo clippy --all-targets -- -D warnings
cargo test
```

The scaffold tool produces structured output and preserves the atomic refusal contract (reject pre-existing target files before writing). On non-zero exit, surface the tool output to the user and stop -- do not attempt to hand-author any of the scaffolded files. For each host command, record `name`, `passed`, and a failure snippet; a green host verification is a precondition for this skill to do useful Update-Mode work.

### 3. Switch to Update Mode

After the scaffold and host checks return green, treat the scaffolded project as an existing implementation and execute **Process: Update Mode** below to fill in:

- Domain types (structs/enums from Design Domain Model)
- Model fields
- Page / Route / ViewModel variants + per-page view structs
- Shell-facing and internal Event variants
- `update()` match-arm logic and helper functions
- `view()` model-to-ViewModel mapping

The scaffolded `app.rs` ships with placeholder `update()` arms (each capability's arm calls `render()` with a `#[allow(clippy::match_same_arms)]` on the function and `#[allow(dead_code)]` on each capability `type` alias). Update Mode replaces those placeholders with real logic; when the placeholder bodies are gone, drop the two render-only-baseline `#[allow(...)]` attributes -- leaving them in place is harmless under `-D warnings` but masks future regressions.

The core scaffold also seeds `deny.toml` with a `[licenses] private = { ignore = true }` allowance and an `[advisories] ignore = [...]` list for today's unavoidable transitive advisories, plus `publish = false` in `shared/Cargo.toml`. Do not hand-seed `supply-chain/config.toml` exemptions during core generation; any broader post-merge vetting is owned by the host verification workflow. If the user later decides to publish the `shared` crate, both `publish = false` **and** a matching `license = "..."` field must land in the same edit (they pair together).

## Test Generation

Tests are generated separately by test-writer. core-writer does not generate tests. The build orchestration layer runs test-writer after core-writer completes, then runs a unified verify-repair loop across both code and tests.

## Process: Update Mode

Use this process when `{project-dir}/shared/src/app.rs` already exists. The goal is to bring an existing implementation into alignment with updated Specify artifacts through targeted, minimal edits. Never regenerate a file from scratch in update mode.

### U1. Read and analyze the Specify artifacts

Same extraction as create mode step 1. Read `{slice-dir}/specs/{feature-name}/spec.md` and `{slice-dir}/design.md`. Build the full picture of the desired application state: app name, features, data model, UI, capabilities, API shapes, and business rules.

### U2. Read existing code

Read every source file in the project:

- `shared/src/app.rs` -- types, events, model, view model, effects, update logic, view logic, helper functions, tests
- `shared/src/lib.rs` -- module declarations, re-exports
- `shared/src/ffi.rs` -- `CoreFFI` bridge type
- Any custom capability modules (e.g., `shared/src/sse.rs`)
- `shared/Cargo.toml` -- dependencies and features
- `{project-dir}/Cargo.toml` -- workspace dependencies

### U3. Build implementation inventory

Extract a structured inventory from the existing code. For each category, list every item by name:

| Category | What to extract | Where to find it |
|---|---|---|
| Domain types | Struct/enum names, fields, derives | Top of `app.rs` |
| Model fields | Field names, types (including `page: Page`) | `struct Model` in `app.rs` |
| Page variants | Variant names | `enum Page` in `app.rs` |
| Shell-facing Event variants | Variant names, payload types | `enum Event` (non-skipped) in `app.rs` |
| Internal Event variants | Variant names, payload types | `enum Event` (`#[serde(skip)]`) in `app.rs` |
| ViewModel variants | Variant names, wrapped view struct types | `enum ViewModel` in `app.rs` |
| Per-page view structs | Struct names, field names, types | Per-page structs in `app.rs` |
| Effect variants | Variant names, operation types | `enum Effect` in `app.rs` |
| Capability type aliases | Alias names | `type Http = ...`, `type KeyValue = ...` in `app.rs` |
| `update()` arms | Event variant -> behavior summary | `fn update()` match block in `app.rs` |
| `view()` logic | Model-to-ViewModel mapping | `fn view()` in `app.rs` |
| Helper functions | Names, signatures, purposes | Free functions and `impl` blocks in `app.rs` |
| Custom capability modules | Module names, operations | Separate `.rs` files, `lib.rs` module decls |
| Dependencies | Crate names, features | `shared/Cargo.toml` `[dependencies]` |
| Tests | Test function names, which events they cover (owned by test-writer) | `#[cfg(test)] mod tests` in `app.rs` |

### U4. Diff analysis

Compare the artifact requirements (from U1) against the implementation inventory (from U3). For each category, classify every item into one of four buckets:

- **Added** -- present in the artifacts but absent from the code. Requires new code.
- **Removed** -- present in the code but absent from the artifacts. Requires deletion.
- **Modified** -- present in both but the artifacts describe different fields, types, behavior, or constraints. Requires editing existing code.
- **Unchanged** -- present in both with matching semantics. Leave alone.

Walk through the categories in this order, since later categories depend on earlier ones:

1. **Capabilities** -- added or removed capabilities affect Effect, Event, imports, and deps.
2. **Views** -- added or removed views affect `Page` enum, `Route` enum (if navigable), `ViewModel` enum, per-page view structs, `Navigate` handler, and `view()` match arms.
3. **Domain types** -- new or changed structs/enums affect Model, Event payloads, and API shapes.
4. **Model fields** -- new state fields may be needed before events can reference them.
5. **Event variants** -- added/removed/modified user actions and internal callbacks.
6. **Per-page view struct fields** -- changes in what individual views display.
7. **API shapes** -- changed endpoints, request/response bodies.
8. **Business rules** -- changed validation or logic in `update()` arms.
9. **`view()` logic** -- changes driven by view struct, Model, or Page changes.

After completing the diff, output a summary listing every added, removed, and modified item before making any edits. This summary serves as the edit plan.

### U5. Apply changes to types and structure

Edit `app.rs` to reflect the structural changes identified in U4. Work top-down through the file:

1. Add, remove, or modify **domain types** (structs, enums, and their fields/variants).
2. Add or remove **Page variants** in `enum Page`, corresponding **ViewModel variants** in `enum ViewModel`, and **Route variants** in `enum Route` (for navigable views). Add or remove per-page view structs as needed.
3. Add or remove **Model fields** (ensure new fields have `Default` values).
4. Add, remove, or modify **per-page view struct fields**.
5. Add or remove **Event variants** -- new shell-facing variants go in the shell section; new internal variants go in the internal section with `#[serde(skip)]` and `#[facet(skip)]`.
5. Add or remove **Effect variants** and update capability **type aliases**.
6. Update **imports** at the top of the file for any added or removed capabilities.

If a new capability is added, also update:
- `shared/Cargo.toml` -- add the crate dependency
- `{project-dir}/Cargo.toml` -- add to `[workspace.dependencies]`
- `shared/src/lib.rs` -- add `pub mod {capability};` if it is a custom module

If a capability is removed, reverse those changes.

### U6. Apply changes to logic

Edit the `update()` and `view()` functions in `app.rs`:

1. For **added Event variants**, add new match arms. Consult `references/crux-command-api.md` for command patterns and `references/crux-capabilities.md` for capability APIs.
2. For **removed Event variants**, delete the match arm.
3. For **modified Event variants**, update the match arm logic to match the new artifact requirements.
4. For **changed business rules**, update the relevant match arm logic or helper functions.
5. For **changed API shapes**, update HTTP call construction (URL, body struct, method) and response handling.
6. Update `view()` if Page variants, ViewModel variants, or per-page view struct fields were added, removed, or their derivation from Model changed. Every `Page` variant must have a corresponding match arm in `view()`.
7. Add, modify, or remove **helper functions** as needed.

### U7. Verify

Run `cargo check` as a quick sanity check after applying all changes. Fix any compilation errors before proceeding. Full verification (fmt, clippy, test suite, regression detection) runs at the orchestration level after test-writer completes.

Also review for:

1. Unused dependencies in `Cargo.toml` (especially after removing a capability).
2. Logic bugs (state consistency, ownership, KV payload types, pending op removal by ID).

### U8. Final diff review

After all edits and verification pass, do a final review of every changed line. Confirm:

- No unchanged code was accidentally modified.
- No orphaned types, fields, imports, or test functions remain.
- The code compiles and clippy is clean.

## Artifact-to-Code Mapping

See [`references/artifact-to-code-mapping.md`](references/artifact-to-code-mapping.md) for the full table mapping each Specify artifact section to its code construct, target file, and diff indicators. Walk it during Update-Mode diff analysis (U4) to identify what changed.

## Update Change Patterns

See [`references/update-change-patterns.md`](references/update-change-patterns.md) for the full checklist of which code elements each common change pattern (add/remove view, add/remove feature, add/remove capability, change API endpoint, change business rule) touches. Use it as a step-by-step guide during U5--U7.

## Preservation Rules

In update mode, minimize collateral changes. See [`rules.md`](rules.md) for the ten-rule preservation contract (never regenerate from scratch; preserve helper functions, test utilities, code organization, `ffi.rs`, custom capability modules, `clippy.toml`/`rust-toolchain.toml`, `Cargo.lock`, doc comments, and `#[allow(...)]` attributes on unchanged functions).

## Reference Documentation

Consult these references during generation. Do not deviate from the patterns they describe.

| Reference | Purpose |
|---|---|
| `references/crux-app-pattern.md` | App trait, Model, Event, ViewModel (enum), Page management, Route/Navigate pattern, Effect type conventions |
| `references/crux-command-api.md` | Command creation, chaining, combining, async context |
| `references/crux-capabilities.md` | HTTP and KV capability APIs |
| `references/crux-custom-capabilities.md` | Building custom Operation + capability (SSE example) |
| `references/crux-testing-patterns.md` | Testing effects, events, resolving requests |

Version pins, Cargo workspace layout, `rust-toolchain.toml`, `ffi.rs`, `codegen.rs`, and `.gitignore` are owned by the Vectis scaffold templates in the [`augentic/specify-cli`](https://github.com/augentic/specify-cli) repo (`<specify-cli>/crates/vectis/` and the Vectis template sources). When a spec change requires updating a pinned version, route that through the Vectis version/template workflow rather than editing generated dependency versions in this crate by hand.

## Examples

See `references/examples/` for complete worked examples:

| Example | Capabilities | Demonstrates |
|---|---|---|
| `01-simple-counter.md` | Render | Minimal app, single-view ViewModel enum, Route/Navigate, basic testing |
| `02-http-counter.md` | Render + HTTP | Two-view pattern (Loading + Counter), Route/Navigate, API calls, optimistic updates |
| `03-kv-notes.md` | Render + KV | Three-view pattern (Loading + NoteList + Error), local persistence, Navigate(Route) |

## Error Handling

| Error | Resolution |
|---|---|
| `cargo check` fails with unresolved import | Verify capability crate is in `[workspace.dependencies]` and `shared/Cargo.toml` |
| `Command` type mismatch | Ensure `update()` returns `Command<Effect, Event>` |
| `facet` derive errors | Ensure `facet` matches the active Vectis version pins, then rerun the core host verification commands. Add `#[repr(C)]` to enums |
| `uniffi` build failures | Ensure `uniffi` is behind `feature = "uniffi"` gate, not unconditional |
| Missing `Operation` impl for custom capability | Each custom request type must `impl Operation` with `type Output` |
| `#[serde(skip)]` on Event variant causes deserialization panic | Internal variants must never be sent from the shell; guard with `#[facet(skip)]` too |
| KV `set`/`delete` callback type mismatch (`Result<(), _>` vs `Result<Option<Vec<u8>>, _>`) | `KeyValue::set` and `KeyValue::delete` return the previous value as `Result<Option<Vec<u8>>, KeyValueError>`, never `Result<(), _>`. Update the Event variant payload to match. |

## Verification Checklist

Before completing, verify. Items marked **(update)** apply only in update mode; all other items apply in both modes.

### Build and lint

- [ ] `cargo check` passes with no errors
- [ ] `cargo clippy --all-targets` passes with no warnings
- [ ] Workspace lints (`all`, `nursery`, `pedantic`, `cargo`, restriction cherry-picks) are configured in workspace `Cargo.toml` and inherited via `[lints] workspace = true`
- [ ] `clippy.toml` exists with `allowed-duplicate-crates` populated for transitive duplicates

### Types and structure

- [ ] Every Event variant is handled in `update()`
- [ ] Every `update()` branch returns a `Command<Effect, Event>` (not `()`)
- [ ] Internal Event variants have `#[serde(skip)]` and `#[facet(skip)]`
- [ ] `ViewModel` is an enum with `#[repr(C)]` and derives `Facet, Serialize, Deserialize, Clone, Debug, Default`
- [ ] Every `Page` variant has a corresponding `ViewModel` variant and a match arm in `view()`
- [ ] Every `Page` variant is reachable by at least one transition in `update()`
- [ ] `Page` and `ViewModel` variants have a 1:1 correspondence
- [ ] `Route` enum exists with variants for user-navigable views (excludes Loading, Error)
- [ ] `Event::Navigate(Route)` variant exists and is handled in `update()`
- [ ] `Navigate` handler is state-aware (considers `model.page` before transitioning)
- [ ] Per-page view structs derive `Facet, Serialize, Deserialize, Clone, Debug, Default`
- [ ] Effect enum uses `#[effect(facet_typegen)]`
- [ ] `CoreFFI` uses feature-gated `uniffi` and `wasm_bindgen` attributes
- [ ] `CoreFFI` methods return `Result<Vec<u8>, CoreError>`, not `Vec<u8>` with `panic!`
- [ ] `CoreError` derives `thiserror::Error` and feature-gated `uniffi::Error` with `uniffi(flat_error)`
- [ ] Type aliases defined for each capability: `type Http = crux_http::Http<Effect, Event>;`
- [ ] KV callback Event variants use `Result<Option<Vec<u8>>, KeyValueError>` for `get`/`set`/`delete` (not `Result<(), _>`) and `Result<bool, KeyValueError>` for `exists`

### Code quality

- [ ] No `unwrap()` or `expect()` in production code (allowed in tests; `expect()` allowed only for provably infallible operations like serializing a simple derive struct)
- [ ] No unused dependencies in `Cargo.toml` -- every crate has a matching `use` in `src/`
- [ ] Helper functions take `&T` / `&[T]` unless they need ownership
- [ ] Doc comments use backticks around type and parameter names
- [ ] State transitions are consistent across chained events (no contradictory state before a follow-up `Command::event`)
- [ ] Pending ops removed by tracked ID (`syncing_id`), never by index (`remove(0)`)

### Update mode only

- [ ] **(update)** All **added** Event variants from the artifacts are present in `enum Event` and handled in `update()`
- [ ] **(update)** All **removed** Event variants are gone from both `enum Event` and the `update()` match block
- [ ] **(update)** No orphaned Model fields -- fields removed from the artifacts are deleted from `struct Model` and all references
- [ ] **(update)** No orphaned ViewModel variants -- views removed from the artifacts are deleted from `enum ViewModel`, `enum Page`, `enum Route` (if navigable), per-page view structs, `Navigate` handler, and `view()` match arms
- [ ] **(update)** No orphaned per-page view struct fields -- fields removed from the artifacts are deleted from the struct and `view()`
- [ ] **(update)** No orphaned internal Event variants -- if a capability was removed, its callback Event variants and match arms are also removed
- [ ] **(update)** No orphaned Effect variants or type aliases for removed capabilities
- [ ] **(update)** No orphaned imports (`use` statements) for removed crates or types
- [ ] **(update)** Preservation rules were followed -- unchanged code, comments, helpers, and test utilities were not modified

## Important Notes

- **Crux versions**: The Vectis scaffold owns all generated version pins through embedded defaults or an explicit `--version-file`. Never hand-edit Cargo dependency versions in a generated project. `crux_core`, `facet`, `uniffi`, and companion crates are selected together so that `crux_core`'s bundled `uniffi_bindgen` matches the runtime `uniffi` crate.
- **No `Capabilities` struct**: The 0.17 API does not use a `Capabilities` struct. Define `Effect` directly as an enum with `#[effect(facet_typegen)]`. The `App` trait requires `type Effect = Effect;`.
- **`Command` is generic**: Return `Command<Effect, Event>` from `update()`.
- **`#[repr(C)]` on Event enums**: Required by `facet` for enums that cross the FFI boundary.
- **Codegen uses `TypeRegistry`**: The codegen binary uses `crux_core::type_generation::facet::TypeRegistry` for compile-time type extraction. Do NOT use `crux_core::cli::run()` which depends on `rustdoc-types` and breaks with newer Rust versions.
- **SSE is not a published crate**: It is a custom capability. Generate it inline when needed.
- **Tests are test-writer's responsibility**: core-writer generates code only. The build orchestration layer runs test-writer after core-writer, then runs a unified verify-repair loop. Test coverage, spec traceability, and test updates are all owned by test-writer.
- **Core only**: This skill generates only the `shared` crate. Shell skills are separate.
