---
name: core-writer
description: Generate or update a Rust Crux shared crate from Specify artifacts. Use when implementing core module tasks from a Specify change, or when the user mentions core-writer.
---

# Crux Core Application Generator

Generate or update a buildable Crux core (`shared` crate) for a multi-platform
application. The core contains all business logic, state management, and side-effect
orchestration. No shell code (iOS, Android, Web) is generated -- separate skills
handle those.

When an existing project is detected, the skill operates in **update mode**: it
compares the Specify artifacts against the current implementation and makes targeted
edits rather than regenerating from scratch.

This skill targets **Crux 0.17.0-rc3**, pinned to the `crux_core-v0.17.0-rc3` tag.
Dependencies use git references with a tag until the crate is published to crates.io.

## Arguments

| Argument | Required | Description |
|---|---|---|
| `change-dir` | **Yes** | Path to the active Specify change directory (`.specify/changes/<change>/`). |
| `module-name` | **Yes** | Spec folder name under `{change-dir}/specs/` identifying the module to generate. |
| `project-dir` | No | Directory to create the project in. Defaults to current directory. |

## Input Artifacts

The skill reads from Specify artifacts rather than a standalone spec file:

- **Spec**: `{change-dir}/specs/{module-name}/spec.md` -- behavioral requirements
  using `### Requirement:` / `#### Scenario:` format.
- **Design**: `{change-dir}/design.md` -- domain model, capabilities, API contracts,
  and technical design decisions.

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

The following are inferred from the Specify artifacts. Do **not** prompt for them unless
the artifacts are too ambiguous to proceed.

| Derived | How to infer | Example |
|---|---|---|
| **App struct name** | PascalCase noun from the design overview or module name | `TodoApp`, `NoteEditor`, `Counter` |
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

If the design describes effects not covered by published capabilities, generate a
custom capability module following the pattern in `references/crux-custom-capabilities.md`.

## Mode Detection

The skill operates in one of two modes depending on whether an existing project is found:

- **Create Mode** -- used when `{project-dir}/shared/src/app.rs` does **not** exist.
  Generates the entire project from scratch (steps 1--12 below).
- **Update Mode** -- used when `{project-dir}/shared/src/app.rs` **does** exist.
  Reads the existing code, diffs it against the artifacts, and makes targeted edits
  (steps U1--U9 below).

The Specify artifacts always represent the **full desired state** of the application, not a
partial diff. In update mode the skill compares the full artifacts against the existing
implementation to determine what changed.

To detect the mode, check for the file `{project-dir}/shared/src/app.rs` before
starting any generation work. If the file exists, switch to update mode. If not,
proceed with create mode.

## Process: Create Mode

Use this process when no existing project is found at `{project-dir}`.

### 1. Read and analyze the Specify artifacts

Read the spec at `{change-dir}/specs/{module-name}/spec.md` and the design at
`{change-dir}/design.md`. Extract:
- The core concept and app name (from the design overview or module name)
- State that needs to be tracked (from **Design Domain Model** -> Model)
- Actions the user can take (from **Spec Requirements** with feature scenarios -> shell-facing Event variants)
- Side-effects needed (from **Design Capabilities** -> Effect variants and internal Event variants)
- What the UI needs to show (from **Spec Requirements** about views/pages -> per-page view structs)
- Distinct screens/pages (from **Spec Requirements** about views -> ViewModel enum variants, internal Page enum)
- API shapes (from **Design API Contracts** -> HTTP call sites and response types)
- Validation and constraints (from **Spec Scenarios** with conditions -> logic in `update()`)

If a required section is missing or too vague to determine Model and Events, ask **one**
clarifying question. Use `[unknown]` tokens for anything genuinely ambiguous rather than
guessing.

### 2. Design the type system

Before writing any code, design these types on paper:

1. **Model** -- all internal state, including a `page: Page` field. Use newtypes and
   enums for domain concepts. Fields should be `pub(crate)` unless needed externally.
2. **Page (internal)** -- an enum with one variant per view. Derives `Default` only
   (no `Facet`, no `Serialize`). The `#[default]` variant is the initial view
   (typically `Loading`). Add to Model as `page: Page`.
3. **Route (shell-facing)** -- an enum enumerating user-navigable destinations.
   Derives `Facet, Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq`
   and has `#[repr(C)]`. Includes only content views the shell can navigate to
   (excludes internal states like `Loading` and `Error`). Use `#[default]` on the
   primary content view. Can carry payload for parameterised views
   (e.g. `ItemDetail(String)`).
4. **Event** -- split into shell-facing variants (serializable, sent by UI) and
   internal variants (marked `#[serde(skip)]` `#[facet(skip)]`, used as effect callbacks).
   Include a `Navigate(Route)` shell-facing variant for shell-initiated view changes.
