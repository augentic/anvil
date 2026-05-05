---
name: omnia-crate-writer
description: "Write Rust WASM crates from Specify artifacts -- greenfield creation or incremental updates -- following Omnia SDK patterns with provider-based dependency injection. Use when implementing crate tasks from a Specify change, regenerating a crate from updated artifacts, or when the user mentions `crate-writer`."
argument-hint: "[crate-name]"
---

# Crate Writer

Write Rust WASM crates from Specify artifacts (specs + design.md), following Omnia SDK patterns for stateless, provider-based WASM components. This skill handles both **greenfield creation** and **incremental updates** to existing crates.

This skill accepts Specify artifacts from any producer:

- **Code-Analysis artifacts** (from `/spec:extract`) -- generates/updates crates from existing source code
- **Feature specs** (from Specify change artifacts) -- updated specs derived from requirements changes

## Critical Path (Quick Reference)

1. **Detect mode**: `$CRATE_PATH/Cargo.toml` exists -> update; missing -> create.
2. **Read** [rules.md](./rules.md) — the Hard Rules and Authority Hierarchy bind every step below.
3. **Read artifacts** (`spec.md`, `design.md`) and required references; pick the matching example under [`examples/`](./examples/).
4. **Derive Omnia capabilities** from design.md (Source Capabilities Summary, External Services, `[runtime]` constraints) via [capability-mapping.md](references/capability-mapping.md) and [wasm-constraints.md](references/wasm-constraints.md); apply artifact corrections (Hard Rule 9) before writing code.
5. **Build the three matrices** (Side-Effect, Outbound Message, Transaction Boundary) for every changed handler; every cell must land in code.
6. **Generate / update code** following the per-mode process below; in update mode apply categories in fixed order: structural → subtractive → modifying → additive.
7. **Smoke check** with `cargo check`, run traceability verification, then inject or update guest wiring (when `src/lib.rs` exists). Tests come from test-writer in a later step.

## Mode Dispatch

| Trigger                          | Mode       | Behaviour                                                                                                |
| -------------------------------- | ---------- | -------------------------------------------------------------------------------------------------------- |
| `$CRATE_PATH/Cargo.toml` missing | **Create** | Greenfield: full crate generation from scratch using the artifacts and references.                      |
| `$CRATE_PATH/Cargo.toml` exists  | **Update** | Incremental: inventory the crate, classify the change set, apply edits in fixed order, preserve unchanged code. |

The binding constraints (Hard Rules and the Authority Hierarchy) that govern every generation pass live in [rules.md](./rules.md). Read it before writing or modifying any code.

## Arguments

```text
$CRATE_NAME     = $ARGUMENTS[0]

# Path derivation
$SLICE_DIR     = .specify/changes/$CRATE_NAME
$SPECS_DIR      = $SLICE_DIR/specs
$DESIGN_PATH    = $SLICE_DIR/design.md
$CRATE_PATH     = crates/$CRATE_NAME
```

## Required References

Before generating or updating code, read these documents:

1. [sdk-api.md](references/sdk-api.md) -- Handler<P>, Context, Reply, IntoBody, Client, Error types
2. [capabilities.md](references/capabilities.md) -- all 9 provider traits with exact signatures and artifact triggers
3. [capability-mapping.md](references/capability-mapping.md) -- mapping from Specify artifact capabilities to Omnia provider traits
4. [wasm-constraints.md](references/wasm-constraints.md) -- translating `[runtime]` constraints to Omnia/WASM patterns
5. [providers.md](references/providers.md) -- Provider struct setup, trait composition rules, MockProvider patterns
6. [error-handling.md](references/error-handling.md) -- error macros, domain error enums, context patterns
7. [guardrails.md](references/guardrails.md) -- WASM constraints and forbidden patterns
8. [cargo-toml.md](references/cargo-toml.md) -- Cargo.toml template and dependency rules
9. [guest-wiring.md](references/guest-wiring.md) -- how crates wire into the WASM guest

**Both modes** -- also read:

10. [checklists.md](references/checklists.md) -- pre-generation and verification checklists
11. [todo-markers.md](references/todo-markers.md) -- TODO marker rules, capability overrides, cache-aside patterns
12. [output-documents.md](references/output-documents.md) -- Migration.md, Architecture.md, CHANGELOG.md, .env.example

**Update mode only** -- also read:

