---
name: importer
description: "Import and normalize external API contracts — detect formats, upgrade versions, decompose inline schemas, and inject Specify metadata."
license: MIT
allowed-tools: Read Write StrReplace Shell Grep Glob
---

# Contracts Importer

Import and normalize external API contracts into the Specify ecosystem. The importer detects input formats, upgrades older specifications to their target versions, decomposes inline schema definitions into separate files, and injects Specify metadata where missing.

The importer is a Layer 2 skill. In Layer 1, operators perform these steps manually — placing OpenAPI 3.1 / AsyncAPI 3.0 / JSON Schema files into the change's `contracts/` directory by hand. The importer automates the manual workflow: accept whatever the operator provides, normalize it, and produce contract files that conform to Specify conventions.

## Authority Hierarchy

When conflicts arise, follow this strict precedence:

1. **This SKILL.md** (highest) — import rules and hard constraints
2. **references/** — format detection and upgrade rules
3. **Specify conventions** — JSON Schema, OpenAPI, AsyncAPI conventions from `plugins/contracts/references/`
4. **Source contract** — the external file being imported
5. **LLM inference** (lowest) — prohibited for unknowns; use `[unknown]` markers

If a source contract uses constructs that have no clear mapping in the target format, mark the construct with `[import — manual review required]` rather than guessing. If a reference document and this skill disagree, this skill wins.

## Hard Rules

Violations of any rule below fail the import. There are no exceptions.

1. **Valid output format** — every imported file must produce valid OpenAPI 3.1, AsyncAPI 3.0, or JSON Schema (draft 2020-12) as appropriate.
2. **No data loss** — every endpoint, channel, schema, and operation in the source must be present in the output. Information may be restructured but not silently dropped.
3. **`$ref` resolution** — all `$ref` pointers in the output must resolve: either within the same file, to sibling files in the change's `contracts/`, or to existing files in the baseline `.specify/contracts/`.
4. **One type per schema file** — after decomposition, each schema file defines exactly one top-level type.
5. **Kebab-case naming** — all output file names use kebab-case with `.yaml` extensions.
6. **`$id` stability** — when importing into a baseline that already has schemas with `$id` values, do not reassign those `$id` values. New schemas get new `$id` values.
7. **Baseline preservation** — never modify baseline files in `.specify/contracts/`. All output goes in the change-level `contracts/` directory.

## Arguments

```text
$CHANGE_DIR     = .specify/changes/<change-name>
$CONTRACTS_DIR  = $CHANGE_DIR/contracts
$BASELINE_DIR   = .specify/contracts
```

**Input**: External contract files placed by the operator into `$CONTRACTS_DIR/`.

**Output**: Normalized contract files in `$CONTRACTS_DIR/`, conforming to Specify conventions. The input files are replaced in-place with their normalized equivalents; decomposed schemas are added as new files under `$CONTRACTS_DIR/schemas/`.

## Required References

Before importing, read these documents:

1. [format-detection.md](references/format-detection.md) — how to identify the input format from top-level YAML keys
2. [upgrade-rules.md](references/upgrade-rules.md) — version conversion rules for each upgrade path
3. [json-schema-conventions.md](../../references/json-schema-conventions.md) — `$id` format, metadata rules, `$ref` conventions
4. [openapi-conventions.md](../../references/openapi-conventions.md) — OpenAPI 3.1 structure and naming
5. [asyncapi-conventions.md](../../references/asyncapi-conventions.md) — AsyncAPI 3.0 structure and naming
6. [artifact-structure.md](../../references/artifact-structure.md) — directory layout and delta rules

---

## The 7-Step Algorithm

### Step 1: Scan for Input Files

Scan `$CONTRACTS_DIR/` recursively for all `.yaml` and `.json` files. Build a manifest of files to process.

If `$CONTRACTS_DIR/` does not exist or contains no files, report that there is nothing to import and stop.

### Step 2: Detect Formats

For each file, read the top-level keys and classify the format using the rules in [format-detection.md](references/format-detection.md).

| Input Format | Detection Signal | Target Format |
|---|---|---|
| Swagger 2.0 | `swagger: "2.0"` | OpenAPI 3.1 |
| OpenAPI 3.0.x | `openapi:` starts with `3.0` | OpenAPI 3.1 |
| OpenAPI 3.1.x | `openapi:` starts with `3.1` | No version conversion needed |
| AsyncAPI 2.x | `asyncapi:` starts with `2.` | AsyncAPI 3.0 |
| AsyncAPI 3.0.x | `asyncapi:` starts with `3.0` | No version conversion needed |
| Standalone JSON Schema | `$schema:` present, no `openapi`/`asyncapi`/`swagger` key | Place in `schemas/` |
| Unrecognized | None of the above keys present | Flag for human review |

Record the classification for each file. If any file cannot be classified, add it to the import report as requiring manual review and skip it for the remaining steps.

### Step 3: Upgrade Versions

For each file that requires version conversion, apply the upgrade rules from [upgrade-rules.md](references/upgrade-rules.md).

**Swagger 2.0 → OpenAPI 3.1:**
- `swagger: "2.0"` → `openapi: "3.1.0"`
- `host` + `basePath` + `schemes` → `servers` array
- `definitions` → `components/schemas` (temporary; Step 4 will decompose)
- Body `parameters` → `requestBody` with `content`
- `produces` / `consumes` → per-operation `content` types
- Response `schema` → `content/<media-type>/schema`
- `$ref: "#/definitions/Foo"` → appropriate target (updated in Step 4)

**OpenAPI 3.0.x → OpenAPI 3.1:**
- `openapi: "3.0.x"` → `openapi: "3.1.0"`
- `nullable: true` on a property → `type: ["<original-type>", "null"]`
- `example` → `examples` (array form) where applicable
- `exclusiveMinimum: true` + `minimum: N` → `exclusiveMinimum: N`
- `exclusiveMaximum: true` + `maximum: N` → `exclusiveMaximum: N`

**AsyncAPI 2.x → AsyncAPI 3.0:**
- `asyncapi: "2.x.x"` → `asyncapi: "3.0.0"`
- Channel items restructured: operations separated from channels
- `publish` → operation with `action: send`
- `subscribe` → operation with `action: receive`
- Message definitions moved to `components/messages` with channel-scoped refs
- Channel `servers` reference updated to top-level `servers`

Write the upgraded content back to the same file (in-place replacement). The file is now in the target format but may still contain inline schemas.

### Step 4: Decompose Inline Schemas

For each OpenAPI or AsyncAPI file (including those that were already in the target version), scan for inline schema definitions and extract them to separate files.

#### What counts as "inline"

An inline schema is any schema definition that appears directly in an OpenAPI or AsyncAPI file rather than as a `$ref` pointer to an external file. This includes:

- **OpenAPI `components/schemas`** — definitions that arrived from a Swagger 2.0 `definitions` block or were already in `components/schemas`.
- **OpenAPI request/response bodies** — schemas defined directly under `content/<media-type>/schema` instead of via `$ref`.
- **AsyncAPI message payloads** — schemas defined directly under `payload` instead of via `$ref`.

Schemas that are already `$ref` pointers to `../schemas/` are left untouched.

#### Decomposition process

For each inline schema found:

1. **Determine the file name.** Derive a kebab-case name from the schema's context:

   | Context | Naming Rule | Example |
   |---|---|---|
   | `components/schemas/Foo` | Kebab-case the key | `foo.yaml` |
   | `paths./users.post.requestBody` | `<resource>-<action>-request` | `user-create-request.yaml` |
   | `paths./users.post.responses.201` | `<resource>-<action>-response` | `user-create-response.yaml` (or the type name if `title` is present) |
   | `paths./users/{id}.get.responses.200` | Use `title` if present; else `<resource>` | `user.yaml` |
   | AsyncAPI `payload` inline | `<channel-concept>` | `order-placed.yaml` |

   When a `title` is present on the inline schema, prefer the kebab-case version of the title as the filename. When multiple schemas would produce the same filename, append a disambiguating suffix (e.g. `-request`, `-response`, `-payload`).

2. **Check for baseline conflicts.** If a schema with the same filename already exists in `$BASELINE_DIR/schemas/`, compare the shapes. If they are structurally equivalent (same properties, types, and required fields), use a `$ref` to the baseline file instead of creating a duplicate. If they differ, use a disambiguated filename (e.g. append the API domain: `user-billing.yaml`).

3. **Write the schema file** to `$CONTRACTS_DIR/schemas/<name>.yaml`.

4. **Replace the inline definition** with a `$ref` pointer: `$ref: "../schemas/<name>.yaml"`.

5. **Handle nested inline schemas.** If the extracted schema itself contains inline sub-schemas (nested objects), decide based on reuse:
   - If the sub-schema is used only within this parent schema, keep it inline (or use `$defs`).
   - If the sub-schema matches a type used elsewhere, extract it to its own file.

#### OpenAPI `components/schemas` cleanup

After extracting all schemas from `components/schemas` to external files:
- Remove the `components/schemas` block from the OpenAPI file.
- Update all internal `$ref: "#/components/schemas/Foo"` pointers to `$ref: "../schemas/foo.yaml"`.
- If `components` is now empty, remove the `components` block entirely.

### Step 5: Inject Specify Metadata

For every schema file in `$CONTRACTS_DIR/schemas/` (both newly decomposed and pre-existing), check for required Specify metadata and add what is missing.

| Field | Rule | Generation |
|---|---|---|
| `$schema` | Must be `"https://json-schema.org/draft/2020-12/schema"` | Add if absent. Update if pointing to an older draft. |
| `$id` | Must be `"urn:specify:schemas/<filename-without-extension>"` | Generate from the file path. Do not overwrite existing `$id` values that match a baseline schema. |
| `title` | PascalCase type name | Derive from filename: `user-registration.yaml` → `UserRegistration`. Do not overwrite existing `title` values. |
| `description` | Non-empty string | If absent, set to `"[imported — description pending review]"`. Do not overwrite existing descriptions. |

For OpenAPI files, verify that `info.title`, `info.version`, and `info.description` are present. If `info.description` is missing, set it to `"[imported — description pending review]"`.

For AsyncAPI files, apply the same `info` field checks.

### Step 6: Place Files in the Correct Directories

Ensure all output files are in the correct subdirectories under `$CONTRACTS_DIR/`:

| File Type | Target Directory | Trigger |
|---|---|---|
| JSON Schema files | `$CONTRACTS_DIR/schemas/` | Standalone schemas, or decomposed from bindings |
| OpenAPI files | `$CONTRACTS_DIR/http/` | Files with `openapi:` top-level key |
| AsyncAPI files | `$CONTRACTS_DIR/messages/` | Files with `asyncapi:` top-level key |

If the operator placed files in the wrong subdirectory (e.g. an OpenAPI file directly in `$CONTRACTS_DIR/` rather than in `$CONTRACTS_DIR/http/`), move it to the correct location. Create subdirectories only when they will contain at least one file.

### Step 7: Validate and Report

Run `/contracts:validator` against the output to verify internal consistency.

Produce the import report (see §*Import Report* below).

---

## Import Report

After completing the 7-step algorithm, produce an import report. This is the primary output alongside the normalized contract files.

### Report Format

```markdown
## Import Report

### Files Processed
- **Total input files:** N
- **Swagger 2.0 → OpenAPI 3.1:** N files upgraded
- **OpenAPI 3.0 → 3.1:** N files upgraded
- **AsyncAPI 2.x → 3.0:** N files upgraded
- **Already at target version:** N files
- **Standalone JSON Schema:** N files placed
- **Unrecognized (skipped):** N files

### Inline Schema Decomposition
- **Schemas extracted:** N
- **Baseline duplicates avoided:** N (matched existing baseline schemas)
<!-- One entry per extracted schema -->
- `components/schemas/User` → `contracts/schemas/user.yaml`
- `paths./users.post.requestBody` → `contracts/schemas/user-create-request.yaml`

### Metadata Injected
<!-- One entry per file that received metadata additions -->
- `contracts/schemas/user.yaml` — added `$id`, `$schema`
- `contracts/http/user-api.yaml` — added `info.description`

### Validation Result
<!-- Output from /contracts:validator -->
All checks passed (N $ref pointers, N schemas, N bindings verified).

### Manual Review Required
<!-- Files or constructs that could not be automatically processed -->
- `unknown-format.yaml` — unrecognized format, no `openapi`/`asyncapi`/`swagger`/`$schema` key found
- `legacy-api.yaml` — `x-custom-auth` extension preserved but not validated
```

### Report semantics

- **A clean import with zero manual review items is the ideal outcome.** All files were detected, upgraded, decomposed, and metadata-injected automatically.
- **Manual review items are expected for complex imports.** Unusual format extensions, vendor-specific constructs, and files that cannot be classified should surface as review items rather than being silently dropped.
- **The validation result at the end confirms internal consistency.** If the validator reports issues, re-enter the decomposition and metadata steps for targeted repair before finalizing the report.

---

## Edge Cases

### Mixed input formats

When `$CONTRACTS_DIR/` contains a mix of Swagger 2.0, OpenAPI 3.0, and standalone JSON Schema files (common when importing from a legacy system with multiple API versions), process each file independently. The importer handles heterogeneous input — every file is classified and processed on its own.

### OpenAPI file with `$ref` to a sibling file

When an imported OpenAPI file references another file via `$ref` (e.g. `$ref: "./common-types.yaml"`), and that sibling file is also in `$CONTRACTS_DIR/`:

1. Process the referenced file first (detect format, upgrade, decompose).
2. Update the `$ref` in the referencing file to point to the new location after decomposition (e.g. `$ref: "../schemas/common-type.yaml"`).

### Swagger 2.0 files with external `$ref`

Swagger 2.0 files may use `$ref` to external URLs or paths outside the change directory. These cannot be automatically resolved. Flag each unresolvable `$ref` in the import report for manual review. Do not silently remove the reference.

### Name collisions during decomposition

When two inline schemas from different files would produce the same filename:
- If the schemas are structurally equivalent, extract once and `$ref` from both locations.
- If the schemas differ, disambiguate by prefixing with the source API domain: `user-api-error.yaml` vs `billing-api-error.yaml`.

### Empty `components/schemas`

When an OpenAPI file has a `components/schemas` block but all entries are already `$ref` pointers to external files, do not create duplicate schema files. Update the `$ref` pointers to use the `../schemas/` path convention and remove the `components/schemas` block.

### AsyncAPI 2.x with inline message payloads

AsyncAPI 2.x files often define message payloads inline under `channels/<name>/subscribe/message/payload` or `channels/<name>/publish/message/payload`. During the combined upgrade + decomposition:
1. Upgrade the channel/operation structure to 3.0 format first.
2. Then decompose inline payloads to `$CONTRACTS_DIR/schemas/`.
3. Wire the decomposed schemas through `components/messages` with `$ref` to `../schemas/`.

### JSON files (not YAML)

When input files use `.json` extension instead of `.yaml`:
1. Read the JSON content.
2. Convert to YAML format.
3. Write with a `.yaml` extension.
4. Process normally through the remaining steps.
5. Note the format conversion in the import report.

### Vendor extensions (`x-` prefixed keys)

Preserve all vendor extensions (`x-*` keys) during version upgrades. They may carry information meaningful to the operator's toolchain. Note their presence in the import report but do not attempt to validate or transform them.

---

## Output Hygiene

- Only emit `.yaml` files under `$CONTRACTS_DIR/`.
- Create subdirectories (`schemas/`, `http/`, `messages/`) only when they contain at least one file.
- Do not create empty directories.
- Do not modify any file outside `$CONTRACTS_DIR/`.
- Do not modify baseline files in `.specify/contracts/`.
- Remove input files that were relocated (e.g. an OpenAPI file moved from `$CONTRACTS_DIR/` to `$CONTRACTS_DIR/http/`).

## Verification Checklist

Before completing the import:

- [ ] Every input file classified (format detected or flagged for review)
- [ ] All Swagger 2.0 files upgraded to OpenAPI 3.1
- [ ] All OpenAPI 3.0.x files upgraded to OpenAPI 3.1
- [ ] All AsyncAPI 2.x files upgraded to AsyncAPI 3.0
- [ ] All inline schemas decomposed to `$CONTRACTS_DIR/schemas/`
- [ ] All `$ref` pointers updated to use `../schemas/` convention
- [ ] All schema files have `$id`, `$schema`, `title`, `description`
- [ ] All files in correct subdirectories (`schemas/`, `http/`, `messages/`)
- [ ] `/contracts:validator` passes on the result
- [ ] Import report produced with per-file results and manual review items
- [ ] No baseline files modified