5. **ViewModel** -- an enum with `#[repr(C)]`. One variant per view, matching the
   `Page` enum 1:1. Variants without data (e.g. `Loading`) have no payload; variants
   with data wrap a per-page view struct. Derive `Facet, Serialize, Deserialize,
   Clone, Debug, Default`.
6. **Per-page view structs** -- one struct per ViewModel variant that carries data
   (e.g. `MainView`, `ErrorView`). Derive `Facet, Serialize, Deserialize, Clone,
   Debug, Default`. All fields are `pub`. Use `String` for formatted display values.
   `ErrorView` should have `message: String` and `can_retry: bool`.
7. **Effect** -- one variant per capability. Annotate with `#[effect(facet_typegen)]`.
8. **Supporting types** -- domain structs/enums used in Model, Event, or per-page
   view structs.

Consult `references/crux-app-pattern.md` for type conventions.

### 3. Generate workspace files

Create the workspace root with two files:

**`{project-dir}/Cargo.toml`** -- workspace manifest:
```toml
[workspace]
members = ["shared"]
resolver = "3"

[workspace.package]
edition = "2024"
rust-version = "1.88"

[workspace.dependencies]
crux_core = { git = "https://github.com/redbadger/crux", tag = "crux_core-v0.17.0-rc3" }
serde = "1.0"
facet = "=0.31"

[workspace.lints.rust]
trivial_numeric_casts = "warn"
unused_extern_crates = "warn"
unsafe_op_in_unsafe_fn = "warn"

[workspace.lints.clippy]
all = "warn"
nursery = "warn"
pedantic = "warn"
cargo = "warn"
as_pointer_underscore = "warn"
assertions_on_result_states = "warn"
clone_on_ref_ptr = "warn"
deref_by_slicing = "warn"
disallowed_script_idents = "warn"
empty_drop = "warn"
empty_enum_variants_with_brackets = "warn"
empty_structs_with_brackets = "warn"
fn_to_numeric_cast_any = "warn"
if_then_some_else_none = "warn"
map_err_ignore = "warn"
redundant_type_annotations = "warn"
renamed_function_params = "warn"
semicolon_outside_block = "warn"
undocumented_unsafe_blocks = "warn"
unnecessary_safety_comment = "warn"
unnecessary_safety_doc = "warn"
unneeded_field_pattern = "warn"
unused_result_ok = "warn"
```

Add capability crates to `[workspace.dependencies]` based on detected capabilities.
See `references/crux-project-config.md` for the full dependency list.

**`{project-dir}/clippy.toml`** -- clippy configuration:
```toml
doc-valid-idents = []

allowed-duplicate-crates = []
```

Populate `allowed-duplicate-crates` after `cargo clippy` reports false-positive duplicate
crate warnings from transitive dependencies. Run `cargo tree -d | grep '^[a-z]'` to discover
which crates are duplicated.

**`{project-dir}/rust-toolchain.toml`** -- toolchain config:
```toml
[toolchain]
channel = "stable"
components = ["rustfmt", "rustc-dev"]
targets = [
    "aarch64-apple-darwin",
    "aarch64-apple-ios",
    "aarch64-apple-ios-sim",
    "aarch64-linux-android",
    "wasm32-unknown-unknown",
    "x86_64-apple-ios",
]
profile = "minimal"
```

### 4. Generate `shared/Cargo.toml`

Follow the template in `references/crux-project-config.md`. Key points:
- `crate-type = ["cdylib", "lib", "staticlib"]`
- Feature-gate `uniffi` and `wasm_bindgen` dependencies
- Add a `codegen` feature for type generation
- Include only the capability crates actually needed
- Add `[lints] workspace = true` to inherit workspace lint configuration
- Add `#![allow(clippy::cargo_common_metadata)]` to `lib.rs` if the crate is not
  intended for crates.io publication (e.g., example projects)

### 5. Generate `shared/src/app.rs`

This is the heart of the application. Follow `references/crux-app-pattern.md`:

1. Define supporting types (domain structs/enums)
2. Define internal `Page` enum with `#[derive(Default)]` and `#[default]` on the
   initial variant (typically `Loading`)
3. Define `Route` enum (shell-facing navigable destinations) with
   `#[derive(Facet, Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]`
   and `#[repr(C)]`. Include only user-navigable content views (exclude `Loading`,
   `Error`). Use `#[default]` on the primary content view.
4. Define `Model` with `#[derive(Default)]`, including a `page: Page` field
5. Define per-page view structs (e.g. `MainView`, `ErrorView`) with
   `#[derive(Facet, Serialize, Deserialize, Clone, Debug, Default)]`