13. [update-patterns.md](references/update-patterns.md) -- update strategy patterns by category
14. [change-classification.md](references/change-classification.md) -- how to classify artifact-vs-code differences

### Examples

**Create mode** (read at least one matching your scenario):

- [single-handler.md](examples/single-handler.md) -- messaging handler crate (like r9k-adapter)
- [multi-handler.md](examples/multi-handler.md) -- multiple HTTP handlers crate (like cars)
- [anti-patterns.md](examples/anti-patterns.md) -- common LLM mistakes with wrong/right pairs
- [capabilities/](examples/capabilities/) -- per-capability worked examples (StateStore, Identity, TableStore, Broadcast, etc.)

**Update mode** (read at least one matching your update scenario):

- [updates/additive.md](examples/updates/additive.md) -- add a new handler to an existing crate
- [updates/modifying.md](examples/updates/modifying.md) -- change business logic in an existing handler
- [updates/subtractive.md](examples/updates/subtractive.md) -- remove an endpoint and its handler
- [updates/structural.md](examples/updates/structural.md) -- refactor a domain model

## Artifact Dispatch

Read design.md Context section to determine origin:

```markdown
## Context

- **Source**: <source-code>
```

### Artifact Mapping

- **design.md Domain Model > Types** -> `src/types.rs` (preserve exact nesting)
- **design.md API Contracts > API Calls** -> `src/handlers.rs`
- **design.md Business Logic** -> domain modules or inline in handler
- **design.md External Services + Source Capabilities Summary + Business Logic cues** -> handler trait bounds (via [capability-mapping.md](references/capability-mapping.md))
- **design.md Implementation Requirements `[runtime]` constraints** -> Omnia patterns (via [wasm-constraints.md](references/wasm-constraints.md))
- **Source paths** -> reference comments (`// Source: $SOURCE_PATH`)
- **`[infrastructure]` steps without Omnia equivalent** -> TODO comment in handler with suggested Omnia approach; documented in Migration.md

## Crate Structure

### Single Handler (messaging adapter, connector)

```
$CRATE_PATH/
├── Cargo.toml
├── src/
│   ├── lib.rs              # Module declarations, error types, re-exports
│   ├── handler.rs          # Handler<P> impl + standalone handle() fn
│   ├── <input_domain>.rs   # Input types (deserialization, validation)
│   └── <output_domain>.rs  # Output types (serialization)
├── tests/
│   ├── provider.rs         # MockProvider implementing required traits
│   └── <test_name>.rs      # Integration tests using Client
├── Migration.md
├── Architecture.md
└── .env.example
```

See [single-handler.md](examples/single-handler.md) for the r9k-adapter example.

### Multi Handler (API crate with multiple endpoints)

**Layout**: Prefer **Multi** handler modules when there are many endpoints/handlers or when handlers share substantial types in the barrel. Use a **barrel + directory** (`src/handlers.rs` + `src/handlers/*.rs`). Flat layout is more idiomatic for small crates and keeps discovery simple.

```
# Flat (preferred for small crates)
$CRATE_PATH/src/
├── lib.rs              # mod r9k; mod smartrak; pub use r9k::*; pub use smartrak::*;
├── r9k.rs   # Handler<P> impl
├── smartrak.rs
└── types.rs            # Shared types

# Barrel + directory (valid for larger crates)
$CRATE_PATH/src/
├── lib.rs
├── handlers.rs         # Barrel + shared types
├── handlers/
│   ├── <endpoint_a>.rs
│   └── <endpoint_b>.rs
└── <utility>.rs
```

See [multi-handler.md](examples/multi-handler.md) for the barrel layout example.

## Handler Pattern

Every handler follows the delegation pattern: request struct implements `Handler<P>`, delegates to a standalone `async fn handle()`.

See [sdk-api.md](references/sdk-api.md) for the Handler trait definition, Input Type Decision Tree, and Response Types (IntoBody).

**Never** use `type Input = MyRequest` -- this bypasses deserialization and is incompatible with the Omnia runtime.

## Error Handling

Domain errors use `thiserror` and convert to `omnia_sdk::Error` via `From<DomainError>`. Use error macros for one-off errors: `bad_request!("msg")`, `server_error!("msg")`, `bad_gateway!("msg")`.

