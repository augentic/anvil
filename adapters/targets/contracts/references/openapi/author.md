# OpenAPI — Author

> **When to read this.** Read this when authoring or extending the OpenAPI document for a Specify change — i.e. when the contracts adapter build brief during `/spec:build` selects the author intent, or an operator wants to add new HTTP interactions to the platform's HTTP baseline. Skip this file when importing an external document (use [`importer.md`](./importer.md)) or when verifying an existing artefact (use [`verifier.md`](./verifier.md)).

## Inputs

```text
$SLICE_DIR     = .specify/slices/<slice-name>
$SPECS_DIR      = $SLICE_DIR/specs
$CONTRACTS_DIR  = $SLICE_DIR/contracts
$BASELINE_DIR   = contracts
```

## Authority hierarchy

When sources conflict, follow this strict precedence:

1. **This file** — author rules and hard constraints for OpenAPI documents.
2. **Specify artefacts** (specs) — behavioural requirements drive the operations.
3. **Format conventions** — [`../../references/openapi-conventions.md`](../../references/openapi-conventions.md), [`../../references/json-schema-conventions.md`](../../references/json-schema-conventions.md).
4. **Baseline contracts** (`contracts/http/`) — existing platform vocabulary; never overwrite silently.
5. **LLM inference** — prohibited for unknowns; mark with `[unknown]` and surface in the alignment report.

If the specs and baseline disagree on a shape, surface the mismatch in the alignment report's Warnings section. Never silently overwrite baseline operations to match the specs — a human reviewer decides.

## The 4-step author algorithm

The author runs four steps end-to-end whenever the contracts adapter build brief asks for HTTP coverage. Each step is a focused, independently-checkable phase; downstream steps assume the upstream output is well-formed.

### Step 1 — Read the baseline

Build an inventory of `$BASELINE_DIR/http/`:

| Source | Extract |
|---|---|
| `http/*.yaml` | OpenAPI `paths` (path + method), `operationId`, request body `$ref` targets, response status codes and schema `$ref` targets, `parameters`, `securitySchemes` |

For each operation, record:

- **Identity** — `(path, method)` tuple plus `operationId`.
- **Shape** — request body schema, parameter list, per-status response schemas, content types.
- **File** — relative path from root `contracts/`.

When `$BASELINE_DIR/http/` is empty or absent, the baseline is empty — every spec interaction becomes delta. Record an empty inventory and proceed.

### Step 2 — Map specs to operations

Read every `*.md` file under `$SPECS_DIR/` and harvest HTTP-shaped requirements. A spec scenario maps to a `paths.<path>.<method>` entry when it describes:

- An endpoint path and HTTP verb (`POST /users`, `GET /orders/{order_id}`).
- A request payload (field names, types, required-ness).
- A response payload (field names, types) and one or more status codes.
- An error condition tied to a specific status code (`409 Conflict`, `404 Not Found`, `422 Unprocessable Entity`).

Build a structured list of **spec interactions**, one per `(path, method)` tuple:

```text
- identity: POST /users/verify
  request_body: VerificationCodeRequest (fields: email, code)
  responses:
    "200": User (fields: id, email, verified_at)
    "400": ErrorResponse
    "410": ErrorResponse  (code expired)
  source: specs/auth.md REQ-014
```

When a spec scenario references a payload type by name (e.g. "respond with a `User`"), check whether the type is defined in this slice's `$SPECS_DIR/` or already in `$BASELINE_DIR/schemas/`. The schema is owned by the json-schema format skill — your job is only to wire the `$ref` correctly.

When the slice has **no specs** (e.g. an importer-only change followed by a normalisation pass), skip steps 2–4 and route to [`importer.md`](./importer.md).

### Step 3 — Compute the minimal delta

Compare each spec interaction from Step 2 against the baseline inventory from Step 1. Classify into one of three buckets:

#### Already covered

The baseline already has a matching `(path, method)`. Verify alignment:

- Endpoint path and method match.
- Request body schema includes the fields the spec references.
- Response schema includes the fields the spec asserts.
- Spec-asserted status codes are present in the baseline's `responses`.