6. Define `ViewModel` as an enum with `#[derive(Facet, Serialize, Deserialize,
   Clone, Debug, Default)]` and `#[repr(C)]`. One variant per page, wrapping the
   corresponding per-page view struct (or no payload for data-less views like `Loading`)
7. Define `Event` with shell and internal variants. Include a `Navigate(Route)`
   shell-facing variant for shell-initiated view changes.
8. Define `Effect` enum with `#[effect(facet_typegen)]`
9. If using HTTP, add type alias: `type Http = crux_http::Http<Effect, Event>;`
10. If using KV, add type alias: `type KeyValue = crux_kv::KeyValue<Effect, Event>;`
11. Implement `App` trait with `update()` and `view()`. The `view()` function must
    match on `model.page` and return the corresponding `ViewModel` variant.
    The `Navigate(route)` arm must be state-aware: navigating to a view that
    requires data may trigger a load sequence rather than an immediate page switch.
12. Write `#[cfg(test)] mod tests` with at least one test per Event variant, plus
    tests verifying page transitions (e.g. `Loading` -> main view after data loads)
    and navigation events (e.g. Navigate from Error triggers re-initialization)

For `update()` logic, consult `references/crux-command-api.md` for command patterns.
For testing, consult `references/crux-testing-patterns.md`.

### 6. Generate `shared/src/ffi.rs`

Follow `references/crux-ffi-scaffolding.md` exactly. The `CoreFFI` struct is identical
across all apps except for the `Bridge<AppType>` generic parameter.

### 7. Generate custom capability modules (if needed)

If SSE or other custom capabilities are needed, generate them as separate modules.
Follow `references/crux-custom-capabilities.md` for the pattern.

### 8. Generate `shared/src/bin/codegen.rs`

Generate the type-generation binary following `references/crux-project-config.md` § Codegen Binary.
Replace `MyApp` with the app struct name. The binary uses `TypeRegistry` from facet to
extract types at compile time -- it does not depend on RustDoc JSON parsing.

### 9. Generate `shared/src/lib.rs`

Wire everything together. The FFI module is conditionally compiled behind the
`uniffi` and `wasm_bindgen` features:
```rust
mod app;
#[cfg(any(feature = "wasm_bindgen", feature = "uniffi"))]
mod ffi;

pub use app::*;
pub use crux_core::Core;

#[cfg(any(feature = "wasm_bindgen", feature = "uniffi"))]
pub use ffi::CoreFFI;

#[cfg(feature = "uniffi")]
uniffi::setup_scaffolding!();
```

Add `pub mod {capability};` for any custom capability modules.

### 10. Verify

Run `cargo check` in the project directory. If it fails:
1. Read the error output carefully
2. Fix the issue in the relevant file
3. Re-run `cargo check`
4. Repeat until clean

Then run `cargo test` to verify tests pass.

### 11. Lint with clippy

Run `cargo clippy --all-targets`. The workspace lints (`all`, `nursery`, `pedantic`, `cargo`,
plus restriction cherry-picks) are configured in the workspace `Cargo.toml`. Fix all warnings
before proceeding. Common issues:

- `use_self` -- use `Self` instead of the type name inside impl blocks
- `match_same_arms` -- merge arms with identical bodies into one
- `too_many_lines` -- extract helpers or allow on the function with a justification comment
  (the `update()` match dispatch is commonly allowed)
- `unnecessary_map_or` -- use `is_none_or` / `is_some_and` instead of `map_or`
- `implicit_clone` -- use `.clone()` instead of `.to_string()` on `&String`
- `unnested_or_patterns` -- nest patterns: `Event::X(Ok(None) | Err(_))` not
  `Event::X(Ok(None)) | Event::X(Err(_))`
- `needless_pass_by_value` -- take `&T` or `&[T]` when the function doesn't consume ownership
- `doc_markdown` -- use backticks around type names in doc comments (e.g., `` `ViewModel` ``)
- `cargo_common_metadata` -- add metadata or allow if the crate is not published
- `multiple_crate_versions` -- add duplicate crate names to `clippy.toml`
  `allowed-duplicate-crates` when they are transitive and cannot be resolved

### 12. Review for unused dependencies

After the build passes, audit `Cargo.toml` against actual usage:

1. For every non-optional dependency in `[dependencies]`, search `src/` for a corresponding
   `use {crate_name}` or a macro/derive from that crate. Remove any dependency that has no
   matching usage.
2. For every crate in `[dev-dependencies]`, search test modules for usage. Remove any that
   are not referenced.
3. Re-run `cargo check` after removals to confirm nothing was missed.

### 13. Self-review for logic bugs

After all mechanical checks pass, review the generated code for these common logic issues:

