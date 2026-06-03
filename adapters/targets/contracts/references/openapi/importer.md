# OpenAPI — Importer

> **When to read this.** Read this when an operator supplies an external OpenAPI (or Swagger 2.0) document and the contracts adapter build brief needs the file normalised onto Specify conventions under a slice's `contracts/http/` directory. Skip this file when authoring from a spec (use [`author.md`](./author.md)) or when verifying an existing artefact (use [`verifier.md`](./verifier.md)).

## Inputs

```text
$SLICE_DIR     = .specify/slices/<slice-name>
$CONTRACTS_DIR  = $SLICE_DIR/contracts
$BASELINE_DIR   = contracts
```

**Input** — external OpenAPI or Swagger 2.0 files placed by the operator anywhere under `$CONTRACTS_DIR/`. Files may be `.yaml`, `.yml`, or `.json`.

**Output** — normalised OpenAPI 3.1.0 files in `$CONTRACTS_DIR/http/`, with inline schemas decomposed into `$CONTRACTS_DIR/schemas/` and Specify metadata injected. The input files are replaced in-place with their normalised equivalents; decomposed schemas are added as new files.

## Authority hierarchy

When sources conflict:

1. **This file** — import rules and hard constraints.
2. **Format conventions** — [`../../references/openapi-conventions.md`](../../references/openapi-conventions.md), [`../../references/json-schema-conventions.md`](../../references/json-schema-conventions.md).
3. **Source contract** — the external file being imported. Preserve every endpoint, schema, and operation it defines; never silently drop information.
4. **LLM inference** — prohibited for unknowns; mark unrecognised constructs with `[import — manual review required]` and surface in the import report.

## The 6-step import algorithm

The importer runs six sequential steps. Each step assumes its predecessors completed cleanly; if a step cannot make progress on a file, mark the file as `manual review required` and continue with the rest.

### Step 1 — Scan and detect format

Walk `$CONTRACTS_DIR/` for `.yaml`, `.yml`, and `.json` files. For each file, read the top-level keys and classify:

| Detection signal | Classification | Target |
|---|---|---|
| `swagger: "2.0"` | Swagger 2.0 | OpenAPI 3.1 |
| `openapi: "3.0.x"` | OpenAPI 3.0.x | OpenAPI 3.1 |
| `openapi: "3.1.x"` | OpenAPI 3.1.x | No version conversion |
| `asyncapi:` (any version) | **Out of scope.** Route to the `asyncapi` importer sub-flow. |
| `$schema:` (without `openapi`/`asyncapi`/`swagger`) | **Out of scope.** Route to the `json-schema` importer sub-flow. |
| None of the above | Unrecognised. Skip and flag for manual review. |

A file with both `openapi:` and `$schema:` keys is an OpenAPI document (`openapi:` wins). Detection keys are case-sensitive — do not normalise casing before classification.

JSON files (`.json`) are converted to YAML during this step: read the JSON, re-serialise as YAML with `.yaml` extension, then continue normalisation.

When the operator placed the file outside `$CONTRACTS_DIR/http/` (e.g. directly in `$CONTRACTS_DIR/`), record the move target — the file will be relocated in Step 5.

### Step 2 — Upgrade Swagger 2.0 → OpenAPI 3.1

For each Swagger 2.0 file, apply the structural conversion below. The conversion is a verbatim mapping; never invent fields the source does not provide.

#### Top-level fields

| Swagger 2.0 | OpenAPI 3.1 | Notes |
|---|---|---|
| `swagger: "2.0"` | `openapi: "3.1.0"` | Direct replacement. |
| `host` + `basePath` + `schemes` | `servers` array | One entry per scheme; `https` default; omit entirely if `host` absent. |
| `consumes` (root) | Removed (per-operation `content`) | Operation-level `consumes` overrides the root-level value. |
| `produces` (root) | Removed (per-operation `content`) | Same resolution rule as `consumes`. |
| `definitions` | `components/schemas` (temporary) | Decomposed in Step 4. |
| `parameters` (top-level) | `components/parameters` | Structural move only. |
| `responses` (top-level) | `components/responses` | Structural move only. |
| `securityDefinitions` | `components/securitySchemes` | OAuth2 flow names rename — see below. |