If alignment fails, record a warning with `{ baseline_file, spec_requirement_id, discrepancy }` for the alignment report. **Do not regenerate covered operations** and **do not overwrite the baseline** — flag the mismatch and let a human resolve it.

#### New or modified

The spec describes an operation that is absent from the baseline, or a baseline file needs new operations on the same API domain. Add to the OpenAPI delta:

- New `(path, method)` tuples in either an existing baseline binding file or a new domain file.
- New response status codes on an existing operation (only when the spec requires them).
- New request body fields (only when the spec asserts them).

When extending an existing API domain (e.g. adding `POST /users/verify` to `user-api.yaml` which already defines `POST /users` and `GET /users/{user_id}`), the delta file must contain **both the existing operations and the new ones**. Merge is opaque file replacement: the slice-level file replaces the baseline file wholesale, so omitting existing operations would silently delete them.

#### Normalisation

The baseline file lacks Specify-required metadata (`info.title`, `info.version`, `info.description`). Propose a normalisation delta that adds the metadata without changing the operations. Surface as a separate section in the alignment report.

### Step 4 — Generate or update OpenAPI files

For every API domain in the delta, write a file under `$CONTRACTS_DIR/http/<domain>.yaml`. File naming follows kebab-case: `user-api.yaml`, `billing-api.yaml`, `notification-api.yaml`. Group related operations into one file by domain, never per-method or per-resource.

Required structure:

```yaml
openapi: "3.1.0"
info:
  title: User API
  version: "1.0.0"
  description: User registration, authentication, and adapter management.
paths:
  /users:
    post:
      operationId: createUser
      summary: Register a new user account.
      requestBody:
        required: true
        content:
          application/json:
            schema:
              $ref: "../schemas/user-registration.yaml"
      responses:
        "201":
          description: User created successfully.
          content:
            application/json:
              schema:
                $ref: "../schemas/user.yaml"
        "409":
          description: Email already registered.
          content:
            application/json:
              schema:
                $ref: "../schemas/error-response.yaml"
```

The full structural rules — path conventions, method semantics, response code defaults, `operationId` patterns, content types, scope boundary — live in [`../../references/openapi-conventions.md`](../../references/openapi-conventions.md). Read it before authoring; the rules below are OpenAPI-only deltas and rules-of-thumb that complement the convention reference.

## Schema reuse and `$ref` discipline

Shared payload schemas live in `contracts/schemas/` and are owned by the `json-schema` sub-flow. The author of an OpenAPI file does **not** create or edit schema files — it only references them.