1. **State consistency** -- when an event triggers a follow-up event (via `Command::event`),
   verify the model state set before the follow-up is consistent with what the follow-up
   handler expects. Example bug: setting `state = Connected` then dispatching `ConnectSse`
   which sets `state = Connecting`.
2. **Ownership in helpers** -- prefer `&T` and `&[T]` over owned `T` and `Vec<T>` in helper
   function signatures when the function only reads the data (cloning internally as needed).
3. **`expect()` in production paths** -- `expect()` panics like `unwrap()`. Only use it
   for operations that are provably infallible (e.g., serializing a simple
   `#[derive(Serialize)]` struct with no custom serializers). Add a descriptive message.
4. **SSE reconnection flow** -- when SSE disconnects and the app re-fetches state then
   reconnects, ensure the SSE connection state transitions are:
   `Connected → Disconnected → (fetch items) → Connecting → Connected` (on first message).
   Never set `Connected` before the stream is actually producing events.
5. **KV event payload types** -- `KeyValue::set` and `KeyValue::delete` both return
   `Result<Option<Vec<u8>>, KeyValueError>` (the previous value), **not** `Result<(), KeyValueError>`.
   A common mistake is to declare the callback event variant as `Saved(Result<(), KeyValueError>)`
   which causes a type mismatch. Always use `Result<Option<Vec<u8>>, KeyValueError>` for
   `set`, `get`, and `delete` callbacks. Only `exists` returns `Result<bool, KeyValueError>`.
6. **Pending op removal by ID, not index** -- when a queue of pending operations is synced
   one-at-a-time via HTTP, never remove the completed op by position (`pending_ops.remove(0)`).
   Concurrent events (SSE, fetch-all) can `retain(...)` the same op out of the queue while
   the HTTP request is in-flight, shifting indices. Instead, store the in-flight op's item ID
   in a `syncing_id: Option<String>` field on the model, set it in `start_sync`, and use
   `retain(|op| op.item_id() != synced_id)` in the response handler. Clear `syncing_id` on
   both success and error. See `references/crux-app-pattern.md` § "Pending Operation Sync
   Queue" for the full pattern.

## Process: Update Mode

Use this process when `{project-dir}/shared/src/app.rs` already exists. The goal is
to bring an existing implementation into alignment with updated Specify artifacts through
targeted, minimal edits. Never regenerate a file from scratch in update mode.

### U1. Read and analyze the Specify artifacts

Same extraction as create mode step 1. Read `{change-dir}/specs/{module-name}/spec.md`
and `{change-dir}/design.md`. Build the full picture of the desired application state:
app name, features, data model, UI, capabilities, API shapes, and business rules.

### U2. Read existing code

Read every source file in the project:

- `shared/src/app.rs` -- types, events, model, view model, effects, update logic,
  view logic, helper functions, tests
- `shared/src/lib.rs` -- module declarations, re-exports
- `shared/src/ffi.rs` -- `CoreFFI` bridge type
- Any custom capability modules (e.g., `shared/src/sse.rs`)
- `shared/Cargo.toml` -- dependencies and features
- `{project-dir}/Cargo.toml` -- workspace dependencies

### U3. Build implementation inventory

Extract a structured inventory from the existing code. For each category, list every
item by name:

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
| Tests | Test function names, which events they cover | `#[cfg(test)] mod tests` in `app.rs` |

### U4. Diff analysis

Compare the artifact requirements (from U1) against the implementation inventory (from U3).
For each category, classify every item into one of four buckets:

- **Added** -- present in the artifacts but absent from the code. Requires new code.
- **Removed** -- present in the code but absent from the artifacts. Requires deletion.
- **Modified** -- present in both but the artifacts describe different fields, types,
  behavior, or constraints. Requires editing existing code.
- **Unchanged** -- present in both with matching semantics. Leave alone.

Walk through the categories in this order, since later categories depend on earlier ones:

1. **Capabilities** -- added or removed capabilities affect Effect, Event, imports, and deps.
2. **Views** -- added or removed views affect `Page` enum, `Route` enum (if navigable),
   `ViewModel` enum, per-page view structs, `Navigate` handler, and `view()` match arms.
3. **Domain types** -- new or changed structs/enums affect Model, Event payloads, and API shapes.
4. **Model fields** -- new state fields may be needed before events can reference them.
5. **Event variants** -- added/removed/modified user actions and internal callbacks.
6. **Per-page view struct fields** -- changes in what individual views display.
7. **API shapes** -- changed endpoints, request/response bodies.
8. **Business rules** -- changed validation or logic in `update()` arms.
9. **`view()` logic** -- changes driven by view struct, Model, or Page changes.