See [error-handling.md](references/error-handling.md) for domain error patterns, macro usage, context chaining, validation placement rules, timestamp semantics, and serde conventions.

**Critical**: Never use `Utc::now()` in `from_input()` -- the test framework's `shift_time` cannot fix validation at parse time.

## Test Generation

Tests are generated separately by test-writer. crate-writer does not generate tests. The build orchestration layer runs test-writer after crate-writer completes, then runs a unified verify-repair loop across both code and tests.

## Guest Wiring (Conditional)

**Trigger**: only when `src/lib.rs` exists.

After generating or updating the crate, inject or update wiring in the guest project. See [guest-wiring.md](references/guest-wiring.md) for templates.

### What to Inject (create mode)

1. `use $crate_name::{...};` import for handler types
2. Axum route entries (HTTP handlers)
3. Topic match arms (messaging handlers)
4. WebSocket handler delegation (WebSocket handlers) -- add delegation inside existing WebSocket Guest impl, or create the full WebSocket Guest export block if none exists
5. Handler functions with `#[omnia_wasi_otel::instrument]`
6. Provider trait impls if new capabilities needed
7. Crate dependency in `Cargo.toml`

### Guest Wiring by Category (update mode)

| Category        | Guest Wiring Action                                     |
| --------------- | ------------------------------------------------------- |
| **Additive**    | Append new routes/topics/imports (append-only pattern)  |
| **Subtractive** | Remove routes/topics/imports for deleted handlers       |
| **Modifying**   | Update route paths, HTTP methods, or handler signatures |
| **Structural**  | Update import names after type/module renames            |

### Rules (both modes)

- Append only in create mode -- do not replace or reorder existing content
- No duplicates -- skip if route/topic/WebSocket handler/import already exists
- All handler functions get `#[omnia_wasi_otel::instrument]`
- Update Provider trait impls if capabilities changed
- Update `ensure_env!` entries for config key changes

