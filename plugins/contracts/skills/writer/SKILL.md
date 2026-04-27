---
name: writer
description: "Validate spec alignment with baseline API contracts and produce the minimal contract delta for uncovered interactions — JSON Schema, OpenAPI 3.1, AsyncAPI 3.0."
allowed-tools: Read, Write, StrReplace, Shell, Grep, Glob
---

# Contracts Writer

Validate spec alignment with baseline API contracts and produce the minimal contract delta for uncovered interactions. The writer produces JSON Schema payload definitions, OpenAPI 3.1 HTTP bindings, and AsyncAPI 3.0 messaging bindings — only for interactions the specs require that the baseline does not already cover.

The algorithm is the same regardless of whether the baseline is empty, rich, or externally imported. The three authorship patterns (contract-first, spec-first, contract-given) differ in baseline state and plan structure, not in code path:

- **Contract-first** — baseline contracts exist from a preceding contract change. Most spec interactions are already covered; the writer validates alignment and produces a small or empty delta.
- **Spec-first** — baseline is empty (single-repo, no external consumers). The delta is the full contract set, derived from the change's specs.
- **Contract-given** — baseline was imported from an external system. The writer validates alignment and produces delta only for extensions.

## Authority Hierarchy

When conflicts arise, follow this strict precedence:

1. **This SKILL.md** (highest) — generation rules and hard constraints
2. **Specify artifacts** (specs) — behavioral specification
3. **references/** — JSON Schema, OpenAPI, AsyncAPI conventions
4. **Baseline contracts** (`.specify/contracts/`) — existing platform vocabulary
5. **LLM inference** (lowest) — prohibited for unknowns; use `[unknown]` markers

If specs conflict with baseline contracts, flag the mismatch for human review rather than silently resolving. If a reference document and this skill disagree, this skill wins. Never guess at payload shapes, endpoint paths, or channel names — mark uncertain elements with `[unknown]` and document them in the alignment report.

## Hard Rules

Violations of any rule below fail generation. There are no exceptions.

1. **Valid JSON Schema** — every generated schema file must be valid JSON Schema (draft 2020-12).
2. **Valid OpenAPI 3.1** — every generated HTTP binding must be valid OpenAPI 3.1.
3. **Valid AsyncAPI 3.0** — every generated messaging binding must be valid AsyncAPI 3.0.
4. **`$ref` resolution** — all `$ref` pointers must resolve to existing files: either in the change's `contracts/schemas/` or in the baseline `.specify/contracts/schemas/`.
5. **`$id` stability** — once a `$id` is assigned to a schema, it must not change. New schemas get new `$id` values; updated schemas keep their existing `$id`.
6. **One type per schema file** — each JSON Schema file defines exactly one top-level type.
7. **Shared schemas via `$ref`** — OpenAPI and AsyncAPI bindings reference `../schemas/` via `$ref`. Never inline duplicate type definitions in binding files.
8. **Kebab-case naming** — all file names use kebab-case with `.yaml` extensions.
9. **Baseline preservation** — never modify baseline files in `.specify/contracts/`. All changes go in the change-level `contracts/` directory.
10. **Minimal delta** — generate only what the specs require that the baseline does not already cover. Do not regenerate covered interactions.

## Arguments

```text
$CHANGE_DIR     = .specify/changes/<change-name>
$SPECS_DIR      = $CHANGE_DIR/specs
$CONTRACTS_DIR  = $CHANGE_DIR/contracts
$BASELINE_DIR   = .specify/contracts
```

## Required References

Before generating contract artifacts, read these documents:

1. [json-schema-conventions.md](../../references/json-schema-conventions.md) — `$id` URI format, `title`/`description` rules, `$ref` conventions, type-mapping guidance
2. [openapi-conventions.md](../../references/openapi-conventions.md) — OpenAPI 3.1 structure, `$ref → ../schemas/`, path/method/response conventions
3. [asyncapi-conventions.md](../../references/asyncapi-conventions.md) — AsyncAPI 3.0 structure, channel/operation conventions
4. [artifact-structure.md](../../references/artifact-structure.md) — `.specify/contracts/` directory layout, naming conventions, change-level delta rules

---

## The 6-Step Algorithm

### Step 1: Read the Baseline Contracts

Read `$BASELINE_DIR` and build an inventory of the existing platform vocabulary.

**When the directory exists and contains files**, parse:

| Source | What to extract |
|---|---|
| `schemas/*.yaml` | `$id`, `title`, `description`, `properties`, `required`, `$ref` targets |
| `http/*.yaml` | OpenAPI `paths` (method + path), request body schemas, response schemas, error responses |
| `messages/*.yaml` | AsyncAPI `channels`, `operations`, message payload schemas |

For each item, record:
- **Type**: schema, http-binding, or message-binding
- **File**: path relative to `.specify/contracts/`
- **Identity**: `$id` for schemas; path+method for HTTP; channel+operation for messaging
- **Shape**: property names and types for schemas; `$ref` targets for bindings

**When `$BASELINE_DIR` does not exist or is empty**, the baseline is empty. This is the spec-first fallback pattern — all spec interactions become delta. Record an empty inventory and proceed to Step 2.

### Step 2: Read the Spec Files

Read all spec files under `$SPECS_DIR/`. Identify requirements that describe interactions:

**HTTP interactions** — look for:
- Endpoint paths and methods (`POST /users`, `GET /orders/{id}`)
- Request payload shapes (field names, types, required/optional)
- Response payload shapes (field names, types)
- Status codes and error conditions (`201 Created`, `409 Conflict`, `404 Not Found`)
- `WHEN`/`THEN` clauses that reference HTTP request/response patterns

**Messaging interactions** — look for:
- Channel or topic names (`user.registered`, `order.placed`)
- Message payload shapes (field names, types)
- Pub/sub patterns (who publishes, who subscribes)
- Event-driven triggers (`WHEN an order is placed, THEN publish...`)

**Data types** — look for:
- Entity names referenced in scenarios (`UserRegistration`, `Order`, `ErrorResponse`)
- Field structures from `WHEN`/`THEN` clauses (`...with a payload including id, email, and created_at`)
- Shared vocabulary types used across multiple scenarios

Build a structured list of **spec interactions** — each entry captures:
- **Interaction type**: HTTP endpoint, message channel, or data type
- **Identity**: path+method, channel+operation, or type name
- **Shape**: fields, types, constraints as described in the spec
- **Source**: spec file path and requirement ID for traceability

**When the change has no specs** (a contract-only import change), skip to Step 3's normalisation path. The delta consists of metadata normalisation only.

### Step 3: Validate Alignment and Determine the Minimal Delta

Compare each spec interaction from Step 2 against the baseline inventory from Step 1. Classify every interaction into one of three categories:

#### Already Covered (primary path in contract-first workflow)

When the baseline already defines an endpoint, channel, or schema that the spec describes:

1. **Match the interaction.** Find the baseline item by identity (path+method for HTTP, channel+operation for messaging, type name for schemas).

2. **Verify alignment.** Check that the baseline's shape matches the spec's requirements:
   - **HTTP bindings**: endpoint path matches, HTTP method matches, request body schema includes the fields the spec references, response schema includes the fields the spec asserts, error status codes from the spec are present in the binding's responses.
   - **Message bindings**: channel name matches, operation direction matches, message payload schema includes the fields the spec references.
   - **Schemas**: property names and types from the spec are present in the baseline schema, required fields match, nested `$ref` types exist.

3. **Flag mismatches.** When alignment verification finds discrepancies:
   - Record each mismatch with: baseline file path, spec requirement ID, description of the discrepancy.
   - Mismatches are **warnings for human review**, not automatic corrections. The writer does not overwrite baseline contracts to match specs — the precedence is ambiguous (specs and baseline may both be partially correct), so a human must decide.
   - Example mismatches: a response schema missing a field that a spec scenario asserts; a spec referencing a status code the binding does not define; a message payload missing a field the spec's `THEN` clause expects.

4. **Do not regenerate.** Covered interactions produce no output files. The alignment result (pass or warning) is recorded in the alignment report.

#### New or Modified (primary path in spec-first workflow)

When the specs require types, endpoints, or channels absent from the baseline:

1. **Data types.** A spec references a type name (e.g. `OAuthToken`) that has no corresponding `$id` in the baseline schemas. → Add to the schema delta.

2. **HTTP endpoints.** A spec describes an endpoint path+method (e.g. `POST /users/verify`) that has no corresponding path entry in any baseline HTTP binding. → Add to the HTTP delta.

3. **Message channels.** A spec describes a channel or topic (e.g. `user.verified`) that has no corresponding channel in any baseline message binding. → Add to the messaging delta.

4. **Extensions to existing bindings.** A spec describes a new path on an API domain that already has a baseline binding file (e.g. adding `POST /users/verify` to the `user-api` domain which already has `POST /users` and `GET /users/{id}`). → The delta produces an updated binding file that includes only the new paths. The existing paths are not duplicated.

Each delta item records: interaction type, identity, shape (derived from spec), source requirement ID.

#### Normalisation

When baseline files lack Specify conventions:

- **Missing `$id` on schemas** — propose a normalisation delta that adds `$id` in the standard URN format without changing the schema's interface shape.
- **Missing `title` or `description`** — propose additions derived from the schema's filename and structure.
- **Inconsistent naming** — flag for human review but do not rename files (renaming would break existing `$ref` pointers).

Normalisation changes go into the change-level `contracts/` directory like any other delta. They are replacements of the baseline file with the metadata added — the interface shape is identical.

### Step 4: Generate JSON Schema Files

For each data type in the delta from Step 3, generate a JSON Schema file under `$CONTRACTS_DIR/schemas/`.

Each file must contain:

```yaml
$schema: "https://json-schema.org/draft/2020-12/schema"
$id: "urn:specify:schemas/<filename-without-extension>"
title: "<TypeName>"
description: "<behavioral description from the spec>"
type: object
properties:
  # ... derived from spec scenario data
required:
  # ... fields the spec treats as always-present
```

Rules:

- **`$id` format**: `urn:specify:schemas/<filename-without-extension>`. The filename is the kebab-case version of the type name. Example: type `UserRegistration` → file `user-registration.yaml` → `$id: "urn:specify:schemas/user-registration"`.
- **`title`**: the PascalCase type name as it appears in the spec.
- **`description`**: a concise sentence describing the type's role, drawn from the spec's behavioral description.
- **`properties`**: derive from the spec's scenario data. Map spec descriptions to JSON Schema types:

  | Spec description | JSON Schema type |
  |---|---|
  | Text, name, email, string | `type: string` |
  | Number, count, amount | `type: number` or `type: integer` |
  | True/false, flag | `type: boolean` |
  | Date, timestamp, created_at | `type: string`, `format: date-time` |
  | UUID, identifier | `type: string`, `format: uuid` |
  | List, array, collection | `type: array`, `items: ...` |
  | Nested object | `$ref` to another schema file |

- **`required`**: include fields the spec treats as always-present in responses or mandatory in requests. When the spec does not clarify, mark the field as required and note the assumption in the alignment report.
- **`$ref` for shared sub-types**: when a property references another domain type that already exists in the baseline or in this delta, use `$ref` rather than inlining. Baseline references use the path relative to the baseline: `$ref: "urn:specify:schemas/<name>"`. Delta references within the same change use relative file paths.
- **One type per file**: never define multiple top-level types in a single schema file. Inline `properties` definitions for simple nested structures (e.g. an `address` sub-object that is not shared) are acceptable; shared sub-types must be separate files.

Reference: [json-schema-conventions.md](../../references/json-schema-conventions.md)

### Step 5: Generate or Update OpenAPI Binding

When Step 3's delta includes uncovered HTTP interactions, generate OpenAPI 3.1 files under `$CONTRACTS_DIR/http/`.

**File organisation**: group related endpoints into a single OpenAPI file by API domain. Use the same file name as the baseline when extending an existing API domain (e.g. `user-api.yaml`). For new API domains, name the file after the domain: `<domain>-api.yaml`.

Each file must contain:

```yaml
openapi: "3.1.0"
info:
  title: "<API Domain> API"
  version: "0.1.0"
  description: "<brief description>"
paths:
  /endpoint/path:
    post:  # or get, put, delete, patch
      operationId: <kebab-case-operation-name>
      summary: "<from spec>"
      requestBody:
        required: true
        content:
          application/json:
            schema:
              $ref: "../schemas/<type>.yaml"
      responses:
        "201":
          description: "<from spec>"
          content:
            application/json:
              schema:
                $ref: "../schemas/<type>.yaml"
        "409":
          description: "<error condition from spec>"
          content:
            application/json:
              schema:
                $ref: "../schemas/error-response.yaml"
```

Rules:

- **`$ref` to schemas**: all request body and response schemas reference `../schemas/<type>.yaml`. Never inline schema definitions in the OpenAPI file.
- **Error responses**: derive from the spec's error conditions. Each error status code in the spec becomes a response entry. Use a shared `error-response.yaml` schema when the error shape is consistent across endpoints; use endpoint-specific error schemas when the spec defines distinct error structures.
- **`operationId`**: kebab-case, unique within the file. Derived from the HTTP method and path: `create-user`, `get-user-by-id`, `delete-user`.
- **When extending an existing API domain**: the delta file contains only the new paths. Do not duplicate existing paths from the baseline. The merge process (opaque replacement at the file level) means the delta file replaces the baseline file — so when extending, the delta file must include both existing and new paths. Read the baseline file, add the new paths, and write the combined result as the delta.
- **Scope boundary**: contracts capture interface shape only. Authentication schemes (`securitySchemes`), rate limits, caching headers, and versioning strategy belong in `design.md`, not in the contract.

Reference: [openapi-conventions.md](../../references/openapi-conventions.md)

### Step 6: Generate or Update AsyncAPI Binding

When Step 3's delta includes uncovered messaging interactions, generate AsyncAPI 3.0 files under `$CONTRACTS_DIR/messages/`.

**File organisation**: group related channels into a single AsyncAPI file by event domain. Use the same file name as the baseline when extending an existing event domain. For new event domains, name the file after the domain: `<domain>-events.yaml`.

Each file must contain:

```yaml
asyncapi: "3.0.0"
info:
  title: "<Event Domain> Events"
  version: "0.1.0"
  description: "<brief description>"
channels:
  <channelName>:
    address: "<topic.name>"
    messages:
      <messageName>:
        payload:
          $ref: "../schemas/<type>.yaml"
operations:
  <operationName>:
    action: send  # or receive
    channel:
      $ref: "#/channels/<channelName>"
    summary: "<from spec>"
```

Rules:

- **`$ref` to schemas**: all message payloads reference `../schemas/<type>.yaml`. Never inline schema definitions.
- **Channel naming**: derive from the spec's topic or channel names. Use dot-notation when the spec uses it (`user.registered`, `order.placed`).
- **Operations**: each channel has at least one operation. Use `send` for publishing and `receive` for subscribing. The spec determines direction — look for "publish", "emit", "send" vs "subscribe", "consume", "receive" language.
- **When extending an existing event domain**: same rule as OpenAPI — the delta file must include both existing and new channels because merge uses opaque replacement. Read the baseline file, add the new channels and operations, and write the combined result.

Reference: [asyncapi-conventions.md](../../references/asyncapi-conventions.md)

---

## Alignment Report

After running the 6-step algorithm, produce an alignment report. This is the primary output — the delta files are the artifact, but the report is how the brief (and the human) understands what happened.

### Report Format

```markdown
## Alignment Report

### Coverage
- **Covered by baseline:** N interactions (M with alignment warnings)
- **New (delta produced):** N interactions
- **Normalisation:** N files updated with metadata

### Alignment Warnings
<!-- One entry per mismatch found in Step 3's "Already Covered" path -->
- `POST /users`: response schema `User` missing `created_at` field present in spec scenario REQ-003
- `order.placed` channel: message payload missing `currency` field expected by spec scenario REQ-012

### Generated Delta
<!-- One entry per file produced in Steps 4–6 -->
- `contracts/schemas/oauth-token.yaml` (new)
- `contracts/schemas/verification-code.yaml` (new)
- `contracts/http/user-api.yaml` (updated — added `POST /users/verify`)

### Normalisation
<!-- One entry per baseline file that received metadata additions -->
- `contracts/schemas/user.yaml` (added `$id`, `description`)
```

### Report semantics

- **A clean report with zero delta is the expected outcome** for implementation changes in a contract-first workflow. This means the specs align with the pre-existing contracts and no new contract artifacts are needed.
- **A report with warnings** requires human review. The writer does not resolve spec-vs-baseline conflicts — it surfaces them.
- **A report with delta** means the change introduces new API surface. This is normal for contract-only changes (contract-first pattern) and for the spec-first fallback.

---

## Edge Cases

### No specs (contract-only import change)

When `$SPECS_DIR/` is empty or absent, skip Step 2 entirely. Step 3 runs only the normalisation path — checking baseline files for missing Specify metadata. Steps 4–6 produce no output unless normalisation is needed. The alignment report shows zero coverage (no specs to align) and lists any normalisation changes.

### No HTTP interactions in specs

When the specs describe only data types and messaging (no endpoint paths or methods), skip Step 5. Do not create an empty `contracts/http/` directory.

### No messaging interactions in specs

When the specs describe only data types and HTTP endpoints (no channels or topics), skip Step 6. Do not create an empty `contracts/messages/` directory.

### Shared error response type

When multiple endpoints reference error responses with the same shape, generate a single `error-response.yaml` schema and reference it via `$ref` from all endpoints. Do not create per-endpoint error schemas unless the spec defines distinct error structures for different endpoints.

### Baseline schema without `$id`

When the baseline contains a JSON Schema file that lacks `$id`, the writer can still match it by filename convention (kebab-case filename → PascalCase type name). Propose a normalisation delta that adds the `$id` without changing the schema's properties or structure.

### Extending a baseline binding file

When a spec describes a new endpoint on an API domain that already has a baseline binding file, the delta must produce a complete replacement file (not a partial patch). Read the baseline file, merge the new paths/channels into it, and write the combined result to the change-level `contracts/` directory. The existing paths/channels are preserved exactly; only the new ones are added.

---

## Output Hygiene

- Only emit `.yaml` files under `$CONTRACTS_DIR/`.
- Create subdirectories (`schemas/`, `http/`, `messages/`) only when they contain at least one file.
- Do not create empty directories.
- Do not modify any file outside `$CONTRACTS_DIR/`.
- Do not modify baseline files in `.specify/contracts/`.

## Troubleshooting

### Spec describes an interaction but the shape is unclear

When a spec scenario references a type or endpoint but does not provide enough detail to derive the full shape (e.g. "responds with a User object" but no fields listed):

1. Check other spec scenarios that reference the same type — the shape may be described across multiple scenarios.
2. Check the baseline for an existing schema with a matching name — use it as the shape if found.
3. If the shape remains unclear, generate a schema with the known fields and mark unknown fields with a `description: "[unknown] — not specified in current scenarios"` annotation. Record the gap in the alignment report.

### Spec and baseline disagree on a shape

Do not silently override either source. Record the mismatch in the alignment report's Warnings section with both the spec's expected shape and the baseline's actual shape. The human reviewer decides which is correct.

### Circular `$ref` between schemas

JSON Schema permits circular references but they complicate code generation. When a circular `$ref` is unavoidable (e.g. `Order` contains `LineItem` which contains `Order`), generate it correctly but flag it in the alignment report as a code-generation consideration.