After completing the diff, output a summary listing every added, removed, and
modified item before making any edits. This summary serves as the edit plan.

### U5. Apply changes to types and structure

Edit `app.rs` to reflect the structural changes identified in U4. Work top-down
through the file:

1. Add, remove, or modify **domain types** (structs, enums, and their fields/variants).
2. Add or remove **Page variants** in `enum Page`, corresponding **ViewModel variants**
   in `enum ViewModel`, and **Route variants** in `enum Route` (for navigable views).
   Add or remove per-page view structs as needed.
3. Add or remove **Model fields** (ensure new fields have `Default` values).
4. Add, remove, or modify **per-page view struct fields**.
5. Add or remove **Event variants** -- new shell-facing variants go in the shell
   section; new internal variants go in the internal section with `#[serde(skip)]`
   and `#[facet(skip)]`.
5. Add or remove **Effect variants** and update capability **type aliases**.
6. Update **imports** at the top of the file for any added or removed capabilities.

If a new capability is added, also update:
- `shared/Cargo.toml` -- add the crate dependency
- `{project-dir}/Cargo.toml` -- add to `[workspace.dependencies]`
- `shared/src/lib.rs` -- add `pub mod {capability};` if it is a custom module

If a capability is removed, reverse those changes.

### U6. Apply changes to logic

Edit the `update()` and `view()` functions in `app.rs`:

1. For **added Event variants**, add new match arms. Consult
   `references/crux-command-api.md` for command patterns and
   `references/crux-capabilities.md` for capability APIs.
2. For **removed Event variants**, delete the match arm.
3. For **modified Event variants**, update the match arm logic to match the new
   artifact requirements.
4. For **changed business rules**, update the relevant match arm logic or helper
   functions.
5. For **changed API shapes**, update HTTP call construction (URL, body struct,
   method) and response handling.
6. Update `view()` if Page variants, ViewModel variants, or per-page view struct
   fields were added, removed, or their derivation from Model changed. Every `Page`
   variant must have a corresponding match arm in `view()`.
7. Add, modify, or remove **helper functions** as needed.

### U7. Update tests

Edit the `#[cfg(test)] mod tests` section in `app.rs`:

1. Add at least one test for every **added** Event variant.
2. Update tests for every **modified** Event variant or business rule.
3. Remove tests for **removed** Event variants.
4. Preserve test utilities (factory functions like `make_item`, setup helpers like
   `seeded_model`) unless the types they construct were changed -- in which case,
   update them to match the new type definitions.
5. If domain types gained or lost fields, update all test code that constructs
   those types.

Consult `references/crux-testing-patterns.md` for testing conventions.

### U8. Verify

Same as create mode steps 10--13:

1. Run `cargo check` -- fix any compilation errors.
2. Run `cargo test` -- fix any test failures.
3. Run `cargo clippy --all-targets` -- fix any warnings.
4. Audit `Cargo.toml` for unused dependencies (especially after removing a
   capability).
5. Self-review for logic bugs (state consistency, ownership, KV payload types,
   pending op removal by ID).

### U9. Final diff review

After all edits and verification pass, do a final review of every changed line.
Confirm:

- No unchanged code was accidentally modified.
- No orphaned types, fields, imports, or test functions remain.
- The code compiles, tests pass, and clippy is clean.

## Artifact-to-Code Mapping

This table shows how each Specify artifact section maps to code constructs. Use it during
update mode diff analysis (step U4) to systematically identify what changed.

