---
name: reverse-spec
description: Reverse-engineer Specify artifacts (specs + design.md) from existing source code with iterative validation. Use when the proposal source is a local path to existing code, or when /spec:define is given a source-code path to analyze.
license: MIT
argument-hint: "[source-path] [change-dir]"
---

# Reverse Spec

Analyze an existing codebase to produce reconstruction-grade Specify artifacts (specs + design.md) with iterative validation until convergence.

Unlike single-pass analyzers, this skill runs user-confirmed checkpoints and a validation loop that compares every generated artifact against the source code until zero critical discrepancies remain.

**Cardinal Rule**: NEVER assume, infer, or hallucinate. If anything is unclear, ASK the user. Use `[unknown]` tokens for anything the code cannot answer. The cost of asking is low. The cost of a wrong assumption is an incorrect specification that propagates through all generated artifacts.

## Derived Arguments

1. **Source Path** (`$SOURCE_PATH`): Path to the source codebase (the source of truth)
2. **Change Directory** (`$CHANGE_DIR`): Specify change directory (e.g., `.specify/changes/my-api/`)

```text
$SOURCE_PATH = $ARGUMENTS[0]
$CHANGE_DIR  = $ARGUMENTS[1]
$SPECS_DIR   = $CHANGE_DIR/specs
$DESIGN_PATH = $CHANGE_DIR/design.md
```

## Workflow

```
Phase 1  STRUCTURAL INVENTORY ──► user validates completeness
Phase 2  CLARIFICATION ──────────► user answers batch questions
Phase 3  DOMAIN-BY-DOMAIN ───────► depth-first extraction
Phase 4  WRITE ARTIFACTS ────────► specs + design.md
Phase 5  VALIDATION LOOP ────────► compare ↔ source until convergence
Phase 6  USER SIGN-OFF ──────────► confirmation before handoff
```

---

## Phase 1: Structural Inventory

Build a complete inventory of `$SOURCE_PATH` before deep analysis.

### 1.1 Scan

Read the source and extract exact names, types, and locations for:

| Category | Extract |
|----------|---------|
| **Types** | Every struct/enum/class/interface with every field name, type, optionality, serialization attributes, wire names, AND all generated trait/interface implementations (e.g., Rust derives, C# attributes, TypeScript decorators) |
| **Handlers** | Every route with HTTP method, path, input type, output type |
| **External calls** | Every outbound HTTP call with URL pattern, method, headers, auth |
| **Config variables** | Every env var / config key, captured verbatim |
| **Validation rules** | Every input validation check with the exact condition |
| **Error types** | Every error variant with status code and trigger |
| **Shared utilities** | Helper functions used across handlers — document each function's behavior independently (never say "behaves like X" without verifying every code path) |
| **Dependencies** | External package dependencies with **exact versions** from manifest AND lock file (resolved versions) |
| **Guest/entry-point** | Middleware (CORS, auth), error code → HTTP status mapping, body injection/transformation, parameter sourcing (`owner` values), and any validation performed before the domain handler |

### 1.1b Dependency Version Pinning

Dependency version drift is the leading cause of build failures when regenerating from a specification. New versions of the same package frequently introduce breaking API changes (renamed types, moved modules, changed method signatures, removed re-exports).

**Capture dependency versions from the source project's lock file, not just the manifest.** The lock file records the exact versions the source was built and tested against. The manifest may use loose version ranges that resolve differently at build time.

| Stack | Manifest | Lock File | Version Source |
|-------|----------|-----------|----------------|
| Rust | `Cargo.toml` | `Cargo.lock` | Lock file |
| Node/TypeScript | `package.json` | `package-lock.json` / `yarn.lock` / `pnpm-lock.yaml` | Lock file |
| Python | `pyproject.toml` / `setup.cfg` | `poetry.lock` / `requirements.txt` (pinned) | Lock file or pinned requirements |
| C# | `.csproj` | `packages.lock.json` | Lock file |
| Go | `go.mod` | `go.sum` | `go.mod` (already pinned) |
| Java/Kotlin | `pom.xml` / `build.gradle` | Dependency tree output | Resolved dependency tree |

For each dependency, record:
- Package name
- **Exact version** from lock file (e.g., `1.4.0`, not `^1.4`)
- Whether it is a direct or transitive dependency
- Any feature flags / optional features enabled

In the design.md Dependencies section, list the **manifest version specifier** (e.g., `"1.0.100"` from Cargo.toml, `"^2.3.0"` from package.json, `">=1.5,<2.0"` from pyproject.toml) as the primary version — this is what goes into the generated project's dependency declaration. Also note the lock file resolved version for API compatibility reference. Using lock file resolved versions as dependency specifiers overstates the minimum version requirement.

**When the lock file is absent**: Use the manifest version constraints and flag this in Risks / Open Questions. The build phase should use the lower bound of any version range to minimize API drift.

### 1.2 Present and Validate

Present the inventory as structured tables. Ask:

> "I've identified N types, M handlers, K external calls, and J config variables. Is this complete? Are there any areas I've missed or misidentified?"

**HALT until the user confirms the inventory is complete.**

### 1.3 Batch Clarifying Questions

For items where the code alone does not explain the **why**, collect ALL questions and present them in a single batch:

- "What is the purpose of config variable `X`?"
- "Handler `X` calls two APIs — are these the same service or different?"
- "This enum has integer values but no semantic names — what do they mean?"
- "Type `X` overlaps with type `Y` — are these the same domain concept?"

---

## Phase 2: Clarification Checkpoint

Before deep analysis, verify understanding of the project architecture. Use the AskQuestion tool if available, otherwise ask conversationally:

1. **Auth model** — What authentication tiers exist? (public, customer, admin)
2. **External services** — What are the external systems, their names, and roles?
3. **Naming conventions** — Wire format (camelCase?), internal (snake_case?)
4. **Domain boundaries** — What functional areas exist? (account, history, payments...)
5. **Intentional deviations** — Any areas where code intentionally differs from ideal design?

**HALT until clarifications are received.**

---

## Phase 3: Domain-by-Domain Extraction

Analyze the codebase **depth-first by functional domain**, not breadth-first.

### Why Depth-First

Breadth-first (scanning all handlers superficially) misses cross-cutting details like shared validation patterns, common header construction, and utility function behavior. Each domain is fully analyzed before moving to the next.

### Per-Domain Process

For each functional domain:

1. **Read ALL types** — capture every field, serialization attribute, wire name, optionality. Copy definitions verbatim from source; do NOT paraphrase or write from memory.

2. **Read EACH handler** — for each handler:
   - Input type and deserialization method
   - Validation logic (checks, order, error messages, guard conditions like `is_none()` vs `is_none_or(String::is_empty)`)
   - URL construction (base URL config key, path segments, query params, URL encoding)
   - Headers (static and dynamic, custom headers)
   - Auth method (API key, Bearer token, identity source)
   - Response parsing (deserialization, field mapping, error handling)
   - Business logic (transformations, conditional branches, loop error granularity)

3. **Read shared utilities** used by this domain.

4. **Cross-reference** — verify every type field referenced in handler logic exists in the type definition, and vice versa.

5. **Orchestration handlers** — when multiple handlers target the same upstream API, document each handler's request body construction INDEPENDENTLY:
   - Exact format strings for generated IDs (e.g., `"prefix-{id}-suffix"`)
   - Wire format differences (flat vs wrapped structures targeting the same endpoint)
   - Body fields set to null/default — document explicitly even when identical to another handler
   - Conditional field values (e.g., `adjustment_amount = None` for full operations vs `Some(value)` for partial)

6. **Secondary/audit API calls** — all outbound calls (including best-effort, non-critical, audit writes) must document exact request bodies with vendor-specific field names. "Best-effort" does not mean "under-specified."

7. **Response type ownership** — track which module/file defines the canonical serialization implementation for each response type. When multiple handlers share a response type, only one should contain the impl. Document this in a deduplication table.

### Type Extraction Rules

Type mismatches were the single largest source of errors in previous reverse-spec work. See [lessons-learned.md](references/lessons-learned.md) for details.

- Copy type definitions verbatim — never hand-write from memory
- Capture exact types (e.g., `i32` vs `i64`, `int` vs `long`, `number` vs `string`)
- Capture ALL generated trait/interface implementations and annotations — not just serialization ones. Missing equality implementations (Rust `PartialEq`/`Eq`, C# `IEquatable`, Python `__eq__`) cause build failures when code uses `==` comparison
- Capture exact serialization attributes per stack — at BOTH struct/class level AND field level:
  - Rust: `serde` attrs (`rename`, `rename_all`, `default`, `skip_serializing_if`, `skip_serializing`, `deserialize_with`, `alias`)
  - C#: `JsonPropertyName`, `JsonIgnore`, `JsonConverter`
  - TypeScript: class-transformer/class-validator decorators
  - Python: Pydantic `Field(alias=...)`, `model_validator`
- For field-level renames: check for keyword collisions (`type`, `class`, `import`, `return`, etc.) where the implementation language uses a different identifier but maps to the original name via rename attribute. These are CRITICAL for wire compatibility
- For deserialization aliases: check for fields that accept multiple wire names (e.g., both `maskedPan` and `maskedPAN`). Missing aliases cause deserialization failures with real upstream data
- For unconditional serialization skips: distinguish between conditional skip (`skip_serializing_if = "is_none"`) and unconditional skip (`skip_serializing` / `JsonIgnore`). An unconditional skip strips the field entirely from output — omitting this changes the response shape
- For collection/array fields: explicitly note which have default-when-absent behavior and which do NOT — do not assume a universal pattern
- For types with custom deserialization: note that they should NOT also use generated/derived deserialization to avoid conflicting implementations
- For empty/marker types (no fields): note the type shape explicitly
- For enums: variant names AND serialization representation (string, integer, etc.)
- For nested types: follow every level of nesting
- For custom deserializers/converters: document exact behavior
- Check wire names by applying the project's naming convention rules (e.g., `camelCase`, `snake_case`, `PascalCase`). Flag cases where field names diverge from the convention producing unexpected wire names
- When multiple types share field names: document each type's field type SEPARATELY in the cross-type table — never merge columns for types with different field sets

---

## Phase 4: Write Artifacts

Write specs and design.md to `$CHANGE_DIR`.

### 4a: Write Spec File

Write `$SPECS_DIR/$CRATE_NAME/spec.md` using flat baseline format:

1. `## Purpose` — 1-2 sentence description
2. `### Requirement: <name>` blocks with `ID: REQ-XXX`, source traceability, and `#### Scenario:` entries using WHEN/THEN format
3. `## Error Conditions` — shared error types with triggers
4. `## Metrics` — metric names, types, emission points

Rules:
- One requirement per distinct behavior
- Use `SHALL`/`MUST` for normative language
- Sequential IDs: `REQ-001`, `REQ-002`, ...
- Source traceability for every requirement

### 4b: Write design.md

Write `$DESIGN_PATH` following the design instruction template:

1. **Context** — source path, purpose, source type: `source-code`
2. **Domain Model** — type tables with columns: Field, Type, Wire Name, Optional?, Serde Attributes; separate tables for field-level renames, aliases, unconditional skips, and conditional skips; deduplication table for shared response types
3. **API Contracts** — method, path, input type, output type, auth
4. **External Services** — name, type, auth, operations
5. **Constants & Configuration** — config keys verbatim, defaults
6. **Business Logic** — tagged pseudocode per handler: `[domain]`, `[infrastructure]`, `[mechanical]`
7. **Implementation Constraints** — runtime constraints
8. **Source Capabilities Summary** — provider trait checklist
9. **Dependencies** — external packages with pinned versions (from lock file)
10. **Risks / Open Questions** — unknowns, `[unknown]` items

Config keys MUST be captured verbatim from source — never renamed.

---

## Phase 5: Iterative Validation Loop

Compare every artifact against the source code until convergence. See [validation-procedure.md](references/validation-procedure.md) for detailed checks.

### Six Validation Dimensions

| Dimension | What It Checks |
|-----------|----------------|
| **V1: Type Fidelity** | Every field, type, serialization attr (struct-level AND field-level: renames, aliases, unconditional skips, conditional skips), wire name against source |
| **V2: Handler Logic** | Every validation, URL, header, error path, request body construction (field values, format strings, conditional nulls, wrapper structures), shared response type deduplication against source |
| **V3: API Contract** | Every endpoint path, method, auth against route definitions |
| **V4: Cross-Reference** | Types ↔ handlers, specs ↔ design, ID stability |
| **V5: Completeness** | No unspecified behaviors, no phantom requirements |
| **V6: Dependency Versions** | Every dependency pinned from lock file, import paths valid for pinned version |

### Severity Classification

| Severity | Convergence | Examples |
|----------|-------------|---------|
| CRITICAL | Blocks | Wrong field type, missing required field, missing field-level rename (keyword collision), wrong request body structure (flat vs wrapped), missing format string for generated IDs |
| HIGH | Blocks | Wrong URL path, missing validation, wrong error code, missing deserialization alias, missing unconditional serialization skip, undocumented vendor-specific field names |
| MEDIUM | Non-blocking | Missing conditional serialization attribute, missing optional field, undocumented response type deduplication |
| LOW | Non-blocking | Missing Display string values, incomplete scenario, undocumented default on nested request fields |

### Loop Logic

```
pass = 0
loop:
  pass += 1
  discrepancies = run V1 through V6 against source
  
  if no CRITICAL or HIGH discrepancies: break (CONVERGED)
  
  report all discrepancies with severity
  fix resolvable items; ask user about ambiguous items
  
  if pass > 5: HALT and escalate remaining issues
```

### Anti-Pattern: Shallow Validation

Every check must compare a **specific value** in the artifact against a **specific value** in the source. "design.md has a Domain Model section" is insufficient. "Domain Model → Token.is_active is `bool` in source, `String` in spec → CRITICAL" is correct.

---

## Phase 6: User Sign-Off

Present a convergence report:

```
## Reverse-Spec Validation Report

### Source: $SOURCE_PATH
### Capability: $CRATE_NAME

### Inventory
- Types: N defined, N verified
- Handlers: M defined, M verified  

### Validation Passes
- Pass 1: X discrepancies (Y fixed, Z clarified)
- Pass 2: 0 discrepancies — CONVERGED

### Artifacts Generated
- specs/$CRATE/spec.md (N requirements, M scenarios)
- design.md
```

Inform the user that specs and design are complete and the define skill will continue with tasks.

---

## Guardrails

### NEVER

- Assume a field type — verify against source
- Rename config keys — capture verbatim
- Invent wire names — extract from serialization attributes/decorators/annotations
- Skip fields — document every field (use `[unknown]` if unclear)
- Skip field-level attributes — keyword-collision renames, aliases, and unconditional skips are wire-format-critical
- Hand-write types from memory — copy from source
- Assume two handlers share construction details because they target the same API — verify each independently
- Proceed past a checkpoint without user confirmation
- Generate test fixtures without verifying against source response shapes
- Use breadth-first analysis — always depth-first by domain
- Record dependency names without versions — always capture exact versions from manifest AND lock file
- Assume "latest" version compatibility — API surfaces change between versions
- Merge cross-struct column headers — use separate columns for each struct type
- State patterns as universal rules — always check for exceptions (e.g., "all collection fields have default-when-absent" is rarely true for all)
- Skip the guest/entry-point layer — middleware, error mapping, body injection, and parameter sourcing are load-bearing behaviors
- Say one function "behaves like" another — verify each function's code paths independently

### ALWAYS

- Present structural inventory before deep analysis
- Batch all clarifying questions together
- Compare every type/class/interface field against source definition, including field-level renames, aliases, and serialization skips
- Run validation passes until convergence or pass limit
- Report discrepancy severity levels
- Include source traceability for every requirement
- Use `[unknown]` rather than guessing
- Capture dependency versions from both manifest and lock file; use manifest specifiers in design.md
- Check serialization wire names by applying naming convention rules — flag divergent naming
- Document each utility function's behavior independently, including error messages and status code handling
- Include guest/entry-point behaviors in the inventory (CORS, error mapping, body injection, owner parameter sourcing)
- Document response type serialization ownership — which module contains the canonical impl, which modules reuse it
- Document every outbound API call body completely, including vendor-specific field names for audit/secondary calls
- For orchestration handlers, document exact format strings, conditional null fields, and wrapper structures independently

### When Uncertain

Ask the user or mark `[unknown]`. Never guess.

## References

- [Validation procedure details](references/validation-procedure.md)
- [Lessons learned from previous reverse-spec attempts](references/lessons-learned.md)