#### Server configuration

```yaml
# Swagger 2.0
host: api.example.com
basePath: /v1
schemes: [https, http]

# OpenAPI 3.1
servers:
  - url: https://api.example.com/v1
  - url: http://api.example.com/v1
```

If `host` is absent, omit the `servers` block — contracts capture interface shape, not deployment configuration.

#### Parameters

Path, query, and header parameters carry over with a wrapping `schema` object:

```yaml
# Swagger 2.0
parameters:
  - name: user_id
    in: path
    required: true
    type: string

# OpenAPI 3.1
parameters:
  - name: user_id
    in: path
    required: true
    schema:
      type: string
```

Body parameters become `requestBody`:

```yaml
# Swagger 2.0
parameters:
  - in: body
    name: body
    required: true
    schema:
      $ref: "#/definitions/UserRegistration"

# OpenAPI 3.1
requestBody:
  required: true
  content:
    application/json:
      schema:
        $ref: "#/components/schemas/UserRegistration"
```

Form parameters (`in: formData`) become `requestBody` with `multipart/form-data` (when any `type: file` parameter is present) or `application/x-www-form-urlencoded` (otherwise). `type: file` becomes `type: string, format: binary` in the resulting schema.

#### Responses

Response codes become strings (`200` → `"200"`), and `schema` moves under `content/<media-type>`:

```yaml
# Swagger 2.0
responses:
  200:
    schema:
      $ref: "#/definitions/User"

# OpenAPI 3.1
responses:
  "200":
    content:
      application/json:
        schema:
          $ref: "#/components/schemas/User"
```

Resolve the response media type from the operation's `produces` array (preferring operation-level over root-level), defaulting to `application/json`.

#### `$ref` updates

Update every `$ref` that targets `#/definitions/<Name>` to `#/components/schemas/<Name>`. Step 4 will rewrite these to `../schemas/<name>.yaml` after decomposition.

#### Security definitions

```yaml
# Swagger 2.0
securityDefinitions:
  oauth2:
    type: oauth2
    flow: accessCode
    authorizationUrl: https://auth.example.com/authorize
    tokenUrl: https://auth.example.com/token
    scopes:
      read: Read access

# OpenAPI 3.1
components:
  securitySchemes:
    oauth2:
      type: oauth2
      flows:
        authorizationCode:
          authorizationUrl: https://auth.example.com/authorize
          tokenUrl: https://auth.example.com/token
          scopes:
            read: Read access
```

OAuth2 flow renames:

| Swagger 2.0 | OpenAPI 3.1 |
|---|---|
| `implicit` | `implicit` |
| `password` | `password` |
| `application` | `clientCredentials` |
| `accessCode` | `authorizationCode` |

#### Type-specific mappings

| Swagger 2.0 | OpenAPI 3.1 / JSON Schema |
|---|---|
| `type: file` | `type: string, format: binary` |
| `type: integer, format: int32` | unchanged |
| `type: integer, format: int64` | unchanged |
| `type: number, format: float` | unchanged |
| `type: number, format: double` | unchanged |

Write the upgraded content back to the file. The file is now in OpenAPI 3.1 form but may still contain inline `components/schemas`.

### Step 3 — Upgrade OpenAPI 3.0.x → 3.1.0

For each OpenAPI 3.0.x file, apply the targeted JSON Schema alignment changes:

#### Version

```yaml
# OpenAPI 3.0
openapi: "3.0.3"

# OpenAPI 3.1
openapi: "3.1.0"
```

#### Nullable handling

OpenAPI 3.0's `nullable: true` becomes a JSON Schema 2020-12 type union:

```yaml
# OpenAPI 3.0
properties:
  nickname:
    type: string
    nullable: true

# OpenAPI 3.1
properties:
  nickname:
    type:
      - string
      - "null"
```

Remove the `nullable` keyword after conversion.

#### Exclusive min/max

```yaml
# OpenAPI 3.0 (Draft 4 style)
properties:
  age:
    type: integer
    minimum: 0
    exclusiveMinimum: true

# OpenAPI 3.1 (Draft 2020-12)
properties:
  age:
    type: integer
    exclusiveMinimum: 0
```