| Artifact Section | Code Construct | File(s) | Diff Indicators |
|---|---|---|---|
| **Design** -- Overview | App struct name, `impl App for X` | `app.rs`, `ffi.rs` | Renamed app |
| **Spec** -- Feature requirements | Shell-facing Event variants | `app.rs` `enum Event` | New/removed/renamed user actions |
| **Spec** -- Feature requirements | `update()` match arms | `app.rs` `fn update()` | New/removed/changed handler logic |
| **Design** -- Domain Model (entities) | Domain structs and enums | `app.rs` top section | New/changed/removed fields or types |
| **Design** -- Domain Model (state) | `struct Model` fields | `app.rs` `struct Model` | New/changed/removed state fields |
| **Spec** -- View/page requirements | `enum ViewModel` variants | `app.rs` `enum ViewModel` | New/removed/renamed views |
| **Spec** -- View/page requirements | `enum Page` variants | `app.rs` `enum Page` | New/removed/renamed internal page states |
| **Spec** -- View/page requirements | `enum Route` variants | `app.rs` `enum Route` | New/removed navigable destinations |
| **Spec** -- View/page requirements | `Event::Navigate` match arm | `app.rs` `fn update()` | Changed navigation handling |
| **Spec** -- View/page requirements | `fn view()` match arms | `app.rs` `fn view()` | New/removed page-to-view mappings |
| **Spec** -- View/page requirements | Per-page view struct fields | `app.rs` per-page structs | New/changed/removed display data |
| **Spec** -- View/page requirements | `fn view()` body | `app.rs` `fn view()` | Changed model-to-view mapping |
| **Design** -- Capabilities | `enum Effect` variants | `app.rs` `enum Effect` | Added/removed capabilities |
| **Design** -- Capabilities | Type aliases (`type Http = ...`) | `app.rs` top section | Added/removed aliases |
| **Design** -- Capabilities | Crate dependencies | `shared/Cargo.toml` | Added/removed deps |
| **Design** -- Capabilities | Custom capability modules | `shared/src/*.rs`, `lib.rs` | Added/removed modules |
| **Design** -- Capabilities | Internal Event variants | `app.rs` `enum Event` | Callback variants for added/removed capabilities |
| **Design** -- API Contracts (endpoints) | HTTP call sites in `update()` | `app.rs` `fn update()` | Changed URLs, methods |
| **Design** -- API Contracts (shapes) | Request/response body structs | `app.rs` domain types | Changed fields |
| **Spec** -- Scenario conditions | Validation logic in `update()` | `app.rs` `fn update()` | Changed guards, conditions |
| **Spec** -- Scenario conditions | Helper functions | `app.rs` free functions | Changed conflict resolution, sync logic |

## Update Change Patterns

Common change patterns and which code elements they touch. Use this as a checklist
when applying changes in steps U5--U7.

### Adding a view

1. Add a new variant to `enum Page`.
2. Add a corresponding variant to `enum ViewModel`, wrapping a new per-page view struct
   if the view carries data.
3. Define the per-page view struct with `Facet, Serialize, Deserialize, Clone, Debug,
   Default` derives.
4. If the view is user-navigable (not an internal state like Loading/Error), add a
   variant to `enum Route` and a match arm in the `Event::Navigate(route)` handler.
5. Add a match arm in `view()` that maps the new `Page` variant to the new `ViewModel`
   variant.
6. Add page transition logic in the relevant `update()` arms (`model.page = Page::NewView`).
7. If the view has user interactions, add shell-facing Event variants for them.
8. Write tests verifying the page transition, navigation, and the view output.

### Removing a view

1. Remove all `update()` arms that transition to the removed `Page` variant.
2. If the view had a `Route` variant, remove it from `enum Route` and from the
   `Navigate` match arm.
3. Remove the match arm from `view()`.
4. Remove the `Page` variant.
5. Remove the `ViewModel` variant and its per-page view struct.
6. Remove any Event variants that were exclusive to the removed view.
7. Remove related tests.

### Adding a feature

1. Add a new shell-facing Event variant to `enum Event`.
2. Add a match arm in `update()` with the handler logic.
3. If the feature needs new state, add a field to `Model` (with a `Default` value).
4. If the feature produces new display data, add a field to the relevant per-page
   view struct and update the corresponding match arm in `view()`.
5. Write at least one test for the new Event variant.

### Removing a feature

1. Remove the Event variant from `enum Event`.
2. Remove the match arm from `update()`.
3. Remove any Model fields that are now unused (not referenced by any remaining
   event handler or `view()`).
4. Remove any per-page view struct fields that are now unused.
5. Remove tests for the removed Event variant.
6. Check for helper functions that are now unused and remove them.

### Modifying a feature

1. Update the Event variant payload if the signature changed.
2. Update the match arm logic in `update()`.
3. Update any Model or ViewModel fields affected by the change.
4. Update tests to reflect the new behavior.

### Adding a capability

1. Add the crate to `[workspace.dependencies]` in the workspace `Cargo.toml`.
2. Add the crate to `[dependencies]` in `shared/Cargo.toml`.
3. Add a variant to `enum Effect` for the new capability's operation type.
4. Add a type alias: `type X = crate_name::X<Effect, Event>;`.
5. Add `use` imports for the capability's types.
6. If the capability is custom (not a published crate), create the module file,
   add `pub mod {name};` to `lib.rs`, and add `use crate::{name}::...;` to `app.rs`.
7. Add internal Event variants for the capability's callbacks (with `#[serde(skip)]`
   and `#[facet(skip)]`).
8. Add match arms in `update()` for the new internal Event variants.
9. Write tests for the new capability interactions.

### Removing a capability

Reverse of adding -- remove in this order to avoid compilation errors:

1. Remove match arms for the capability's internal Event variants from `update()`.
2. Remove the internal Event variants from `enum Event`.
3. Remove the type alias.
4. Remove the Effect variant.
5. Remove `use` imports.
6. Remove the crate from `shared/Cargo.toml` and workspace `Cargo.toml`.
7. If custom, delete the module file and remove `pub mod {name};` from `lib.rs`.
8. Remove related tests.

