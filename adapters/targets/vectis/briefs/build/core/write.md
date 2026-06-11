# Vectis build — core (write)

Loaded by [../../build.md](../../build.md) Step 4. Generates or updates the Crux shared Rust core under `${PROJECT_DIR}/shared/`. Run inside its own sub-agent with a clean context window.

The Crux 0.17 idioms and the artifact-to-code mapping live in the [`../../../references/crux/`](../../../references/crux/) reference shelf.

## Mode detection

Inspect `${PROJECT_DIR}/shared/src/app.rs`:

- Missing → **create mode**: render the scaffold with `specify tool run vectis -- scaffold core <APP_NAME> [--caps <csv>]`, then enter update mode for feature-specific code.
- Present → **update mode**: diff the artifact-derived target against the existing implementation and apply targeted edits.

Repair sub-agent (invoked by the verify-repair loop in [../test.md](../test.md)) uses `mode: repair` plus the failing error output to apply the minimum change to fix the reported errors.

## Critical path

1. **Read inputs.** `${SLICE_DIR}/specs/${DOMAIN_NAME}/spec.md` (core body + platform sections), `${SLICE_DIR}/design.md` (Domain Model, Adapters, API Contracts, Implementation Constraints). Extract App name, Model, Events, ViewModel / Page / Route, capability set, and any HTTP / SSE / KV shapes.
2. **Detect mode.** In create mode, render the scaffold via `specify tool run vectis -- scaffold core <APP_NAME> --caps <comma-separated-caps> [--version-file <path>]` and run an explicit `cargo check --workspace` sanity gate before any further edits.
3. **Build an implementation inventory** of existing types and diff against the artifact-derived target — Added / Removed / Modified / Unchanged — per category in dependency order: capabilities → views → domain → model → events → API → logic. The full mapping rules live in [`crux/artifact-to-code-mapping.md`](../../../references/crux/artifact-to-code-mapping.md) and [`crux/update-change-patterns.md`](../../../references/crux/update-change-patterns.md).
4. **Apply structural edits** to `app.rs`: domain types → `Page` / `ViewModel` / `Route` → `Model` → `Event` / `Effect` → imports → `Cargo.toml` updates for new capabilities. Adopt screen names, ViewModel variants, per-page view structs, field names, and `Event` / `Route` variants verbatim from `design.md`.
5. **Apply logic edits** to `update()` and `view()`: per-`Event` match arms, business rules from the spec, model-to-ViewModel mapping for new pages. Consult the Crux 0.17 surface (see [`crux/app-pattern.md`](../../../references/crux/app-pattern.md), [`crux/command-api.md`](../../../references/crux/command-api.md), [`crux/capabilities.md`](../../../references/crux/capabilities.md), [`crux/custom-capabilities.md`](../../../references/crux/custom-capabilities.md)): return `Command<Effect, Event>` from `update()`; mark `Event` enums `#[repr(C)]`; never define a `Capabilities` struct (the 0.17 API uses `Effect` directly as an enum with `#[effect(facet_typegen)]`); never call `crux_core::cli::run()` (use `crux_core::type_generation::facet::TypeRegistry` instead); generate SSE inline as a custom adapter — not a published crate.
6. **Generate shared component helpers.** When `CATALOG_PATH` exists, for each `confirmed` catalog entry referenced by a `component: <slug>` directive in the regenerated `composition.yaml`, generate a shared view-model helper module at `${PROJECT_DIR}/shared/src/components/<slug>.rs`. Each module encapsulates the ViewModel fragment, event variants, and model-to-ViewModel mapping logic for that component so per-screen `view()` code invokes the shared helper instead of inlining the mapping. Add a `mod components;` declaration in `lib.rs` (or `app.rs` depending on the crate layout) and re-export each component module. In update mode, diff existing component modules and apply targeted edits — never regenerate from scratch. When `CATALOG_PATH` is absent, skip this step. **Retroactive factoring (B7).** When the catalog gains a component newly `confirmed` this build that is referenced by baseline screens *outside the current slice's domains* (composition.md Step 6a attached the `component: <slug>` directive to those prior screens via `delta.modified`), the shared module must be generated **and** the affected prior screens' generated `view()` code refactored to consume it in place of the inlined mapping. The writers already run in `update` mode against the live shell tree, so editing prior-slice screens is in scope here. Because the skeletons are identical by construction, the refactor is behaviour-preserving; the verify-repair loops in [../test.md](../test.md) catch any regression. Idempotent: a prior screen already delegating to the shared helper needs no further edit.
7. **Run `cargo check`** as a sanity gate. Full clippy / test / regression runs happen at orchestration level in [`../test.md`](../test.md).
8. **Preserve helpers, comments, custom adapter modules, and `Cargo.lock`.** Never regenerate a file from scratch in update mode. Never hand-edit Cargo dependency versions — the scaffold tool owns version pins so `crux_core`'s bundled `uniffi_bindgen` matches the runtime `uniffi` crate. Never write tests in this sub-agent ([`../test.md`](../test.md) owns them). Never generate shell code (the per-platform write sub-briefs own them).

## Hard rules

Full set at [`hard-rules-core.md`](../../../references/hard-rules-core.md). Highlights:

- Stay inside `app.rs` for domain + state-machine code; helpers live in adapter modules.
- Generated-type conventions (`#[repr(C)]`, `#[derive(Facet)]`, kebab-case → PascalCase via uniffi rules) per [`crux/generated-type-conventions.md`](../../../references/crux/generated-type-conventions.md).
- The `Effect` enum carries `#[effect(facet_typegen)]`; never reintroduce the retired `Capabilities` struct.

## Worked examples

- [`examples/core/01-simple-counter.md`](../../../references/examples/core/01-simple-counter.md) — minimal `Model` + `Event` + `update()`.
- [`examples/core/02-http-counter.md`](../../../references/examples/core/02-http-counter.md) — HTTP capability + custom adapter.
- [`examples/core/03-kv-notes.md`](../../../references/examples/core/03-kv-notes.md) — KV capability + multi-screen ViewModel.