In 3.0, `exclusiveMinimum` / `exclusiveMaximum` are booleans that modify `minimum` / `maximum`. In 3.1 they are standalone numeric values.

#### Example → examples

Schema-level `example` (singular) is deprecated in 3.1. Convert to `examples` (plural array):

```yaml
# OpenAPI 3.0
properties:
  email:
    type: string
    example: "user@example.com"

# OpenAPI 3.1
properties:
  email:
    type: string
    examples:
      - "user@example.com"
```

Note: `example` (singular) on **media type objects** and **parameter objects** remains valid in 3.1 — only schema-level `example` is deprecated.

#### Other JSON Schema differences

| OpenAPI 3.0 | OpenAPI 3.1 |
|---|---|
| `$ref` cannot have sibling keywords | `$ref` may have sibling keywords (3.1 is a superset) — no change needed |
| Draft 4 subset | Draft 2020-12 — `$schema` may be declared on schemas if useful |
| No `const` / `if` / `then` / `else` | Available in 3.1 — no migration required for existing input |

#### Webhooks

OpenAPI 3.1 introduces a `webhooks` top-level key. Importer never adds it — that is new functionality, not a migration of existing content.

Write the upgraded content back to the file. Files already at 3.1 skip Steps 2 and 3 entirely.

### Step 4 — Decompose inline schemas

For every OpenAPI file (whether upgraded or already at 3.1), scan for inline schema definitions and extract them to standalone files in `$CONTRACTS_DIR/schemas/`. The schemas are owned by the json-schema format skill once they land — the importer just creates them.

#### What counts as inline

- **`components/schemas/<Name>`** — definitions inherited from a Swagger 2.0 `definitions` block or already in `components/schemas`.
- **Inline request body schemas** under `paths.<path>.<method>.requestBody.content.<media-type>.schema` (no `$ref`).
- **Inline response body schemas** under `paths.<path>.<method>.responses.<status>.content.<media-type>.schema` (no `$ref`).
- **Inline parameter schemas** in `parameters[].schema` when the parameter's schema is an object with multiple properties (simple primitives stay inline).

Schemas that are already `$ref` pointers to `../schemas/` are left untouched.

#### Filename derivation

| Context | Naming rule | Example |
|---|---|---|
| `components/schemas/<Name>` | Kebab-case the key | `User` → `user.yaml` |
| Inline schema with `title:` | Kebab-case the title | `title: "User Adapter"` → `user-adapter.yaml` |
| `paths./users.post.requestBody` (no title) | `<resource>-<action>-request` | `user-create-request.yaml` |
| `paths./users.post.responses.201` (no title) | Use baseline schema name if shape matches; else `<resource>-<action>-response` | `user-create-response.yaml` |
| `paths./users/{user_id}.get.responses.200` (no title) | Use the resource singular | `user.yaml` |

Disambiguation: when two extracted schemas would produce the same filename, append the API domain (`user-billing.yaml` vs `user.yaml`). Filenames are kebab-case with a `.yaml` extension; one type per file.

#### Baseline conflict check

Before writing each extracted schema, compare it to any existing file with the same name in `$BASELINE_DIR/schemas/`:

- **Structurally equivalent** (same `properties`, `required`, types) — drop the extracted file and replace inline references with `$ref` to the baseline file. No new schema file in the slice.
- **Differs structurally** — disambiguate by prefixing with the API domain (`user-billing.yaml`) and write the new file.

#### Replacement

Write each extracted schema to `$CONTRACTS_DIR/schemas/<name>.yaml`. Replace the inline definition with:

```yaml
schema:
  $ref: "../schemas/<name>.yaml"
```

After decomposition, walk every `$ref` in the OpenAPI document and rewrite `#/components/schemas/<Name>` to `../schemas/<name>.yaml`. When `components/schemas` is empty, remove the block. When `components` itself is empty, remove the block too.

#### Nested inline sub-schemas

When an extracted schema itself contains inline sub-schemas (nested objects):

