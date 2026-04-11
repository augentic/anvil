---
name: analyze
description: Analyze source code to produce reconstruction-grade, language-agnostic Specify artifacts (specs + design.md) with iterative validation until convergence.
license: MIT
argument-hint: "[source-path] [change-dir]"
---

# Analyze

Analyze a source codebase to produce reconstruction-grade, **language-agnostic** Specify artifacts (specs + design.md) capturing domain-level business logic. The artifacts split behavioral requirements (specs) from technical details (design), enabling cleaner separation of "what" from "how", in a format suitable for migration to any target language or runtime.

Unlike single-pass analyzers, this skill runs user-confirmed checkpoints and a validation loop that compares every generated artifact against the source code until zero critical discrepancies remain.

**Cardinal Rule**: NEVER assume, infer, or hallucinate. If anything is unclear, ASK the user. Use `[unknown]` tokens for anything the code cannot answer. The cost of asking is low. The cost of a wrong assumption is an incorrect specification that propagates through all generated artifacts.

**Key principle**: The artifacts are an intermediary format with **no bias toward any target language**. They describe what the code does, not how it should be implemented in a specific language.

## Derived Arguments

1. **Source Path** (`$SOURCE_PATH`): Path to the source codebase (the source of truth)
2. **Change Directory** (`$CHANGE_DIR`): Specify change directory (e.g., `.specify/changes/my-api/`)

```text
$SOURCE_PATH = $ARGUMENTS[0]
$CHANGE_DIR  = $ARGUMENTS[1]
$SPECS_DIR   = $CHANGE_DIR/specs
$DESIGN_PATH = $CHANGE_DIR/design.md
```

## Principles (Non-Negotiable)

1. **Focus**: Extract only domain/business logic and its inputs/outputs. Exclude infrastructure unless part of a domain rule.
2. **Descriptive, not interpretive**: Produce algorithmic descriptions of what the code does. Do not infer "why" unless present in source.
3. **Zero inference**: Do not invent behavior or semantics. Use explicit `unknown` tokens.
4. **Explicit constants**: List every constant by identifier and semantic availability.
5. **Traceability**: Each statement must be traceable to code. Do not attribute intent not in comments.
6. **Tagging**: Each Business Logic line must include one tag: `[domain]`, `[infrastructure]`, `[mechanical]`, or `[unknown]`.
7. **Conservatism**: Prefer `unknown` over guessing.
8. **Language-agnostic**: Do not introduce target-language concepts. Describe behavior, not implementation.

## Tags and Unknown Tokens

