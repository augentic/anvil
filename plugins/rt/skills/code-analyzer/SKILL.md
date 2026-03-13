---
name: code-analyzer
description: Analyze source code to produce Specify artifacts (specs + design.md) capturing business logic for code migration.
argument-hint: [source-path] [change-dir]
allowed-tools: Read, Write, StrReplace, Shell, Grep
---

# Code Analyzer Skill

## Overview

Analyze a source codebase to produce reconstruction-grade, **language-agnostic** Specify artifacts (specs + design.md) capturing domain-level business logic. The artifacts split behavioral requirements (specs) from technical details (design), enabling cleaner separation of "what" from "how", in a format suitable for migration to any target language or runtime.

**Key principle**: The artifacts are an intermediary format with **no bias toward any target language**. They describe what the code does, not how it should be implemented in a specific language.

## Derived Arguments

1. **Source Path** (`$SOURCE_PATH`): Path to the source codebase
2. **Change Directory** (`$CHANGE_DIR`): Specify change directory (e.g., `./.specify/changes/component/`)

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

See complete definitions in [Specify Artifact Format Specification - Tags Reference](references/specify.md#tags-reference).

## Process

### Step 1: Identify Component Structure

**THINK**: Before analyzing code, reason through these questions:

1. What source language is this? (Check file extensions: .ts, .js, .go, .py, .rs, .java, .cs)
2. What is the entry point? (Look for: main.\*, index.\*, handler exports, main functions)
3. How is the code organized? (Monolithic file? Multiple modules? Layered architecture?)
4. What external libraries are used? (Check manifest: package.json, go.mod, requirements.txt, Cargo.toml)
5. What async patterns are present? (async/await, Promises, goroutines, callbacks, futures)
6. What types are defined? (interfaces, classes, structs, enums)

**ANALYZE**: Read the source at `$SOURCE_PATH` and identify:

1. **Source language**: Detect from file extensions
2. Entry points (e.g., `main.*`, `index.*`, handler exports, `func main()`, `if __name__ == "__main__"`, etc.)
3. Module organization and file structure
4. External dependencies from manifest files (`package.json`, `go.mod`, `requirements.txt`, `Cargo.toml`, `pom.xml`, etc.)
5. Async boundaries (async/await, Promises, goroutines, threads, futures, etc.)
6. Type definitions (interfaces, types, classes, structs, enums)

**VERIFY**: Check your understanding:

- [ ] I've identified the primary source language correctly
- [ ] I've found all entry points (there may be multiple)
- [ ] I've understood the module structure (not just listed files)
- [ ] I've checked the manifest file for dependencies
- [ ] I've noted async vs sync execution patterns

### Step 2: Extract Business Logic

**SEMANTIC DISCOVERY** (Optional but Recommended):

If semantic search tool available (grepai, CocoIndex), use it to discover business logic patterns:

```bash
# Discover business logic hotspots
semantic-search "business logic and validation rules" $SOURCE_PATH

# Find error handling patterns
semantic-search "error handling and edge cases" $SOURCE_PATH

# Locate external dependencies
semantic-search "HTTP API calls and external services" $SOURCE_PATH
```

Use semantic results to:

- Prioritize which files/functions to analyze deeply
- Inform tag classification ([domain] vs [infrastructure] vs [mechanical])
- Identify hidden business logic in utility functions
- Reduce [unknown] tags by 15-25%

See [Semantic Search Reference](references/semantic-search.md) for detailed guidance.

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

**ANALYZE**: For each function/method, document:

- Symbol name and return type
- **Execution mode** (synchronous, asynchronous, parallel)
- Algorithm (pseudocode with tags and control flow)
- **Conditional branches** (if/else, switch, ternary)
- **Error handling** (try/catch, error propagation, recovery)
- **State mutations** (what data/state is modified)
- Preconditions and postconditions
- Edge cases and failure modes
- Complexity/cost notes
- **Constants and configuration** (hardcoded values, env vars)
  - **Config keys verbatim**: Environment variable names and config keys must be captured exactly as written in the source code. If the code reads `process.env.CC_STATIC_URL`, the artifacts must document `CC_STATIC_URL`, not a paraphrased `GTFS_STATIC_URL`. Do not rename config keys for clarity.
  - **Active subsets**: When a lookup table is filtered at runtime by a config value, document only the active entries in the primary constant. Note the full table's existence and entry count separately. See [Context Gaps #11](references/context-gaps.md#11-active-subset-vs-full-dataset).

  **Active subset identification process**:
  1. **Identify the full table**: Count total entries, note any "unmapped" or sentinel values
  2. **Identify the runtime filter**: What config/constant limits the active entries? Default value if config is absent?
  3. **Document BOTH in design.md Constants section**:

     ```markdown
     - `ACTIVE_STATIONS` — source: env var `STATIONS`, default: `"0,19,40"`; semantics: Station IDs to process
     - `STATION_ID_TO_STOP_CODE_MAP` — source: hardcoded; value: { 0: "133", 19: "9218", 40: "134" };
       semantics: Maps active station IDs to GTFS stop codes (full table has 47 entries but only
       stations from `ACTIVE_STATIONS` are processed)
     ```

  **Why**: Without this, downstream code generators produce code that processes all entries instead of the filtered subset.

- **Input types** (full shape with nested structure)
  - **Field optionality**: For each field in an input type, determine whether it may be absent, null, or empty at runtime. Add an `Optional?` column to the type definition table with values `yes`, `no`, or `unknown`.

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

- **Output types** (full shape with nested structure)
  - **Full schema from shared types**: When the source code constructs output objects using a type imported from an external or shared library (e.g., `new SmarTrakEvent()`, a shared DTO class), trace the **full** type definition in that library. Document ALL fields of the output type, not just the fields populated by this component. For each field, note whether this component populates it or whether it is present in the schema for other producers. This allows code generators to produce the complete output type rather than a stripped-down subset.
  - Example: If `SmarTrakEvent` has 8 fields but this component only sets 5, document all 8 fields and annotate the 3 unused ones with "not populated by this component".
- **Serialization mappings** (when input/output types are deserialized from or serialized to a wire format):
  - **CRITICAL**: For EVERY field in input/output types, check for serialization decorators/annotations:
    - TypeScript: `@JsonProperty`, `@JsonConverter`, `@Serializable`
    - Go: struct tags like `json:"fieldName"` or `xml:"elementName"`
    - Python: `@dataclass`, `field(metadata=...)`
    - Java: `@JsonProperty`, `@XmlElement`
    - C#: `[JsonProperty]`, `[XmlElement]`
  - Document the wire-format field name for each property (trace through decorators/annotations)
  - Document custom converters and their EXACT behavior:
    - What is the input format? (e.g., string `"true"/"false"`, number as string)
    - What is the output type? (e.g., `boolean`, `number`)
    - Is it bidirectional or one-way?
    - Example: `BooleanConverter: deserializes string "true"/"false" → boolean`
  - Document XML root element names and array-wrapping configuration
  - Add to design.md type tables with a `Wire Name` column and `Converter` column:

    ```markdown
    | Field        | Type      | Wire Name   | Converter                       | Optional? |
    | ------------ | --------- | ----------- | ------------------------------- | --------- |
    | `hasArrived` | `boolean` | `haEntrado` | string "true"/"false" → boolean | no        |
    ```

  - If a wire-format name cannot be determined, use `unknown — wire name not visible in source`
  - See [Language Mapping Guide - Serialization Decorators](references/language-mapping.md#serialization-decorators-and-field-name-mappings) for per-language examples

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

### Step 3: Document External API Surfaces

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

**Critical**: Trace actual deserialization, not type declarations. If code does `const allocated: string[] = await response.json()`, the response shape is `string[]`, not some broader interface type.

**ANALYZE**: For each external HTTP/API call:

For each external HTTP/API call:

- Endpoint URL pattern (EXACT path and query parameters as constructed in source)
- HTTP method
- Request headers (list each, including how values are obtained -- from config, hardcoded, etc.)
- Request body shape (exact JSON/XML structure)
- Response body shape (CRITICAL: capture full nesting)
- Authentication method (including where the identity/token name comes from -- config variable or hardcoded)
- Error responses (status codes and body shapes)
- **Retry behavior** (if present)
- **Timeout** (if specified)

**Trace actual deserialization, not type declarations.** When the source code parses an API response (e.g., `response.json()`, `JSON.parse()`), trace what the result is assigned to and how its fields are accessed. The runtime response shape is determined by how the code uses the response, not by interface declarations that may be broader. If the code does `const allocated: string[] = await response.json()`, the response shape is `string[]`, not the full interface type. Always follow the data from the HTTP response through parsing to usage to determine the true shape.

**Response shape documentation**: Include a concrete JSON example showing the actual response structure. This prevents downstream code generators from fabricating wrapper types.

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

### Step 4: Capture External Service Dependencies

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

### Step 5: Capture Publication & Timing Patterns

Document exactly:

- **Publication count**: The exact number of times each event is published (e.g., "2 times", NOT "twice with delays" which is ambiguous). Count by reading the loop bounds in the source code (e.g., `for _ in 0..2` means 2 publications).
- **Delay placement**: Whether the delay occurs BEFORE or AFTER each publication round. Document the exact loop structure: "sleep 5s then publish all events, repeated 2 times" is different from "publish, then sleep 5s, then publish again".
- **Payload identity**: Whether the published payload is IDENTICAL across rounds or modified between rounds (e.g., timestamps incremented). Most patterns publish identical payloads -- document explicitly if the source modifies the payload between iterations.
- Timing/delay operations with exact durations
- Retry patterns with counts and backoff
- Batch vs individual publication
- **Concurrent operations** (parallel vs sequential)
- **Message metadata**: For each published message, document all metadata beyond the payload:
  - Partition/routing key (e.g., `message.key = externalId`)
  - Custom headers (e.g., `message.headers["key"] = value`)
  - Topic construction pattern (e.g., `${env}-${TOPIC_CONSTANT}` vs full topic from config)

**Publication pattern example**:

```markdown
- **Publication pattern**: Publish all events 2 times with 5-second delay before each round
- **Loop structure**: `for round in 0..2 { sleep(5s); for each event { publish(event) } }`
- **Payload modification**: None -- identical event published each round
- **Purpose**: Signal departure from station for schedule adherence
```

### Step 6: Capture Metrics and Observability Patterns

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

### Step 7: Write Specify Artifacts

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

**GENERATE**: Write Specify artifacts to `$CHANGE_DIR` using the format specified in [specify.md](references/specify.md). The artifact format follows the `augentic` schema from [augentic/lifecycle](https://github.com/augentic/lifecycle).

#### 7a: Create Directory Structure

Create `$CHANGE_DIR/` and `$SPECS_DIR/` directories.

#### 7b: Write design.md

Write `$DESIGN_PATH` with the following sections (see [specify.md](references/specify.md) Design Document Format for the full template):

1. **Context** — source component path, target runtime, purpose, source files analyzed
2. **Domain Model** — full nested type definitions with wire-format annotations; entities with attributes, relationships, and business rules
3. **Structures** — source code structure inventory (imports, exports, classes, functions, external dependencies)
4. **API Contracts** — inbound endpoints with request/response schemas; outbound API calls with complete request/response shapes traced from actual deserialization
5. **External Services** — each service with type (database, managed table store, message broker, cache, identity provider, API, WebSocket), technology, operations, connection details, authentication
6. **Constants & Configuration** — every constant with source (hardcoded/env var), literal value, semantics, required flag, default
7. **Business Logic** — tagged pseudocode algorithm for every handler/function. **Every controller endpoint** that delegates to a service method must have a corresponding block, including simple list endpoints — otherwise downstream code generators have no algorithm to implement. See [Context Gaps §14](references/context-gaps.md#14-simple-list-endpoints-missing-business-logic-blocks). Include: execution mode, input/output types, error handling, state mutations, preconditions, postconditions, edge cases, errors raised, unknowns
8. **Publication & Timing Patterns** — topic/queue names, construction patterns, message counts, timing, payload structures, partition keys, custom headers
9. **Output Event Structures** — full nested output type schemas
10. **Implementation Constraints** — factual `[runtime]` constraints describing source behavior (do NOT prescribe target-specific solutions). Examples:
    - `[runtime]` Source uses in-memory cache with startup/background loading
    - `[runtime]` Source uses `setTimeout`/`setInterval` for periodic cache refresh
    - `[runtime]` Source uses circuit breaker library for outbound HTTP
    - `[runtime]` Source caches OAuth tokens in process memory
    When API response parity matters, fill **Serialization & API Fidelity** (optional fields, DateTime format, field naming, concurrency)
11. **Source Capabilities Summary** — derive from External Services; checklist of generic capability categories (Configuration, Outbound HTTP, Message publishing, Key-value state, Authentication/Identity, Table/database access, Real-time messaging)
12. **Dependencies** — external packages with purpose
13. **Notes** — additional observations, source-specific constructs, performance/security considerations

**IMPORTANT — Managed data store classification:**

When the source code uses `@azure/data-tables`, `TableClient`, `listEntities`, `createEntity`, `updateEntity`, `deleteEntity`, or calls Azure Table Storage REST endpoints (`*.table.core.windows.net`):
- The External Services section **MUST** classify these as type: `managed table store`, NOT as type: `API`.
- The Source Capabilities Summary **MUST** check `Table/database access`.
- Cloud-managed table/document stores (Azure Table Storage, Cosmos DB, DynamoDB) are data stores, not external HTTP APIs, regardless of their access protocol.
- When the source loads data from a managed table store and caches it in memory, the Source Capabilities Summary should include **both** `Table/database access` and `Key-value state`.

#### 7c: Write Spec File

Write a single consolidated spec file at `$SPECS_DIR/$CRATE_NAME/spec.md` with a `## Handler: <handler_name>` section for each handler/capability identified in Steps 2-6. Each handler section contains:

1. **Purpose** — 1-2 sentence description from the function's role
2. **Requirements** — one requirement per distinct business rule (use `The system SHALL ...` format). Each requirement includes:
   - Source traceability (source function path)
   - **Scenarios** derived from algorithm steps (happy path), error handling (error paths), and edge cases. Use BDD Given/When/Then format.
3. **Error Conditions** — error type, description, HTTP status, and conditions that trigger each error
4. **Metrics** — metric name, type (counter/gauge/histogram), emission point, and labels

See [specify.md](references/specify.md) Spec File Format and Deriving Specs from Source Code for the complete template.

**VERIFY**: After writing, validate against the checklist:

- [ ] One spec file per crate at `$SPECS_DIR/$CRATE_NAME/spec.md` with `## Handler:` sections
- [ ] Each spec has Purpose, Requirements with BDD scenarios, and Error Conditions
- [ ] design.md has all required sections (Context through Notes)
- [ ] design.md Business Logic has tagged algorithm for every handler
- [ ] Every business logic statement has a tag: [domain], [infrastructure], [mechanical], or [unknown]
- [ ] No inference or guessing — unknowns are marked explicitly
- [ ] Language-agnostic — no target language concepts introduced
- [ ] Traceability — every statement traceable to source code
- [ ] Complete type shapes — no abbreviated or simplified structures with wire-format annotations
- [ ] Config keys verbatim — exactly as written in source

Note: Steps are numbered 1-7. Ensure all steps are completed before writing the artifacts.

### Output

Write completed Specify artifacts to `$CHANGE_DIR`. The artifacts are a language-agnostic intermediate format that can be used for code generation in any target language.

## Reference Documentation

Detailed guidance and specifications are available in `references/`:

- **[Specify Artifact Format Specification](references/specify.md)** - Complete artifact structure with spec and design.md templates
- **[Language Mapping Guide](references/language-mapping.md)** - How to map common language constructs to artifact format (with examples from TypeScript, Go, Python, etc.)
- **[Context Gaps Reference](references/context-gaps.md)** - Commonly missed details and how to capture them, including data access phrasing (§13) and ensuring every endpoint has a business logic block (§14)
- **[Examples](references/examples/)** - Complete analysis examples for different scenarios

## Examples

Detailed examples are available in the `references/examples/` directory:

1. [outbound-http.md](references/examples/outbound-http.md) - Analyze a TypeScript HTTP handler and produce Specify artifacts
2. [branching-caching.md](references/examples/branching-caching.md) - Capture complex conditional logic with hierarchical numbering
3. [parallel-execution.md](references/examples/parallel-execution.md) - Document async/parallel execution patterns in artifacts

## Error Handling

### Common Issues and Resolutions

| Issue                           | Cause                                             | Resolution                                                             |
| ------------------------------- | ------------------------------------------------- | ---------------------------------------------------------------------- |
| TypeScript source doesn't parse | Invalid TypeScript or missing dependencies        | Run `tsc --noEmit` to verify source compiles first                     |
| Too many [unknown] tags in artifacts | Dynamic typing, metaprogramming, or unclear logic | Review source for type annotations; add comments for clarity      |
| Artifacts missing business logic | Functions not exported or in inaccessible modules | Check module imports; ensure key functions are exported               |
| Artifacts missing API endpoints | Routes defined dynamically or in middleware       | Check framework-specific routing patterns (Express, Nest, etc.)        |
| Config keys not captured        | Environment variables accessed indirectly         | Search for `process.env` patterns across all source files              |
| Type shapes incomplete          | Complex generic types or union types              | Document full type definition; use `unknown` for unresolvable generics |

### Recovery Process

1. Review the generated artifacts against the source code
2. For missing sections: identify the source construct that should have been captured
3. Re-analyze the specific source file or function
4. For persistent [unknown] tags: add source code comments to clarify intent
5. Re-run the full analysis

## Verification Checklist

Before completing, verify all items from the [Specify Artifact Validation Checklist](references/specify.md#validation-checklists) are satisfied.

Additionally, verify these skill-specific items:

**Artifact completeness**:

- [ ] **One spec per crate**: Single consolidated spec file at `$SPECS_DIR/$CRATE_NAME/spec.md` with `## Handler:` sections for each handler/capability
- [ ] **design.md complete**: design.md includes all required sections (Context, Domain Model, Structures, API Contracts, External Services, Constants & Configuration, Business Logic, Publication & Timing, Output Event Structures, Implementation Constraints, Source Capabilities Summary, Dependencies, Notes)
- [ ] **BDD scenarios**: Each spec has Requirements with Given/When/Then scenarios derived from algorithm steps and error handling

**Analysis fidelity**:

- [ ] **Config keys verbatim**: Environment variable names captured exactly as written in source (not renamed for clarity). If source reads `CC_STATIC_URL`, artifacts must say `CC_STATIC_URL` -- not `GTFS_CC_STATIC_URL` or `GTFS_STATIC_URL`.
- [ ] **API response shapes**: Each external API response includes the actual deserialized type (e.g., `string[]` vs `{ data: { all: [...] } }`), traced from actual deserialization code, not inferred from type declarations. Include a concrete JSON example.
- [ ] **API URL fidelity**: API URL paths and query parameters match the source code exactly. Do not add or remove query parameters.
- [ ] **Authentication source**: For each authenticated API call, document whether the identity name is hardcoded or comes from a config variable (e.g., `AZURE_IDENTITY`).
- [ ] **Publication pattern precision**: Publication count, delay placement (before/after), and payload identity (identical or modified) documented from actual loop structure in source code.
- [ ] **Metrics**: Metric names, types, emission points, and labels documented in the relevant spec file's Metrics section
- [ ] **Message metadata**: Partition keys, headers, and topic construction patterns captured
- [ ] **Wire-format field names**: All deserialized/serialized types include wire-format field name annotations where the source uses renaming decorators/annotations/config
- [ ] **Custom converters**: Conversion logic for custom deserializers/serializers is documented (e.g., what `BooleanConverter` does)
- [ ] **Active subsets**: Lookup tables that are filtered at runtime note which entries are active (see [Context Gaps #11](references/context-gaps.md#11-active-subset-vs-full-dataset))
- [ ] **Field optionality**: Input type fields include an `Optional?` column indicating whether the field may be absent/null at runtime (yes/no/unknown)
- [ ] **Output type completeness**: Output types document ALL fields from the type definition (including fields not populated by this component), with notes on which fields this component populates vs. which are present in the shared schema
- [ ] **Output field types**: Output type fields document exact types (e.g., `integer` not `float` for integer fields, exact enum types not raw strings). When the source code uses a specific numeric type (e.g., `speed: integer`), do not generalize it (e.g., `speed: float`).
- [ ] **External service classification**: All external services categorized by type (database, managed table store, cache, message broker, identity provider, API, WebSocket). Managed data stores (Azure Table Storage, Cosmos DB, DynamoDB) classified as `managed table store`, not `API`.
- [ ] **Source capabilities summary**: Source Capabilities Summary checklist present in design.md, derived from External Services. `Table/database access` checked whenever source uses ORM, SQL, or managed table stores.

## Important Notes

- **Language-agnostic**: Do not introduce target language concepts (e.g., Rust traits, Python decorators). Describe behavior only.
- **Preserve structure**: Maintain exact field names, nesting, and type shapes from source
- **No inference**: Use `unknown` tokens rather than guessing behavior or values
- **Traceability**: Every statement must be traceable to source code or comments
- **Reconstruction-grade**: The artifacts must contain sufficient detail for accurate code generation in any language
