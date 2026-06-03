# Writing artifacts (Step 7)

Step 7 writes the spec and design.md artifacts to `$SLICE_DIR` using the format specified in [augentic-specify-usage.md](../references/spec-runtime/specialist-usage.md). The artifact format follows the `augentic` schema from [augentic/lifecycle](https://github.com/augentic/lifecycle).

## THINK: synthesize before writing

Before writing the artifacts, synthesize your findings:

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
11. Have I documented guest/entry-point layer behaviors?
12. Have I captured dependency versions from the lock file?

### Check for common omissions

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
- [ ] Keyword-collision renames and deserialization aliases
- [ ] Unconditional vs conditional serialization skips
- [ ] Guest/entry-point behaviors (middleware, error mapping, body injection)
- [ ] Response type deduplication table
- [ ] Dependency versions from lock file

## 7a — Create directory structure

Create `$SLICE_DIR/` and `$SPECS_DIR/` directories.

## 7b — Write design.md

Write `$DESIGN_PATH` with the following sections (see [augentic-specify-usage.md](../references/spec-runtime/specialist-usage.md) Design Document Format for the full template):

1. **Context** — source component path, target runtime, purpose, source files analyzed
2. **Domain Model** — full nested type definitions with wire-format annotations; entities with attributes, relationships, and business rules. Include separate tables for field-level renames, aliases, unconditional skips, and conditional skips. Include deduplication table for shared response types.
3. **Structures** — source code structure inventory (imports, exports, classes, functions, external dependencies)
4. **API Contracts** — inbound endpoints with request/response schemas; outbound API calls with complete request/response shapes traced from actual deserialization
5. **External Services** — each service with type (database, managed table store, message broker, cache, identity provider, API, WebSocket), technology, operations, connection details, authentication
6. **Constants & Configuration** — every constant with source (hardcoded/env var), literal value, semantics, required flag, default
7. **Business Logic** — tagged pseudocode algorithm for every handler/function. **Every controller endpoint** that delegates to a service method must have a corresponding block, including simple list endpoints — otherwise downstream code generators have no algorithm to implement. See [Context Gaps §14](context-gaps.md#14-simple-list-endpoints-missing-business-logic-blocks). Include: execution mode, input/output types, error handling, state mutations, preconditions, postconditions, edge cases, errors raised, unknowns
8. **Publication & Timing Patterns** — topic/queue names, construction patterns, message counts, timing, payload structures, partition keys, custom headers
9. **Output Event Structures** — full nested output type schemas
10. **Implementation Constraints** — factual `[runtime]` constraints describing source behavior (do NOT prescribe target-specific solutions). Examples:
    - `[runtime]` Source uses in-memory cache with startup/background loading
    - `[runtime]` Source uses `setTimeout`/`setInterval` for periodic cache refresh
    - `[runtime]` Source uses circuit breaker library for outbound HTTP
    - `[runtime]` Source caches OAuth tokens in process memory When API response parity matters, fill **Serialization & API Fidelity** (optional fields, DateTime format, field naming, concurrency)
11. **Source Capabilities Summary** — derive from External Services; checklist of generic adapter categories (Configuration, Outbound HTTP, Message publishing, Key-value state, Authentication/Identity, Table/database access, Real-time messaging, Blob storage, Document storage)
12. **Dependencies** — external packages with manifest version specifier (for generated project dependency declaration) and lock file resolved version (for API compatibility reference). Include feature flags / optional features enabled.
13. **Risks / Open Questions** — unknowns, `[unknown]` items, missing lock file, ambiguous source patterns
14. **Notes** — additional observations, source-specific constructs, performance/security considerations

### IMPORTANT — Managed data store classification

When the source code uses `@azure/data-tables`, `TableClient`, `listEntities`, `createEntity`, `updateEntity`, `deleteEntity`, or calls Azure Table Storage REST endpoints (`*.table.core.windows.net`):

- The External Services section **MUST** classify these as type: `managed table store`, NOT as type: `API`.
- The Source Capabilities Summary **MUST** check `Table/database access`.
- Cloud-managed table/document stores (Azure Table Storage, Cosmos DB, DynamoDB) are data stores, not external HTTP APIs, regardless of their access protocol.
- When the source uses blob storage APIs (`BlobServiceClient`, `ContainerClient`, `S3Client`, `putObject`, `getObject`), classify as type: `blob store` and check `Blob storage` in the Source Capabilities Summary.
- When the source uses document database APIs (`MongoClient`, `CosmosClient` document API, `find`, `insertOne`), classify as type: `document store` and check `Document storage` in the Source Capabilities Summary.
- When the source loads data from a managed table store and caches it in memory, the Source Capabilities Summary should include **both** `Table/database access` and `Key-value state`.

## 7c — Write spec file

Write a single consolidated spec file at `$SPECS_DIR/$CRATE_NAME/spec.md` using the flat baseline format:

1. `## Purpose` — 1-2 sentence description of what the crate/adapter does overall
2. `### Requirement: <Behavior Name>` — one top-level block per distinct business rule (use `The system SHALL ...` format). Add `ID: REQ-XXX` immediately after the heading, numbering requirements sequentially in file order. Each requirement includes:
   - Source traceability (source function path)
   - `#### Scenario: <name>` entries derived from algorithm steps (happy path), error handling (error paths), and edge cases
3. `## Error Conditions` — shared error type, description, HTTP status, and trigger conditions when the source exposes them
4. `## Metrics` — metric name, type (counter/gauge/histogram), emission point, and labels when explicit in the source

See [augentic-specify-usage.md](../references/spec-runtime/specialist-usage.md) Spec File Format and Deriving Specs from Source Code for the complete template.

Note: Steps are numbered 1–7. Ensure all steps are completed before writing the artifacts.

### Output

Write completed Specify artifacts to `$SLICE_DIR`. The artifacts are a language-agnostic intermediate format that can be used for code generation in any target language.