- **Used only inside this parent** — keep it inline, optionally inside `$defs`.
- **Used elsewhere too** — extract to its own file and `$ref` from both locations.

### Step 5 — Inject Specify metadata

For every schema file in `$CONTRACTS_DIR/schemas/` (newly decomposed and pre-existing), inject Specify-required metadata where missing. Never overwrite values that the source already provided.

| Field | Rule | Generation |
|---|---|---|
| `$schema` | `"https://json-schema.org/draft/2020-12/schema"` | Add if absent. Update older draft URIs to 2020-12 (see [`../../references/json-schema-conventions.md`](../../references/json-schema-conventions.md)). |
| `$id` | `"urn:specify:schemas/<filename-without-extension>"` | Generate from the file path. Never reassign an existing `$id` that matches a baseline schema. |
| `title` | PascalCase type name | Derive from filename: `user-registration.yaml` → `UserRegistration`. Do not overwrite existing `title`. |
| `description` | Non-empty string | If absent, set to `"[imported — description pending review]"` and surface in the import report. |

For the OpenAPI document itself, verify that `info.title`, `info.version`, and `info.description` are present. Inject `info.description: "[imported — description pending review]"` if missing.

**Contract normalisation rules for top-level OpenAPI documents:**

- **`info.version` MUST be SemVer.** When the imported value does not parse as SemVer (e.g. `2024-01-15`, `v2`, `"1"`), do **not** auto-rewrite. Surface a `[manual review required]` entry in the import report naming the file and the offending value, and let the operator decide on the canonical SemVer string. The single-mode verifier (Check 4) and the merge-time `specify tool run contract` gate (the contracts adapter merge contract) will block on the unaltered value until the operator resolves it.
- **Preserve `info.x-specify-id` verbatim.** When the source carries `info.x-specify-id`, copy it through unchanged — even when the value violates the kebab-case format (the verifier flags the format issue with the file path, which is enough for the operator to fix). Never invent or auto-derive an id during import; new ids are an authoring decision.

### Step 6 — Place files, validate, report

#### Place files

Move each file to its canonical subdirectory under `$CONTRACTS_DIR/`:

| File type | Target | Trigger |
|---|---|---|
| OpenAPI files | `$CONTRACTS_DIR/http/` | Top-level `openapi:` key |
| JSON Schema files (decomposed) | `$CONTRACTS_DIR/schemas/` | Step 4 output |

Remove the original file when the canonical location differs from where the operator placed it. Create subdirectories only when they will contain at least one file.

#### Validate

Run [`verifier.md`](./verifier.md) in `single` mode against `$SLICE_DIR` to confirm `$ref` resolution, schema metadata completeness, and binding coverage. If the verifier reports issues, re-enter Steps 4–5 for targeted repair before producing the report.

#### Report

Produce a markdown import report:

```markdown
## Import Report (HTTP)

### Files Processed
- **Total input files:** N
- **Swagger 2.0 → OpenAPI 3.1:** N
- **OpenAPI 3.0 → 3.1:** N
- **Already at OpenAPI 3.1:** N
- **JSON-only inputs converted to YAML:** N
- **Unrecognised (skipped):** N

### Inline Schema Decomposition
- **Schemas extracted:** N
- **Baseline duplicates avoided:** N (matched existing baseline schemas)
- `components/schemas/User` → `contracts/schemas/user.yaml`
- `paths./users.post.requestBody` → `contracts/schemas/user-create-request.yaml`

### Metadata Injected
- `contracts/schemas/user.yaml` — added `$id`, `$schema`
- `contracts/http/user-api.yaml` — added `info.description`

### Validation Result
All checks passed (N $ref pointers, N schemas, N bindings verified).

### Manual Review Required
- `unknown-format.yaml` — missing `openapi`/`swagger` key, no JSON Schema signature.
- `legacy-api.yaml` — `x-internal-billing-tier` extension preserved but not validated.
```

Report semantics:

- **Zero manual review items** is the ideal outcome — every file detected, upgraded, decomposed, and metadata-injected automatically.
- **Manual review items are expected for complex imports.** Vendor-specific constructs and unclassifiable files surface here rather than being silently dropped.
- **The validation result confirms internal consistency.** If the verifier reports issues, fix and re-run — do not finalise the report until the verifier passes.