Before starting code generation, verify artifact completeness per [checklists.md](references/checklists.md#pre-generation-checklist). If any item is NO or UNCLEAR, mark with TODO in generated code and note it in Migration.md.

---

## Mode: Create

### Generation Process

1. Read Specify artifacts from `$SLICE_DIR`:
   - Read the spec file from `$SPECS_DIR/$CRATE_NAME/spec.md` (single consolidated file with flat `### Requirement:` / `#### Scenario:` blocks)
   - Read design.md from `$DESIGN_PATH`
2. **Derive Omnia capabilities from artifacts:**
   - Read the design.md **Source Capabilities Summary** checklist and map each checked capability to an Omnia provider trait using [capability-mapping.md](references/capability-mapping.md).
   - Read the design.md **External Services** and cross-reference service types against the mapping table. Verify that SQL databases map to `TableStore`, Azure Table Storage and document databases (Cosmos DB, MongoDB) map to `DocumentStore`, blob stores (Azure Blob Storage, AWS S3) map to `Blobstore`, caches map to `StateStore`, etc.
   - Read the design.md **Implementation Requirements** `[runtime]` constraints and translate each to an Omnia pattern using [wasm-constraints.md](references/wasm-constraints.md).
   - Scan design.md **Business Logic** for data access phrasing (`Table access:`, `Cache:`, `Document:`, `Blob:`) and map to appropriate traits.
3. **Artifact correction — fix known misassignments before generating** (SKILL.md > artifacts per authority hierarchy):
   - If design.md External Services lists a SQL database but the Source Capabilities Summary does not check `Table/database access`: **add `TableStore`** to the derived traits.
   - If design.md External Services lists Azure Table Storage, Cosmos DB document API, or MongoDB but the capabilities do not include document access: **add `DocumentStore`** to the derived traits.
   - If design.md External Services lists Azure Blob Storage or AWS S3 but the capabilities do not include blob access: **add `Blobstore`** to the derived traits.
   - If any algorithm step phrases managed data store access as an HTTP call: **override to the correct trait** (`TableStore` for SQL, `DocumentStore` for document/table stores, `Blobstore` for blob stores).
   - If the artifacts describe pre-populating a cache via external cron/ETL for data the source loads on startup: **override to on-demand cache-aside** (StateStore + data source trait).
4. Determine artifact origin from design.md Context section
5. Read reference documents from `references/`
6. Read matching example from `examples/`
7. **Cross-Cutting Analysis** -- before generating any handler code, build three matrices from the spec and design artifacts. These matrices are working artifacts (not persisted) but every cell must be satisfied in the generated code. If a cell cannot be implemented, mark it with a TODO per the todo-markers rules.

   **a. Side-Effect Matrix**

   For every handler that performs write operations (e.g., HTTP POST/PUT/PATCH/DELETE endpoints, message-triggered handlers that insert or update data), read the design.md Business Logic section and list every entity the handler must read or mutate *beyond its primary entity*. Include cross-handler delegations where one handler invokes or depends on another handler's write path.

   | Handler | Primary Entity | Cross-Entity Read | Cross-Entity Mutation | Spec Reference |
   |---------|---------------|-------------------|----------------------|----------------|

   Every cell in the Cross-Entity Mutation column becomes a mandatory code path in the generated handler. If a handler's Business Logic references another entity's data, that reference MUST appear in the generated code -- even if the handler could function without it on the "happy path."

   **b. Outbound Message Matrix**

   For every event or notification published as a side effect in design.md, compare the outbound payload shape against the primary entity's API response shape. If they differ, document the transformation (field additions, removals, renames). Each transformation becomes a dedicated serialization function -- never serialize the entity struct directly for outbound messages unless the shapes are confirmed identical.

   | Topic | Source Entity | Stripped Fields | Added Fields | Transform Function | Spec Reference |
   |-------|-------------|----------------|--------------|-------------------|----------------|

   **c. Transaction Boundary Matrix**

   For every handler whose Business Logic contains multiple sequential write operations (inserts/updates, or delegated calls to other handlers that write), identify whether the spec requires atomicity (look for REQ references to transactions, "all-or-nothing" language, multi-entity consistency requirements, or post-commit-only side effects).

   | Handler | Write Operations | Atomic? | Post-Commit Side Effects | Spec Reference |
   |---------|-----------------|---------|--------------------------|----------------|

   Every row with Atomic=Yes MUST generate transaction-scoped wrapping for its write operations, with event/notification publishes occurring only after successful commit.

8. Run pre-generation checklist above (verify artifact completeness)
9. Generate `Cargo.toml` (see [cargo-toml.md](references/cargo-toml.md))
10. Generate `src/lib.rs` with module declarations and re-exports
11. Generate `src/types.rs` or domain type modules
12. Generate `src/handlers.rs` (or `src/handler.rs` for single handler) -- consult the three matrices from step 7 while generating each handler to ensure cross-cutting concerns are wired
13. Generate domain-specific modules as needed
14. Generate `Migration.md`, `Architecture.md`, `.env.example`
15. Run `cargo check` as a smoke check (full verification runs at the orchestration level after test-writer completes)
16. **Traceability Verification** -- verify that every spec requirement and design.md Business Logic step has a corresponding code path in the generated crate. For each `### Requirement:` block in spec.md:
    - Verify a traceability comment referencing the requirement ID exists in the generated code
    - For each `#### Scenario:` under that requirement, verify that the described behavior has a corresponding branch or code path in a handler

    For each row in the Side-Effect Matrix (step 7a):
    - Verify that every Cross-Entity Mutation has a corresponding function call in the handler

    For each row in the Outbound Message Matrix (step 7b):
    - Verify that the transform function exists and is called before publishing

    For each row in the Transaction Boundary Matrix (step 7c) where Atomic=Yes:
    - Verify that transaction-scoped wrapping encloses the handler's write operations and that post-commit side effects are outside the transaction

    If any verification fails: implement the missing code path before proceeding. Do not rely on the verify-repair loop or test-writer to catch these -- the code must satisfy the spec before handoff.

    After implementing any missing code paths, re-run `cargo check` to verify the new code compiles.

17. If `src/lib.rs` exists: inject guest wiring (see Guest Wiring section above)

---

## Mode: Update

### Update Scope

Four categories of change, ordered by application priority:

| Category        | Description                                                                | Examples                                                                                                            | Complexity |
| --------------- | -------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------- | ---------- |
| **Structural**  | Changes to domain model relationships, type renames, handler splits/merges | Rename `OrderEvent` to `PurchaseEvent` across all files; split a multi-handler module; merge two handlers into one  | High       |
| **Subtractive** | Removal of handlers, endpoints, types, or features                         | Remove a deprecated endpoint; delete unused types; remove a topic handler                                           | Medium     |
| **Modifying**   | Changes to existing business logic, validation, types, or provider bounds  | Add a field to an existing type; change validation threshold; add a new provider trait bound; update error handling | Medium     |
| **Additive**    | New handlers, endpoints, types, or features added to an existing crate     | Add a new HTTP handler; add a new domain type; add a new test                                                       | Low        |

### Update Process

Apply changes in fixed order — **structural → subtractive → modifying → additive** — to minimise intermediate breakage: type renames propagate first, dead code is removed before any new code, and additive code depends on the already-updated type system.

Before starting, read every document listed in [Required References](#required-references) (including the update-specific entries) and at least one matching update example.

#### Step 1: Inventory Existing Crate

Parse the existing crate to build a structural inventory mapping artifact concepts to file locations:

| Source                                                      | What to Extract                                                                               |
| ----------------------------------------------------------- | --------------------------------------------------------------------------------------------- |
| `Cargo.toml`                                                | Crate name, dependencies, features                                                            |
| `src/lib.rs`                                                | Module declarations, re-exports, error type definitions                                       |
| `src/handler.rs` or `src/handlers.rs` + `src/handlers/*.rs` | Handler implementations, provider trait bounds, input types, `from_input` patterns            |
| `src/types.rs` and domain modules                           | Type definitions, serde attributes, newtypes, enums                                           |
| `src/lib.rs` (guest, if exists)                             | Routes, topic arms, WebSocket handlers, imports, Provider trait impls                         |

The inventory is an in-memory working model, not a persisted artifact. For each item, record:

- **Concept**: handler name, type name, endpoint path, topic, etc.
- **File**: path relative to `$CRATE_PATH`
- **Lines**: approximate line range
- **Signature**: handler trait bounds, type fields, serde attributes

#### Step 2: Derive Change Set

Read the updated artifacts from `$SLICE_DIR` (specs and design.md) and compare them against the inventory:

| Artifacts vs Inventory                                                             | Classification  |
| ---------------------------------------------------------------------------------- | --------------- |
| Handler/endpoint in artifacts but not in inventory                                 | **Additive**    |
| Handler in both, but business logic, input/output types, or provider bounds differ | **Modifying**   |
| Handler/endpoint in inventory but not in artifacts                                 | **Subtractive** |
| Type renamed, relationships changed, handler split/merged                          | **Structural**  |
| Type in artifacts but not in inventory                                              | **Additive**    |
| Type in both but fields/attributes differ                                          | **Modifying**   |
| Type in inventory but not in artifacts                                              | **Subtractive** |
| Config key in artifacts but not in `.env.example`                                  | **Additive**    |
| Config key in `.env.example` but not in artifacts                                  | **Subtractive** |

See [change-classification.md](references/change-classification.md) for detailed classification rules and edge cases.

#### Step 2a: Cross-Cutting Analysis (changed handlers only)

For every handler that is classified as **Additive** or **Modifying** in the change set, build the same three matrices as Create mode step 7. Also include any unchanged handler whose cross-cutting behavior depends on a modified entity or handler (e.g., an unchanged handler that reads or mutates an entity whose schema changed).

- **Side-Effect Matrix** -- list cross-entity reads and mutations for each changed handler
- **Outbound Message Matrix** -- list payload transformations for each changed handler's outbound messages
- **Transaction Boundary Matrix** -- identify atomicity requirements for each changed handler's write sequences

See step 7 in the Create mode Generation Process for the full matrix definitions. Every cell in these matrices must be satisfied in the updated code.

#### Step 3: Generate Update Plan

For each change, determine the specific edit operations. The plan is a structured list:

```text
STRUCTURAL (apply first):
  1. Rename OrderEvent → PurchaseEvent
     - src/types.rs: lines 15-30 (struct definition)
     - src/handler.rs: lines 45, 67 (references)

SUBTRACTIVE (apply second):
  2. Remove GET /legacy-status endpoint
     - src/handlers/legacy_status.rs: delete file
     - src/handlers.rs: remove mod + pub use
     - guest src/lib.rs: remove route + import

MODIFYING (apply third):
  3. Add `priority` field to WorksiteRequest
     - src/handlers/worksite.rs: lines 20-28 (struct definition)
     - src/handlers/worksite.rs: lines 45-60 (filter builder)

ADDITIVE (apply last):
  4. Add POST /worksite handler
     - src/handlers/create_worksite.rs: new file
     - src/handlers.rs: add mod + pub use
     - guest src/lib.rs: add route + import
```

Log the plan for traceability. Do not modify any files in this step.

#### Step 4: Apply Changes by Category

Execute the plan in the fixed order: structural, subtractive, modifying, additive.

**Structural Changes**: Rename types, modules, or restructure relationships. After completing all structural changes:
- Run `cargo check` to verify compilation
- Re-scan the crate to update the inventory ([rules.md](./rules.md) Hard Rule 16)
- Proceed only if compilation passes
- Patterns: See [update-patterns.md](references/update-patterns.md#structural-patterns)

**Subtractive Changes**: Remove handlers, types, and guest wiring for features no longer in the artifacts:
1. Remove handler implementation files (or handler functions from shared files)
2. Remove corresponding type definitions (only if not used by remaining handlers)
3. Remove module declarations from `lib.rs` or barrel modules
4. Remove unused dependencies from `Cargo.toml`
5. Document each removal in CHANGELOG.md ([rules.md](./rules.md) Hard Rule 13)
- Patterns: See [update-patterns.md](references/update-patterns.md#subtractive-patterns)

**Modifying Changes**: Update existing handler logic, types, or provider bounds:
1. Update type definitions (fields, serde attributes, derive macros)
2. Update handler business logic to match updated artifacts
3. Update provider trait bounds if new capabilities are needed
4. Update `from_input()` for structural validation changes
5. Update `handle()` for temporal/contextual validation changes
6. Preserve function signatures where possible; when signatures change, update all call sites
- Patterns: See [update-patterns.md](references/update-patterns.md#modifying-patterns)

**Additive Changes**: Add new handlers and types following the create-mode patterns exactly:
1. Generate new handler files following the Handler pattern
2. Generate new type definitions
3. Add module declarations to `lib.rs` or barrel modules
4. Add dependencies to `Cargo.toml`
- Patterns: See [update-patterns.md](references/update-patterns.md#additive-patterns)

#### Step 5: Update Guest Wiring

If `src/lib.rs` exists, apply guest wiring changes per the Guest Wiring by Category table above.

#### Step 6: Smoke Check

Run `cargo check` as a quick sanity check after applying all changes.

#### Step 7: Traceability Verification (changed handlers only)

For every handler classified as **Additive** or **Modifying** in the change set, verify that the updated code satisfies the spec and cross-cutting matrices from Step 2a:

- For each spec requirement and scenario that maps to a changed handler, verify a corresponding code path exists
- For each row in the Side-Effect Matrix (Step 2a), verify cross-entity mutations are implemented
- For each row in the Outbound Message Matrix (Step 2a), verify transform functions exist and are called
- For each row in the Transaction Boundary Matrix (Step 2a) where Atomic=Yes, verify transaction-scoped wrapping is in place

If any verification fails: implement the missing code path and re-run `cargo check` to verify the new code compiles.

Full verification (fmt, clippy, test suite, regression detection) runs at the orchestration level after test-writer completes.

---

## Outputs & Quality

| Topic                    | Reference                                                                                                                                                              |
| ------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| TODO markers             | [todo-markers.md](references/todo-markers.md) — marker format, capability overrides, cache-aside patterns. Never silently drop artifact steps; mark at call site and document in Migration.md. |
| Verification checklist   | [checklists.md](references/checklists.md#verification-checklist) — compilation, handler compliance, artifact fidelity, type quality, guest wiring, update-mode checks. |
| Output documents         | [output-documents.md](references/output-documents.md) — Migration.md, Architecture.md, CHANGELOG.md (update mode), `.env.example`.                                     |
| Troubleshooting          | [error-handling.md](references/error-handling.md#troubleshooting) — common issues and resolutions in both modes.                                                       |

Only emit `.rs` source files, `Cargo.toml`, and the required docs. Never emit `target/`, `Cargo.lock`, or build artifacts. Test verification runs at the orchestration level after test-writer completes.

## Important Notes

- Mode is auto-detected from `$CRATE_PATH/Cargo.toml`; tests are test-writer's responsibility (a unified verify-repair loop runs after both writers).
- In update mode, apply categories in fixed order (structural → subtractive → modifying → additive) and re-inventory after structural changes; if an artifact section already matches the existing code, do nothing.