- **Always `$ref`** request bodies, response bodies, and reusable parameter schemas to `../schemas/<type>.yaml`.
- **Never inline** a domain type. If the spec mentions a new payload type, route the schema work to the `json-schema` sub-flow (or, in the same `/spec:build` invocation, rely on the contracts adapter build brief's fixed `json-schema`-first ordering).
- **`$ref` resolution scope.** All `$ref` paths must resolve either to `$CONTRACTS_DIR/schemas/` (this slice's delta) or `$BASELINE_DIR/schemas/` (the platform baseline). The verifier flags any `$ref` that does not resolve.
- **Inline `$defs`** for one-shot sub-objects used only inside one parent payload are acceptable in the schema files themselves, but not in the OpenAPI document. Keep the OpenAPI side a flat list of `$ref` pointers.
- **`components/schemas` is forbidden** for domain types. The `components` block may host `parameters`, `headers`, or `securitySchemes` (see Auth below).

## Baseline-delta computation rules

OpenAPI deltas fall into three categories — every operation in the delta belongs to exactly one:

| Category | Trigger | Effect on the delta file |
|---|---|---|
| **Operations added** | `(path, method)` not in the baseline | New entry under `paths.<path>.<method>` |
| **Operations modified** | Baseline operation, but the spec asserts a new status code, response field, or required request field | Edit the baseline operation in-place inside the delta file (preserving every other property byte-for-byte) and surface the diff in the alignment report |
| **Operations removed** | Baseline operation that no spec scenario references and the slice explicitly deprecates it | **Out of scope.** OpenAPI deltas have no remove semantics — removal is a manual baseline edit. Surface as a warning in the alignment report so a human can act |

Computation rules applied at file scope:

1. **One file per API domain.** Always read the matching baseline file first. The delta file replaces it wholesale at merge time.
2. **`info.version` MUST parse as SemVer (contract identity/version validation).** New top-level OpenAPI documents MUST set `info.version` to a value that parses per [semver.org](https://semver.org), including optional prerelease labels (`1.0.0-draft.1`). Do not bump the baseline's `info.version` automatically — version policy is a platform decision, not an authoring decision. If the slice requires a version bump, the contracts adapter build brief flags it for human review. The verifier sibling enforces SemVer in single mode (Check 4), and `specify tool run contract` enforces it again at merge time on the baseline (the contracts adapter merge contract); a non-SemVer value is a hard validation failure at both gates.
3. **`info.x-specify-id` rename-stable identifier (contract identity/version validation).** SHOULD set `info.x-specify-id` on every new top-level OpenAPI document to a kebab-case slug (typically the file stem; `^[a-z][a-z0-9-]*$`, ≤ 64 characters). The id is a hint that survives file moves and version bumps. MUST preserve any pre-existing `info.x-specify-id` when extending the baseline; MUST NOT change it across `info.version` bumps. Path-based references in `registry.yaml` remain canonical — the id is a rename-stable hint, not a substitute.
4. **Preserve `operationId` keys.** When extending a baseline file, every existing operation's `operationId` stays exactly as it is. New operations get fresh kebab-cased or camelCased `operationId` values that are unique across the contract tree.
5. **Diff at the operation level.** When modifying an existing operation, change only the keys the spec asserts. Do not reformat or reorder unrelated keys — opaque file replacement means a re-ordered file looks like a wholesale rewrite to reviewers.

## Examples (`examples` keyword)

OpenAPI 3.1 supports `examples` (plural array) at media-type, parameter, and schema scope. Attach examples sparingly:

- **When the spec includes a concrete example payload** (e.g. "the response includes `{"id":"u_123","email":"a@b.com"}`"), copy it verbatim into the operation's `responses.<status>.content.application/json.examples` entry under a meaningful key.
- **When the spec describes only the field shape**, do not invent example values. The schema's field types and constraints carry enough for downstream code generation.
- **Do not emit `example` (singular)** — it is deprecated in OpenAPI 3.1 schema scope. The importer normalises 3.0-style `example` to `examples` (plural array); authored output must use the plural form from the start.

OpenAPI 3.1 example shape:

```yaml
responses:
  "200":
    description: User adapter.
    content:
      application/json:
        schema:
          $ref: "../schemas/user.yaml"
        examples:
          minimal:
            summary: Minimal valid response
            value:
              id: u_123
              email: a@b.com
              created_at: "2026-01-01T00:00:00Z"
```

## Authentication and security schemes

`securitySchemes` belong in the contract **only when the spec requires the consumer to send a specific authentication header for request validation** — i.e. when omitting auth would change the wire shape. Otherwise, authentication policy lives in `design.md`, not in the OpenAPI document. See [`../../references/openapi-conventions.md`](../../references/openapi-conventions.md) §Scope Boundary.

When a `securityScheme` is required by the spec, declare it under `components/securitySchemes` and reference it via the operation-level `security` array:

```yaml
components:
  securitySchemes:
    bearerAuth:
      type: http
      scheme: bearer
      bearerFormat: JWT
    oauth2:
      type: oauth2
      flows:
        authorizationCode:
          authorizationUrl: https://auth.example.com/authorize
          tokenUrl: https://auth.example.com/token
          scopes:
            read: Read access
            write: Write access
paths:
  /users/me:
    get:
      operationId: getCurrentUser
      security:
        - bearerAuth: []
      responses:
        "200":
          description: Current user adapter.
          content:
            application/json:
              schema:
                $ref: "../schemas/user.yaml"
        "401":
          description: Missing or invalid bearer token.
          content:
            application/json:
              schema:
                $ref: "../schemas/error-response.yaml"
```

OAuth2 flow names:

| Use case | OpenAPI 3.1 flow |
|---|---|
| Browser-based authorization with redirect | `authorizationCode` |
| Native app / single-page app implicit | `implicit` |
| Resource-owner credentials | `password` |
| Service-to-service tokens | `clientCredentials` |

Never invent scopes. The spec must list the scope identifiers and their meanings; if it does not, mark the `scopes` map as `"[unknown]"` in the alignment report.

## Alignment report

Every author run produces an alignment report alongside the delta files. The report is the primary output for the contracts adapter build brief — the YAML files are the artefact, but the report is how the brief decides whether the slice can proceed.

```markdown
## Alignment Report (HTTP)

### Coverage
- **Covered by baseline:** N operations (M with alignment warnings)
- **New (delta produced):** N operations
- **Normalisation:** N files updated with metadata

### Alignment Warnings
- `POST /users`: response schema `User` missing `created_at` field present in spec scenario REQ-003
- `GET /orders/{order_id}`: spec asserts `404` response but binding only defines `200` and `500`

### Generated Delta
- `contracts/http/user-api.yaml` (updated — added `POST /users/verify`)
- `contracts/http/billing-api.yaml` (new)

### Normalisation
- `contracts/http/user-api.yaml` (added `info.description`)
```

Report semantics:

- **Zero delta with zero warnings** is the expected outcome for an implementation slice in a contract-first workflow — specs already align with the pre-existing contract.
- **Warnings require human review.** The author never resolves spec-vs-baseline mismatches automatically.
- **A non-empty delta** is normal for contract-only changes and for spec-first changes where the baseline is empty.

After producing the report, run [`verifier.md`](./verifier.md) in `single` mode against `$SLICE_DIR` to validate `$ref` resolution, schema metadata, and binding coverage before declaring the artefact ready.

## Edge cases

| Scenario | Handling |
|---|---|
| Spec references a payload type not yet authored | Mark `[unknown]` in the report; the json-schema skill (called first by the contracts adapter build brief) should have produced the schema. If it did not, halt and surface the gap. |
| Spec asserts a status code with no response shape | Use `$ref: "../schemas/error-response.yaml"` (or the spec-named error schema) and add a one-sentence `description` derived from the spec's wording. |
| Two specs claim the same `(path, method)` with different shapes | Surface the conflict as a warning; do not write a delta until the specs are reconciled. |
| Baseline operation uses `components/schemas` (legacy from a manual import) | Do not propagate the inline form into the delta. Run [`importer.md`](./importer.md) on the baseline file first, then re-author. |
| Spec describes pagination | Use the standard `limit` / `offset` query params from [`../../references/openapi-conventions.md`](../../references/openapi-conventions.md) §Pagination unless the spec requires cursor-based — in which case define a pagination wrapper schema via the `json-schema` sub-flow. |

## Verification checklist

Before declaring the author run complete:

- [ ] Every spec-described HTTP interaction maps to an operation in either the baseline or the delta.
- [ ] All `$ref` pointers in the delta resolve into `$CONTRACTS_DIR/schemas/` or `$BASELINE_DIR/schemas/`.
- [ ] No domain types are inlined; every request body and response body uses `$ref`.
- [ ] When extending a baseline file, every existing operation is preserved verbatim alongside the new ones.
- [ ] Alignment report enumerates coverage, warnings, generated delta files, and normalisation entries.
- [ ] [`verifier.md`](./verifier.md) (single mode) ran clean against `$SLICE_DIR`.

## See also

- [`openapi-conventions`](../../references/openapi-conventions.md) — file structure, paths, methods, response codes, `operationId`.
- [`artifact-structure`](../../references/artifact-structure.md) — directory layout for the slice-local delta and the baseline.
- [`baseline-vs-delta`](../../references/baseline-vs-delta.md) — cross-format rules for the three authorship patterns, the already-covered / new-or-modified / normalisation classification, and the opaque-file-replacement merge contract that the §Baseline-delta computation rules above operationalise for OpenAPI.
- [`report-shape`](../../references/report-shape.md) — markdown shape for the alignment report produced by this author path.
- [`json-schema-conventions`](../../references/json-schema-conventions.md) — schema files referenced by the OpenAPI document (owned by the `json-schema` sub-flow).
- [`importer.md`](./importer.md) — sibling for normalising external OpenAPI documents.
- [`verifier.md`](./verifier.md) — sibling for validating the authored output.