## Edge cases

| Scenario | Handling |
|---|---|
| Mixed input formats (Swagger 2.0 + OpenAPI 3.0 + JSON Schema) in one directory | Process each file independently. JSON Schema files are out of scope — route them to the `json-schema` importer sub-flow. |
| OpenAPI file `$ref`s a sibling file in `$CONTRACTS_DIR/` | Process the referenced file first; rewrite the `$ref` to the post-decomposition path. |
| Swagger 2.0 file with external `$ref` (URL or absolute path) | Cannot auto-resolve. Flag in the report; never silently drop. |
| Name collision during decomposition (two distinct schemas, same derived filename) | Disambiguate by prefixing with the source API domain (`user-api-error.yaml` vs `billing-api-error.yaml`). |
| Empty `components/schemas` after decomposition | Remove the block; remove the parent `components` block if it is now empty. |
| Vendor extensions (`x-*` keys) | Preserve verbatim during upgrade and decomposition. Note their presence in the report; never validate or transform them. |
| File contains multiple YAML documents (`---` separators) | Rare. Process the first document; flag the rest in the report for manual review. |
| Original OpenAPI file uses `example` (singular) at media-type or parameter scope | Preserve. Only schema-level `example` becomes `examples` (plural). |

## Hard rules

1. **No data loss.** Every endpoint, response, parameter, schema, and security scheme in the source must be present in the output. Information may be restructured but not silently dropped.
2. **Valid OpenAPI 3.1.** Every output file must parse as OpenAPI 3.1.0.
3. **One type per schema file** after decomposition.
4. **Kebab-case `.yaml` filenames** for both OpenAPI and decomposed schema files.
5. **`$ref` resolution.** Every `$ref` in the output must resolve to a file in `$CONTRACTS_DIR/schemas/`, `$BASELINE_DIR/schemas/`, or (for `components/parameters`, `components/securitySchemes`) within the same OpenAPI document.
6. **`$id` stability.** Never reassign a baseline `$id` value. New schemas get fresh `$id` values from the file path.
7. **Baseline preservation.** Never modify any file in root `contracts/`.

## Verification checklist

Before completing the import:

- [ ] Every input file classified — format detected or flagged for review.
- [ ] All Swagger 2.0 files upgraded to OpenAPI 3.1.
- [ ] All OpenAPI 3.0.x files upgraded to OpenAPI 3.1.
- [ ] All inline schemas decomposed to `$CONTRACTS_DIR/schemas/`.
- [ ] All `$ref` pointers updated to use `../schemas/` convention.
- [ ] All schema files have `$id`, `$schema`, `title`, `description`.
- [ ] All OpenAPI files have `info.title`, `info.version`, `info.description`.
- [ ] Files placed in correct subdirectories (`http/`, `schemas/`).
- [ ] [`verifier.md`](./verifier.md) (single mode) ran clean.
- [ ] Import report produced with per-file results and manual-review items.
- [ ] No baseline files modified.

## See also

- [`../../references/openapi-conventions.md`](../../references/openapi-conventions.md) — target OpenAPI 3.1 conventions.
- [`../../references/json-schema-conventions.md`](../../references/json-schema-conventions.md) — target JSON Schema conventions for decomposed payloads.
- [`../../references/artifact-structure.md`](../../references/artifact-structure.md) — directory layout for the post-import baseline shape.
- [`../../references/import-upgrade-policy.md`](../../references/import-upgrade-policy.md) — cross-format framework for format detection, upgrade targets, lossless-vs-lossy decisions, and "when to refuse and ask the operator" cases.
- [`../../references/baseline-vs-delta.md`](../../references/baseline-vs-delta.md) — `$id` stability, baseline immutability, and the contract-given authorship pattern this importer realises.
- [`../../references/report-shape.md`](../../references/report-shape.md) — markdown shape for the import report produced at the end of Step 6.
- [`author.md`](./author.md) — sibling for spec-driven authoring.
- [`verifier.md`](./verifier.md) — sibling for validating imported output.
