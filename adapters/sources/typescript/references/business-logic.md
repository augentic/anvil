# Business-logic extraction (Step 2)

Step 2 extracts behaviour from the source's domain code. When `$INCLUDE` / `$EXCLUDE` / `$MANIFEST` are set, restrict the read set for this step and every subsequent step accordingly. Files outside the scoped read set are not analyzed here — business-logic extraction, external-surface documentation, and type capture operate only on the filtered tree. Sentinel files ([scope-filters.md](scope-filters.md) §"Sentinels always read") remain available to Step 1 but do not feed Step 2's business-logic pass unless they also match the scope filter.

## Semantic discovery (optional but recommended)

If a semantic search tool is available (grepai, CocoIndex), use it to discover business logic patterns:

```bash
semantic-search "business logic and validation rules" $SOURCE_PATH
semantic-search "error handling and edge cases" $SOURCE_PATH
semantic-search "HTTP API calls and external services" $SOURCE_PATH
```

Use semantic results to:

- Prioritize which files/functions to analyze deeply
- Inform tag classification (`[domain]` vs `[infrastructure]` vs `[mechanical]`)
- Identify hidden business logic in utility functions
- Reduce `[unknown]` tags by 15-25%

See [Semantic Search Reference](semantic-search.md) for detailed guidance.

## Depth-first organization

When the source has clear functional domain boundaries (e.g., auth, catalog, payments), analyze **depth-first by domain** rather than breadth-first by artifact type. For each domain, fully analyze types → handlers → utilities → cross-references before moving to the next domain. This catches cross-cutting details like shared validation patterns, common header construction, and utility function behavior that breadth-first scanning misses.

Fall back to step-by-step (all types, then all handlers, etc.) for simpler or single-domain components.

## Per-function THINK / ANALYZE / VERIFY

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

### Tag classification reasoning

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
  - **Active subsets**: When a lookup table is filtered at runtime by a config value, document only the active entries in the primary constant. Note the full table's existence and entry count separately. See [Context Gaps #11](context-gaps.md#11-active-subset-vs-full-dataset).

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
    - Rust: `serde` attrs (`rename`, `rename_all`, `default`, `skip_serializing_if`, `skip_serializing`, `deserialize_with`, `alias`)
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
  - See [Language Mapping Guide - Serialization Decorators](language-mapping.md#serialization-decorators-and-field-name-mappings) for per-language examples

- Errors raised and propagation flow
- Unknowns

## Type extraction rules

Type mismatches are the single largest source of errors in extraction. Observe these rules strictly:

- **Copy type definitions verbatim from source** — never hand-write types from memory
- Capture **exact types** (e.g., `i32` vs `i64`, `int` vs `long`, `number` vs `string`)
- Capture **ALL generated trait/interface implementations and annotations** — not just serialization ones. Missing equality implementations (Rust `PartialEq`/`Eq`, C# `IEquatable`, Python `__eq__`) cause build failures when code uses `==` comparison
- Capture **exact serialization attributes per stack** — at BOTH struct/class level AND field level:
  - Rust: `serde` attrs (`rename`, `rename_all`, `default`, `skip_serializing_if`, `skip_serializing`, `deserialize_with`, `alias`)
  - C#: `JsonPropertyName`, `JsonIgnore`, `JsonConverter`
  - TypeScript: class-transformer/class-validator decorators
  - Python: Pydantic `Field(alias=...)`, `model_validator`
- **Keyword-collision renames**: Check for fields where the implementation language uses a different identifier but maps to the original name via rename attribute (e.g., `balance_type` renamed to `"type"` because `type` is a reserved keyword). These are CRITICAL for wire compatibility
- **Deserialization aliases**: Check for fields that accept multiple wire names (e.g., both `maskedPan` and `maskedPAN`). Missing aliases cause deserialization failures with real upstream data
- **Unconditional vs conditional serialization skips**: Distinguish between conditional skip (`skip_serializing_if = "is_none"`) and unconditional skip (`skip_serializing` / `JsonIgnore`). An unconditional skip strips the field entirely from output — omitting this changes the response shape
- **Collection/array fields**: Explicitly note which have default-when-absent behavior and which do NOT — do not assume a universal pattern
- **Custom deserialization**: For types with custom deserialization, note that they should NOT also use generated/derived deserialization to avoid conflicting implementations
- **Empty/marker types** (no fields): Note the type shape explicitly
- **Enums**: Variant names AND serialization representation (string, integer, etc.)
- **Nested types**: Follow every level of nesting
- **Custom deserializers/converters**: Document exact behavior
- **Wire name verification**: Check wire names by applying the project's naming convention rules (e.g., `camelCase`, `snake_case`, `PascalCase`). Flag cases where field names diverge from the convention
- **Cross-struct tables**: When multiple types share field names, document each type's field type SEPARATELY — never merge columns for types with different field sets

## Orchestration and shared handler rules

- **Independent handler documentation**: When multiple handlers target the same upstream API, document each handler's request body construction INDEPENDENTLY:
  - Exact format strings for generated IDs (e.g., `"prefix-{id}-suffix"`)
  - Wire format differences (flat vs wrapped structures targeting the same endpoint)
  - Body fields set to null/default — document explicitly even when identical to another handler
  - Conditional field values (e.g., `adjustment_amount = None` for full operations vs `Some(value)` for partial)
- **Secondary/audit API calls**: All outbound calls (including best-effort, non-critical, audit writes) must document exact request bodies with vendor-specific field names. "Best-effort" does not mean "under-specified."
- **Response type ownership**: Track which module/file defines the canonical serialization implementation for each response type. When multiple handlers share a response type, only one should contain the impl. Document this in a deduplication table.
- **Cross-reference check**: After documenting all handlers in a domain, verify every type field referenced in handler logic exists in the type definition, and vice versa.

## Per-function VERIFY checklist

- [ ] I've captured the complete input schema (all nested fields with Optional? annotations)
- [ ] I've captured the complete output schema (traced through shared types if needed)
- [ ] I've tagged every business logic statement with [domain], [infrastructure], [mechanical], or [unknown]
- [ ] I've documented config keys EXACTLY as written in source (not renamed)
- [ ] I've identified active subsets for filtered lookup tables
- [ ] I've captured wire-format field names and custom converters
- [ ] I've documented all conditional branches and edge cases
- [ ] I've noted execution mode (sync/async) and any concurrent operations
- [ ] I've copied type definitions verbatim (not from memory)
- [ ] I've captured ALL serialization attributes at both struct and field level
- [ ] I've checked for keyword-collision renames and deserialization aliases
- [ ] I've distinguished unconditional from conditional serialization skips
- [ ] When uncertain, I've used [unknown] rather than guessing