See complete definitions in the [Specify Artifact Format Specification - Tags Reference](../../references/specify.md#tags-reference).

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

**THINK**: Before analyzing code, reason through these questions:

1. What source language is this? (Check file extensions: .ts, .js, .go, .py, .rs, .java, .cs)
2. What is the entry point? (Look for: main.\*, index.\*, handler exports, main functions)
3. How is the code organized? (Monolithic file? Multiple modules? Layered architecture?)
4. What external libraries are used? (Check manifest: package.json, go.mod, requirements.txt, Cargo.toml, pom.xml)
5. What async patterns are present? (async/await, Promises, goroutines, callbacks, futures)
6. What types are defined? (interfaces, classes, structs, enums)

**ANALYZE**: Read the source at `$SOURCE_PATH` and extract exact names, types, and locations for:

| Category | Extract |
|----------|---------|
| **Source language** | Detect from file extensions |
| **Entry points** | `main.*`, `index.*`, handler exports, `func main()`, `if __name__ == "__main__"`, etc. |
| **Module organization** | File structure, layered architecture, monolithic vs modular |
| **Types** | Every struct/enum/class/interface with every field name, type, optionality, serialization attributes, wire names, AND all generated trait/interface implementations (e.g., Rust derives, C# attributes, TypeScript decorators) |
| **Handlers** | Every route with HTTP method, path, input type, output type |
| **External calls** | Every outbound HTTP call with URL pattern, method, headers, auth |
| **Config variables** | Every env var / config key, captured verbatim |
| **Validation rules** | Every input validation check with the exact condition |
| **Error types** | Every error variant with status code and trigger |
| **Shared utilities** | Helper functions used across handlers — document each function's behavior independently (never say "behaves like X" without verifying every code path) |
| **Dependencies** | External package dependencies with **exact versions** from manifest AND lock file (resolved versions) |
| **Async boundaries** | async/await, Promises, goroutines, threads, futures |
| **Guest/entry-point** | Middleware (CORS, auth), error code → HTTP status mapping, body injection/transformation, parameter sourcing (`owner` values), and any validation performed before the domain handler |

**VERIFY**: Check your understanding:

- [ ] I've identified the primary source language correctly
- [ ] I've found all entry points (there may be multiple)
- [ ] I've understood the module structure (not just listed files)
- [ ] I've checked the manifest file for dependencies
- [ ] I've noted async vs sync execution patterns

**SEMANTIC DISCOVERY** (Optional but Recommended):

If semantic search tool available (grepai, CocoIndex), use it to discover business logic patterns:

```bash
semantic-search "business logic and validation rules" $SOURCE_PATH
semantic-search "error handling and edge cases" $SOURCE_PATH
semantic-search "HTTP API calls and external services" $SOURCE_PATH
```

Use semantic results to prioritize which files/functions to analyze deeply, inform tag classification, identify hidden business logic in utility functions, and reduce `[unknown]` tags. See [Semantic Search Reference](references/semantic-search.md) for detailed guidance.

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

#### 3.1 Read ALL Types

Capture every field, serialization attribute, wire name, optionality. Copy definitions verbatim from source; do NOT paraphrase or write from memory.

**THINK**: Before extracting each type, reason through:

1. What fields does it have? (Full nested structure)
2. What serialization decorators/annotations are present at struct AND field level?
3. Are there keyword collisions requiring field-level renames?
4. Are there deserialization aliases accepting multiple wire names?
5. Are there unconditional serialization skips vs conditional skips?
6. What trait/interface implementations are generated? (Not just serialization — equality, cloning, etc.)

#### Type Extraction Rules

Type mismatches were the single largest source of errors in previous analysis work. See [lessons-learned.md](references/lessons-learned.md) for details.

- Copy type definitions verbatim — never hand-write from memory
- Capture exact types (e.g., `i32` vs `i64`, `int` vs `long`, `number` vs `string`)
- Capture ALL generated trait/interface implementations and annotations — not just serialization ones. Missing equality implementations (Rust `PartialEq`/`Eq`, C# `IEquatable`, Python `__eq__`) cause build failures when code uses `==` comparison
- Capture exact serialization attributes per stack — at BOTH struct/class level AND field level:
  - Rust: `serde` attrs (`rename`, `rename_all`, `default`, `skip_serializing_if`, `skip_serializing`, `deserialize_with`, `alias`)
  - C#: `JsonPropertyName`, `JsonIgnore`, `JsonConverter`
  - TypeScript: class-transformer/class-validator decorators, `@JsonProperty`, `@JsonConverter`
  - Python: Pydantic `Field(alias=...)`, `model_validator`, `@dataclass`
  - Go: struct tags like `json:"fieldName"` or `xml:"elementName"`
  - Java: `@JsonProperty`, `@XmlElement`
- For field-level renames: check for keyword collisions (`type`, `class`, `import`, `return`, etc.) where the implementation language uses a different identifier but maps to the original name via rename attribute. These are CRITICAL for wire compatibility
- For deserialization aliases: check for fields that accept multiple wire names (e.g., both `maskedPan` and `maskedPAN`). Missing aliases cause deserialization failures with real upstream data
- For unconditional serialization skips: distinguish between conditional skip (`skip_serializing_if = "is_none"`) and unconditional skip (`skip_serializing` / `JsonIgnore`). An unconditional skip strips the field entirely from output — omitting this changes the response shape
- For collection/array fields: explicitly note which have default-when-absent behavior and which do NOT — do not assume a universal pattern
- For types with custom deserialization: note that they should NOT also use generated/derived deserialization to avoid conflicting implementations
- For empty/marker types (no fields): note the type shape explicitly
- For enums: variant names AND serialization representation (string, integer, etc.)
- For nested types: follow every level of nesting
- For custom deserializers/converters: document exact behavior (input format, output type, bidirectional or one-way)
- Check wire names by applying the project's naming convention rules (e.g., `camelCase`, `snake_case`, `PascalCase`). Flag cases where field names diverge from the convention producing unexpected wire names
- When multiple types share field names: document each type's field type SEPARATELY in the cross-type table — never merge columns for types with different field sets

See [Language Mapping Guide - Serialization Decorators](references/language-mapping.md#serialization-decorators-and-field-name-mappings) for per-language examples.

#### Field Optionality Detection

For each field in an input type, determine whether it may be absent, null, or empty at runtime. Add an `Optional?` column to the type definition table with values `yes`, `no`, or `unknown`.

**Field optionality detection rules**:
1. A field is `Optional? = yes` if the source code:
   - Checks for null/undefined: `if (field != null)`
   - Uses optional chaining: `obj?.field`
   - Uses nullish coalescing: `field ?? defaultValue`
   - Uses fallback patterns: `fieldA || fieldB || defaultValue`
   - Has TypeScript type annotation with `?`: `field?: string`

2. A field is `Optional? = no` if:
   - Accessed unconditionally without checks
   - Marked as required in type annotations

3. Use `Optional? = unknown` if:
   - Field is accessed but pattern is unclear
   - Third-party library type without clear documentation

**When fallback patterns are used** (e.g., `trainUpdate.evenTrainId || trainUpdate.oddTrainId`):
- Mark BOTH fields as `Optional? = yes`
- Document the fallback logic in Algorithm section

#### 3.2 Read EACH Handler

**THINK**: Before extracting logic, reason through each function:

1. What is the function's purpose? (What business operation does it perform?)
2. Is it synchronous or asynchronous? (Look for async keyword, Promises, callbacks)
3. What are the inputs and their shapes? (Full nested structure, not just top-level)
4. What are the outputs? (Complete schema, trace through return statements)
5. What validations are performed? (Required fields, format checks, business rules)
6. What external calls are made? (HTTP, database, cache, pub/sub)
7. What can go wrong? (Error handling, edge cases, failure modes)
8. Are there any hardcoded values or config keys? (Environment variables, constants)
9. How does data flow through the function? (Transformations, mutations)
10. Are there conditional branches? (if/else, switch, ternary operators)

**Tag Classification Reasoning**:

- Is this core business logic that defines "what the business does"? → `[domain]`
- Is this technical plumbing to communicate with external systems? → `[infrastructure]`
- Is this simple data transformation without business meaning? → `[mechanical]`
- Am I uncertain about the behavior or purpose? → `[unknown]`

**ANALYZE**: For each handler, document:

- Symbol name and return type
- **Execution mode** (synchronous, asynchronous, parallel)
- Input type and deserialization method
- Validation logic (checks, order, error messages, guard conditions like `is_none()` vs `is_none_or(String::is_empty)`)
- URL construction (base URL config key, path segments, query params, URL encoding)
- Headers (static and dynamic, custom headers)
- Auth method (API key, Bearer token, identity source)
- Response parsing (deserialization, field mapping, error handling)
- Business logic (transformations, conditional branches, loop error granularity)
- Algorithm (pseudocode with tags and control flow)
- **Error handling** (try/catch, error propagation, recovery)
- **State mutations** (what data/state is modified)
- Preconditions and postconditions
- Edge cases and failure modes
- **Constants and configuration** (hardcoded values, env vars)
  - **Config keys verbatim**: Environment variable names and config keys must be captured exactly as written in the source code. If the code reads `process.env.CC_STATIC_URL`, the artifacts must document `CC_STATIC_URL`, not a paraphrased `GTFS_STATIC_URL`. Do not rename config keys for clarity.
  - **Active subsets**: When a lookup table is filtered at runtime by a config value, document only the active entries in the primary constant. Note the full table's existence and entry count separately. See [Context Gaps #11](references/context-gaps.md#11-active-subset-vs-full-dataset).
- **Input types** (full shape with nested structure, with Optional? annotations)
- **Output types** (full shape with nested structure)
  - **Full schema from shared types**: When the source code constructs output objects using a type imported from an external or shared library (e.g., `new SmarTrakEvent()`, a shared DTO class), trace the **full** type definition in that library. Document ALL fields of the output type, not just the fields populated by this component. For each field, note whether this component populates it or whether it is present in the schema for other producers.
- **Serialization mappings** (when input/output types are deserialized from or serialized to a wire format):
  - **CRITICAL**: For EVERY field in input/output types, check for serialization decorators/annotations
  - Document the wire-format field name for each property (trace through decorators/annotations)
  - Document custom converters and their EXACT behavior
  - Add to design.md type tables with a `Wire Name` column and `Converter` column
  - If a wire-format name cannot be determined, use `unknown — wire name not visible in source`
- Errors raised and propagation flow
- Unknowns

**VERIFY**: For each function documented, check:

- [ ] I've captured the complete input schema (all nested fields with Optional? annotations)
- [ ] I've captured the complete output schema (traced through shared types if needed)
- [ ] I've tagged every business logic statement with [domain], [infrastructure], [mechanical], or [unknown]
- [ ] I've documented config keys EXACTLY as written in source (not renamed)
- [ ] I've identified active subsets for filtered lookup tables
- [ ] I've captured wire-format field names and custom converters
- [ ] I've documented all conditional branches and edge cases
- [ ] I've noted execution mode (sync/async) and any concurrent operations
- [ ] When uncertain, I've used [unknown] rather than guessing

#### 3.3 Read Shared Utilities

Read shared utilities used by this domain. Document each function's behavior independently, including error messages and status code handling.

#### 3.4 Cross-Reference

Verify every type field referenced in handler logic exists in the type definition, and vice versa.

#### 3.5 Orchestration Handlers

When multiple handlers target the same upstream API, document each handler's request body construction INDEPENDENTLY:
- Exact format strings for generated IDs (e.g., `"prefix-{id}-suffix"`)
- Wire format differences (flat vs wrapped structures targeting the same endpoint)
- Body fields set to null/default — document explicitly even when identical to another handler
- Conditional field values (e.g., `adjustment_amount = None` for full operations vs `Some(value)` for partial)

#### 3.6 Secondary/Audit API Calls

All outbound calls (including best-effort, non-critical, audit writes) must document exact request bodies with vendor-specific field names. "Best-effort" does not mean "under-specified."

#### 3.7 Response Type Ownership

Track which module/file defines the canonical serialization implementation for each response type. When multiple handlers share a response type, only one should contain the impl. Document this in a deduplication table.

### Document External API Surfaces

**THINK**: Before documenting each API call, reason through:

1. What is the complete URL? (Is it hardcoded, from config, or dynamically constructed?)
2. What HTTP method? (GET, POST, PUT, PATCH, DELETE)
3. What headers are sent? (Authorization, Content-Type, custom headers)
4. What is the request body? (Full JSON/XML structure, not just described)
5. What does the response look like? (Trace through actual deserialization code, not type declarations)
6. How is the response parsed? (response.json()? XML parser? Text?)
7. What fields are actually accessed from the response? (This reveals the true shape)
8. What happens on errors? (Status codes, error response format, retry behavior)
9. Are there timeouts? (Explicit timeout values)
10. Is authentication required? (API keys, tokens, basic auth)

**Critical**: Trace actual deserialization, not type declarations. If code does `const allocated: string[] = await response.json()`, the response shape is `string[]`, not some broader interface type. Always follow the data from the HTTP response through parsing to usage to determine the true shape.

**ANALYZE**: For each external HTTP/API call:

- Endpoint URL pattern (EXACT path and query parameters as constructed in source)
- HTTP method
- Request headers (list each, including how values are obtained — from config, hardcoded, etc.)
- Request body shape (exact JSON/XML structure)
- Response body shape (CRITICAL: capture full nesting)
- Authentication method (including where the identity/token name comes from — config variable or hardcoded)
- Error responses (status codes and body shapes)
- **Retry behavior** (if present)
- **Timeout** (if specified)

**Response shape documentation**: Include a concrete JSON example showing the actual response structure.

```markdown
- **Response shape**: `string[]` (flat JSON array)
- **Example response**: `["NZ 1234", "NZ 5678"]`
- **Usage**: Each string is a vehicle label; spaces are stripped before use as partition key
```

**Authentication source**: When documenting how a token or identity is obtained, capture whether the identity name is hardcoded or comes from configuration:

```markdown
- **Auth**: Bearer token from identity provider
  - Identity name: from config `AZURE_IDENTITY` (NOT hardcoded)
  - Token acquisition: access token requested using identity name
```

### Capture External Service Dependencies

For each external service or system dependency, document thoroughly:

- Service name and type — use one of: `database`, `managed table store`, `message broker`, `cache`, `identity provider`, `API`, `WebSocket`
- Technology (e.g., PostgreSQL, Azure Table Storage, Redis, Kafka, Azure AD)
- Connection details visible in source
- Operations performed (read, write, publish, subscribe, query, token acquisition)
- Data formats (if different from internal types)
- Authentication method

**Service type classification**:

- **database**: SQL databases accessed via ORM, raw SQL, or repository patterns (PostgreSQL, MySQL, SQL Server, etc.)
- **managed table store**: Cloud-managed NoSQL/table storage services accessed via SDK or REST API (Azure Table Storage via `@azure/data-tables`/`TableClient`, Azure Cosmos DB, DynamoDB, etc.). Do NOT classify these as `API` — they are managed data stores, not external HTTP APIs.
- **cache**: Key-value stores used for caching or ephemeral state (Redis, Memcached, in-memory cache libraries)
- **message broker**: Message queues and event streaming (Kafka, RabbitMQ, Azure Service Bus, SQS)
- **identity provider**: Authentication/token services (Azure AD, OAuth providers, Auth0)
- **API**: External HTTP/REST/GraphQL APIs
- **WebSocket**: WebSocket connections for real-time messaging

### Capture Publication & Timing Patterns

Document exactly:

- **Publication count**: The exact number of times each event is published (e.g., "2 times", NOT "twice with delays" which is ambiguous). Count by reading the loop bounds in the source code (e.g., `for _ in 0..2` means 2 publications).
- **Delay placement**: Whether the delay occurs BEFORE or AFTER each publication round. Document the exact loop structure.
- **Payload identity**: Whether the published payload is IDENTICAL across rounds or modified between rounds (e.g., timestamps incremented). Most patterns publish identical payloads — document explicitly if the source modifies the payload between iterations.
- Timing/delay operations with exact durations
- Retry patterns with counts and backoff
- Batch vs individual publication
- **Concurrent operations** (parallel vs sequential)
- **Message metadata**: For each published message, document all metadata beyond the payload:
  - Partition/routing key (e.g., `message.key = externalId`)
  - Custom headers (e.g., `message.headers["key"] = value`)
  - Topic construction pattern (e.g., `${env}-${TOPIC_CONSTANT}` vs full topic from config)

### Capture Metrics and Observability Patterns

For each metric emission in the source code (counters, gauges, histograms, log-structured events):

- Metric name and type (counter, gauge, histogram)
- When it is emitted (which step in the algorithm)
- Dimensions/labels attached
- Purpose (operational visibility, alerting, debugging)

Example artifacts:

```markdown
- **Metrics**:
  - `events_published` — type: monotonic counter; emitted: after each successful publish; labels: none
  - `irrelevant_station` — type: monotonic counter; emitted: when station is filtered out; labels: station ID
  - `r9k_delay` — type: gauge; emitted: during validation; labels: none; value: message delay in seconds
```

---

## Phase 4: Write Artifacts

**THINK**: Before writing the artifacts, synthesize your findings:

1. Have I captured ALL entry points and handlers?
2. Have I documented ALL external API calls with complete request/response shapes?
3. Have I traced ALL config keys and constants exactly as written?
4. Have I identified ALL business logic and tagged it appropriately?
5. Have I captured ALL type definitions with complete nested structures?
6. Have I noted ALL optional fields, wire-format names, and custom converters?
7. Have I documented ALL error handling patterns?
8. Have I captured ALL metrics, message metadata, and timing patterns?
9. Are there any `[unknown]` tags that I should investigate further?
10. Do the artifacts provide sufficient detail for reconstruction-grade code generation?

**Check for common omissions**:

- [ ] Config keys captured verbatim (not renamed for clarity)
- [ ] Active subsets identified for filtered lookup tables
- [ ] Wire-format field names for all serialized types
- [ ] Custom converter behavior documented
- [ ] Field optionality marked for all input types
- [ ] Complete output schemas (including fields not populated by this component)
- [ ] Message partition keys and custom headers
- [ ] Metrics with emission points and labels
- [ ] Retry patterns and timeout values
- [ ] Concurrent vs sequential operation patterns

Write specs and design.md to `$CHANGE_DIR`.

### 4a: Write Spec File

Write `$SPECS_DIR/$CRATE_NAME/spec.md` using flat baseline format:

1. `## Purpose` — 1-2 sentence description of what the crate/capability does overall
2. `### Requirement: <Behavior Name>` — one top-level block per distinct business rule (use `The system SHALL ...` format). Add `ID: REQ-XXX` immediately after the heading, numbering requirements sequentially in file order. Each requirement includes:
   - Source traceability (source function path)
   - `#### Scenario: <name>` entries derived from algorithm steps (happy path), error handling (error paths), and edge cases
3. `## Error Conditions` — shared error type, description, HTTP status, and trigger conditions when the source exposes them
4. `## Metrics` — metric name, type (counter/gauge/histogram), emission point, and labels when explicit in the source

Rules:
- One requirement per distinct behavior
- Use `SHALL`/`MUST` for normative language
- Sequential IDs: `REQ-001`, `REQ-002`, ...
- Source traceability for every requirement

### 4b: Write design.md

Write `$DESIGN_PATH` with the following sections (see [specify.md](../../references/specify.md) Design Document Format for the full template):

1. **Context** — source component path, purpose, source files analyzed, source type: `source-code`
2. **Domain Model** — full nested type definitions with wire-format annotations; type tables with columns: Field, Type, Wire Name, Optional?, Serde Attributes; separate tables for field-level renames, aliases, unconditional skips, and conditional skips; deduplication table for shared response types
3. **Structures** — source code structure inventory (imports, exports, classes, functions, external dependencies)
4. **API Contracts** — inbound endpoints with request/response schemas; outbound API calls with complete request/response shapes traced from actual deserialization
5. **External Services** — each service with type (database, managed table store, message broker, cache, identity provider, API, WebSocket), technology, operations, connection details, authentication
6. **Constants & Configuration** — every constant with source (hardcoded/env var), literal value, semantics, required flag, default. Config keys MUST be captured verbatim from source — never renamed.
7. **Business Logic** — tagged pseudocode algorithm for every handler/function. **Every controller endpoint** that delegates to a service method must have a corresponding block, including simple list endpoints — otherwise downstream code generators have no algorithm to implement. See [Context Gaps §14](references/context-gaps.md#14-simple-list-endpoints-missing-business-logic-blocks). Include: execution mode, input/output types, error handling, state mutations, preconditions, postconditions, edge cases, errors raised, unknowns
8. **Publication & Timing Patterns** — topic/queue names, construction patterns, message counts, timing, payload structures, partition keys, custom headers
9. **Output Event Structures** — full nested output type schemas
10. **Implementation Constraints** — factual `[runtime]` constraints describing source behavior (do NOT prescribe target-specific solutions). When API response parity matters, fill **Serialization & API Fidelity** (optional fields, DateTime format, field naming, concurrency)
11. **Source Capabilities Summary** — derive from External Services; checklist of generic capability categories (Configuration, Outbound HTTP, Message publishing, Key-value state, Authentication/Identity, Table/database access, Real-time messaging, Blob storage, Document storage)
12. **Dependencies** — external packages with pinned versions (manifest specifier as primary, lock file version for API compatibility reference)
13. **Risks / Open Questions** — unknowns, `[unknown]` items
14. **Notes** — additional observations, source-specific constructs, performance/security considerations

**IMPORTANT — Managed data store classification:**

When the source code uses `@azure/data-tables`, `TableClient`, `listEntities`, `createEntity`, `updateEntity`, `deleteEntity`, or calls Azure Table Storage REST endpoints (`*.table.core.windows.net`):

- The External Services section **MUST** classify these as type: `managed table store`, NOT as type: `API`.
- The Source Capabilities Summary **MUST** check `Table/database access`.
- Cloud-managed table/document stores (Azure Table Storage, Cosmos DB, DynamoDB) are data stores, not external HTTP APIs, regardless of their access protocol.
- When the source uses blob storage APIs (`BlobServiceClient`, `ContainerClient`, `S3Client`, `putObject`, `getObject`), classify as type: `blob store` and check `Blob storage` in the Source Capabilities Summary.
- When the source uses document database APIs (`MongoClient`, `CosmosClient` document API, `find`, `insertOne`), classify as type: `document store` and check `Document storage` in the Source Capabilities Summary.
- When the source loads data from a managed table store and caches it in memory, the Source Capabilities Summary should include **both** `Table/database access` and `Key-value state`.

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

### Additional Verification Items

Beyond V1-V6, also verify:

- [ ] **Config keys verbatim**: Environment variable names captured exactly as written in source
- [ ] **API response shapes**: Each external API response includes the actual deserialized type, traced from actual deserialization code. Include a concrete JSON example.
- [ ] **API URL fidelity**: API URL paths and query parameters match the source code exactly
- [ ] **Authentication source**: For each authenticated API call, document whether the identity name is hardcoded or comes from a config variable
- [ ] **Publication pattern precision**: Publication count, delay placement, and payload identity documented from actual loop structure
- [ ] **Metrics**: Metric names, types, emission points, and labels documented
- [ ] **Message metadata**: Partition keys, headers, and topic construction patterns captured
- [ ] **Active subsets**: Lookup tables that are filtered at runtime note which entries are active
- [ ] **Field optionality**: Input type fields include an `Optional?` column
- [ ] **Output type completeness**: Output types document ALL fields from the type definition
- [ ] **Output field types**: Exact types documented (not generalized)
- [ ] **External service classification**: Managed data stores classified as `managed table store`, not `API`
- [ ] **Source capabilities summary**: Checklist present, derived from External Services
- [ ] **Language-agnostic**: No target language concepts introduced
- [ ] **Every endpoint has a Business Logic block**: Even when logic is trivial

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
## Analysis Validation Report

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
- Introduce target-language concepts — describe behavior, not implementation

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
- Tag every business logic line with `[domain]`, `[infrastructure]`, `[mechanical]`, or `[unknown]`
- Ensure every controller endpoint has a Business Logic block (even trivial ones)
- Maintain exact field names, nesting, and type shapes from source

### When Uncertain

Ask the user or mark `[unknown]`. Never guess.

## Error Handling

### Common Issues and Resolutions

- **Source doesn't parse**: Cause: invalid source or missing dependencies. Resolution: verify the source compiles first.
- **Too many [unknown] tags in artifacts**: Cause: dynamic typing, metaprogramming, or unclear logic. Resolution: review the source for type annotations and add comments for clarity.
- **Artifacts missing business logic**: Cause: functions not exported or in inaccessible modules. Resolution: check module imports and ensure key functions are exported.
- **Artifacts missing API endpoints**: Cause: routes defined dynamically or in middleware. Resolution: check framework-specific routing patterns.
- **Config keys not captured**: Cause: environment variables accessed indirectly. Resolution: search for env access patterns across all source files.
- **Type shapes incomplete**: Cause: complex generic types or union types. Resolution: document the full type definition and use `unknown` for unresolvable generics.

### Recovery Process

1. Review the generated artifacts against the source code
2. For missing sections: identify the source construct that should have been captured
3. Re-analyze the specific source file or function
4. For persistent [unknown] tags: add source code comments to clarify intent
5. Re-run the full analysis

## Reference Documentation

Detailed guidance and specifications are available in `references/`:

- **[Specify Artifact Format Specification](../../references/specify.md)** - Complete artifact structure with spec and design.md templates
- **[Language Mapping Guide](references/language-mapping.md)** - How to map common language constructs to artifact format (with examples from TypeScript, Go, Python, etc.)
- **[Context Gaps Reference](references/context-gaps.md)** - Commonly missed details and how to capture them, including data access phrasing (§13) and ensuring every endpoint has a business logic block (§14)
- **[Semantic Search Reference](references/semantic-search.md)** - Using semantic search to improve analysis coverage
- **[Validation Procedure](references/validation-procedure.md)** - Detailed V1-V6 validation dimension checks
- **[Lessons Learned](references/lessons-learned.md)** - Anti-patterns from previous analysis attempts
- **[Examples](references/examples/)** - Complete analysis examples for different scenarios