### Changing an API endpoint

1. Update the URL pattern in the HTTP call site within `update()`.
2. Update the HTTP method if it changed (e.g., `Http::post` -> `Http::put`).
3. Update request body structs if the payload shape changed.
4. Update response handling if the response shape changed (may require updating
   the internal Event variant's payload type).
5. Update tests that verify HTTP requests or responses.

### Changing a business rule

1. Locate the match arm(s) in `update()` that enforce the rule.
2. Update the guard condition, validation logic, or helper function.
3. Update or add tests that verify the old and new rule behavior.

## Preservation Rules

In update mode, minimize collateral changes. Follow these rules:

1. **Never regenerate a file from scratch.** Always make targeted edits to existing
   files. The only exception is creating an entirely new file (e.g., a new custom
   capability module that did not exist before).
2. **Preserve helper functions** that serve unchanged spec requirements. Do not
   rename, refactor, or move them unless the spec change requires it.
3. **Preserve test utilities** -- factory functions (e.g., `make_item`), setup
   helpers (e.g., `seeded_model`), and test infrastructure. Update them only if the
   types they construct changed.
4. **Preserve code organization** -- section header comments
   (e.g., `// ── Domain types ──`), module structure, and blank-line grouping.
5. **Preserve `ffi.rs`** unless the App type name changed (which changes the
   `Bridge<AppType>` generic parameter).
6. **Preserve custom capability modules** unless the spec changes their operation
   types or API contract.
7. **Preserve `clippy.toml` and `rust-toolchain.toml`** unless a newly added
   capability introduces duplicate transitive crates or requires new build targets.
8. **Preserve `Cargo.lock`** -- do not delete or manually edit it. Let `cargo`
   update it when dependencies change.
9. **Preserve doc comments and code comments** on unchanged items.
10. **Preserve `#[allow(...)]` attributes** on unchanged functions (e.g.,
    `#[allow(clippy::too_many_lines)]` on `update()`).

## Reference Documentation

Consult these references during generation. Do not deviate from the patterns they describe.

| Reference | Purpose |
|---|---|
| `references/crux-app-pattern.md` | App trait, Model, Event, ViewModel (enum), Page management, Route/Navigate pattern, Effect type conventions |
| `references/crux-command-api.md` | Command creation, chaining, combining, async context |
| `references/crux-capabilities.md` | HTTP and KV capability APIs |
| `references/crux-custom-capabilities.md` | Building custom Operation + capability (SSE example) |
| `references/crux-testing-patterns.md` | Testing effects, events, resolving requests |
| `references/crux-ffi-scaffolding.md` | CoreFFI struct, uniffi, wasm-bindgen |
| `references/crux-project-config.md` | Cargo workspace, toolchain, features, dependencies |

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
| `facet` derive errors | Ensure `facet = "=0.31"` is pinned exactly; add `#[repr(C)]` to enums |
| `uniffi` build failures | Ensure `uniffi` is behind `feature = "uniffi"` gate, not unconditional |
| Missing `Operation` impl for custom capability | Each custom request type must `impl Operation` with `type Output` |
| `#[serde(skip)]` on Event variant causes deserialization panic | Internal variants must never be sent from the shell; guard with `#[facet(skip)]` too |
| KV `set`/`delete` callback type mismatch (`Result<(), _>` vs `Result<Option<Vec<u8>>, _>`) | `KeyValue::set` and `KeyValue::delete` return the previous value as `Result<Option<Vec<u8>>, KeyValueError>`, never `Result<(), _>`. Update the Event variant payload to match. |

## Verification Checklist

Before completing, verify. Items marked **(update)** apply only in update mode;
all other items apply in both modes.

### Build and lint

- [ ] `cargo check` passes with no errors
- [ ] `cargo test` passes with no failures
- [ ] `cargo clippy --all-targets` passes with no warnings
- [ ] Workspace lints (`all`, `nursery`, `pedantic`, `cargo`, restriction cherry-picks)
  are configured in workspace `Cargo.toml` and inherited via `[lints] workspace = true`
- [ ] `clippy.toml` exists with `allowed-duplicate-crates` populated for transitive duplicates

### Types and structure

- [ ] Every Event variant is handled in `update()`
- [ ] Every `update()` branch returns a `Command<Effect, Event>` (not `()`)
- [ ] Internal Event variants have `#[serde(skip)]` and `#[facet(skip)]`
- [ ] `ViewModel` is an enum with `#[repr(C)]` and derives `Facet, Serialize, Deserialize,
  Clone, Debug, Default`
- [ ] Every `Page` variant has a corresponding `ViewModel` variant and a match arm in `view()`
- [ ] Every `Page` variant is reachable by at least one transition in `update()`
- [ ] `Page` and `ViewModel` variants have a 1:1 correspondence
- [ ] `Route` enum exists with variants for user-navigable views (excludes Loading, Error)
- [ ] `Event::Navigate(Route)` variant exists and is handled in `update()`
- [ ] `Navigate` handler is state-aware (considers `model.page` before transitioning)
- [ ] Per-page view structs derive `Facet, Serialize, Deserialize, Clone, Debug, Default`
- [ ] Effect enum uses `#[effect(facet_typegen)]`
- [ ] `CoreFFI` uses feature-gated `uniffi` and `wasm_bindgen` attributes
- [ ] Type aliases defined for each capability: `type Http = crux_http::Http<Effect, Event>;`
- [ ] KV callback Event variants use `Result<Option<Vec<u8>>, KeyValueError>` for `get`/`set`/`delete`
  (not `Result<(), _>`) and `Result<bool, KeyValueError>` for `exists`

### Code quality

- [ ] At least one test per Event variant exists
- [ ] No `unwrap()` or `expect()` in production code (allowed in tests; `expect()` allowed
  only for provably infallible operations like serializing a simple derive struct)
- [ ] No unused dependencies in `Cargo.toml` -- every crate has a matching `use` in `src/`
- [ ] Helper functions take `&T` / `&[T]` unless they need ownership
- [ ] Doc comments use backticks around type and parameter names
- [ ] State transitions are consistent across chained events (no contradictory state before
  a follow-up `Command::event`)
- [ ] Pending ops removed by tracked ID (`syncing_id`), never by index (`remove(0)`)

### Update mode only

- [ ] **(update)** All **added** Event variants from the artifacts are present in `enum Event`
  and handled in `update()`
- [ ] **(update)** All **removed** Event variants are gone from both `enum Event` and
  the `update()` match block
- [ ] **(update)** No orphaned Model fields -- fields removed from the artifacts are deleted
  from `struct Model` and all references
- [ ] **(update)** No orphaned ViewModel variants -- views removed from the artifacts are
  deleted from `enum ViewModel`, `enum Page`, `enum Route` (if navigable), per-page
  view structs, `Navigate` handler, and `view()` match arms
- [ ] **(update)** No orphaned per-page view struct fields -- fields removed from the artifacts
  are deleted from the struct and `view()`
- [ ] **(update)** No orphaned internal Event variants -- if a capability was removed,
  its callback Event variants and match arms are also removed
- [ ] **(update)** No orphaned Effect variants or type aliases for removed capabilities
- [ ] **(update)** No orphaned imports (`use` statements) for removed crates or types
- [ ] **(update)** Tests exist for every **new or modified** Event variant
- [ ] **(update)** Existing passing tests for **unchanged** features still pass
- [ ] **(update)** Preservation rules were followed -- unchanged code, comments,
  helpers, and test utilities were not modified

## Important Notes

- **0.17 is pre-release**: Use git dependencies pinned to the `crux_core-v0.17.0-rc3` tag.
  When 0.17 is published to crates.io, replace git dependencies with versioned ones.
- **`facet` version pinning**: The `facet` crate must be pinned to `"=0.31"` exactly.
  Other versions may be incompatible with `crux_core`.
- **`uniffi` version pinning**: The `uniffi` crate must be pinned to `"=0.29.4"` exactly,
  to match the `uniffi_bindgen` version bundled in `crux_core::cli`. Using a different
  version causes symbol-name mismatches during iOS/Android code generation.
- **Dependency version policy**: Use the latest published version of all Rust crate
  dependencies by default. The exceptions are `facet` (pinned to `"=0.31"`) and `uniffi`
  (pinned to `"=0.29.4"`) which must match versions expected by `crux_core`.
- **No `Capabilities` struct**: The 0.17 API does not use a `Capabilities` struct.
  Define `Effect` directly as an enum with `#[effect(facet_typegen)]`. The `App` trait
  requires `type Effect = Effect;`.
- **`Command` is generic**: Return `Command<Effect, Event>` from `update()`.
- **`#[repr(C)]` on Event enums**: Required by `facet` for enums that cross the FFI boundary.
- **Codegen uses `TypeRegistry`**: The codegen binary uses `crux_core::type_generation::facet::TypeRegistry`
  for compile-time type extraction. Do NOT use `crux_core::cli::run()` which depends on
  `rustdoc-types` and breaks with newer Rust versions.
- **SSE is not a published crate**: It is a custom capability. Generate it inline when needed.
- **Core only**: This skill generates only the `shared` crate. Shell skills are separate.
