# JSON Schema — Importer

> **When to read this.** Read this when an operator supplies external JSON Schema files and the contracts adapter build brief needs them normalised onto Specify conventions under a slice's `contracts/schemas/` directory. Skip this file when authoring from a spec (use [`author.md`](./author.md)) or when verifying an existing artefact (use [`verifier.md`](./verifier.md)). When the supplied file is an OpenAPI or AsyncAPI bundle (with inline schemas), this importer routes the file out — only standalone schema documents stay here.

## Inputs

```text
$SLICE_DIR     = .specify/slices/<slice-name>
$CONTRACTS_DIR  = $SLICE_DIR/contracts
$SCHEMAS_DIR    = $CONTRACTS_DIR/schemas
$BASELINE_DIR   = contracts
```

**Input** — external schema files placed by the operator anywhere under `$CONTRACTS_DIR/`. Files may be `.yaml`, `.yml`, or `.json`. Inputs may carry any JSON Schema draft (4, 6, 7, 2019-09, 2020-12) and may use `definitions` or `$defs` for sub-types.

**Output** — normalised JSON Schema Draft 2020-12 files in `$SCHEMAS_DIR/`, with one type per file, kebab-case filenames, URN `$id` values, and Specify metadata injected. Input files are replaced in-place with their normalised equivalents; multi-type bundles are decomposed into multiple files.

## Authority hierarchy

When sources conflict:

1. **This file** — import rules and hard constraints for JSON Schema.
2. **Format conventions** — [`../../references/json-schema-conventions.md`](../../references/json-schema-conventions.md).
3. **Source contract** — the external schema file being imported. Preserve every type, property, and constraint it defines; never silently drop information.
4. **LLM inference** — prohibited for unknowns; mark unrecognised constructs with `[import — manual review required]` and surface in the import report.

## The 5-step import algorithm

The importer runs five sequential steps. Each step assumes its predecessors completed cleanly; if a step cannot make progress on a file, mark the file as `manual review required` and continue with the rest.

### Step 1 — Scan and detect

Walk `$CONTRACTS_DIR/` for `.yaml`, `.yml`, and `.json` files. For each file, read the top-level keys and classify:

| Detection signal | Classification | Target |
|---|---|---|
| `swagger:` (any value) | **Out of scope.** Route to the `openapi` importer sub-flow. |
| `openapi:` (any value) | **Out of scope.** Route to the `openapi` importer sub-flow. |
| `asyncapi:` (any value) | **Out of scope.** Route to the `asyncapi` importer sub-flow. |
| `$schema:` (Draft URI), no protocol key | Standalone JSON Schema | Normalise to Draft 2020-12 |
| `$id:` present, no `$schema:`, no protocol key | Probable JSON Schema | Normalise; inject `$schema` |
| `type:` / `properties:` / `definitions:` / `$defs:` at root, no protocol key, no `$schema:` | Probable JSON Schema (draft unknown) | Normalise; inject `$schema` and surface as `[import — draft unknown]` warning |
| None of the above | Unrecognised | Skip and flag for manual review |

A file with both `$schema:` and a protocol key (`openapi`, `asyncapi`, `swagger`) is **not** a standalone schema — it is a bundle. Route it to the appropriate protocol importer; that importer's decomposition step will produce schema files this skill subsequently normalises.

JSON files (`.json`) are converted to YAML in this step: read the JSON, re-serialise as YAML with a `.yaml` extension, then continue normalisation.

### Step 2 — Decompose multi-type bundles

A standalone schema file may carry multiple sibling types in a `definitions` or `$defs` block, plus a top-level type. Specify's one-type-per-file rule requires decomposition.

#### What counts as a bundle

| Source shape | Treatment |
|---|---|
| Top-level `type: object` plus a `definitions` block (Draft 4 / 6 / 7 style) | Top-level schema becomes one file; each `definitions/<Name>` becomes a sibling file |
| Top-level schema plus `$defs` block (Draft 2019-09 / 2020-12 style) where any `$defs` entry is referenced from outside the parent file | Same: extract each cross-file-referenced `$defs` entry to its own file |
| Top-level schema plus `$defs` block whose entries are referenced **only** internally | Keep the `$defs` block; do not decompose. File-local sub-types are valid. |
| File whose root is purely a `definitions` / `$defs` map with no top-level type | Each entry becomes its own file; the original wrapper file is deleted |

#### Decomposition process

For each extracted entry:

1. **Determine the filename.** Use the kebab-case form of the entry's key (the `definitions/<Name>` map key, or the `title` field if present). When `title` and key disagree, prefer `title`; when neither resolves cleanly, surface the file in the report as `[import — manual filename review required]`.
2. **Check for baseline conflicts.** If a schema with the same filename already exists in `$BASELINE_DIR/schemas/`:
   - Structurally equivalent (same `properties`, `required`, types) — drop the extracted file; rewrite incoming `$ref` pointers to target the baseline file.
   - Differs structurally — disambiguate by prefixing with the source bundle's domain (`user-billing.yaml` vs `user.yaml`).
3. **Write the file** to `$SCHEMAS_DIR/<name>.yaml`.
4. **Rewrite `$ref` pointers** in every file that referenced the extracted entry: `#/definitions/<Name>` and `#/$defs/<Name>` both become `<name>.yaml` (relative path within `schemas/`).

After decomposition, each output file holds exactly one named, top-level type. Sub-types referenced from outside their parent are now standalone files; sub-types referenced only internally remain in `$defs`.

### Step 3 — Upgrade drafts to 2020-12

For each output file, normalise to JSON Schema Draft 2020-12. The conversion is mechanical; never invent fields the source does not provide.

#### `$schema` value mapping

| Source `$schema` value | Target |
|---|---|
| `http://json-schema.org/draft-04/schema#` | `https://json-schema.org/draft/2020-12/schema` |
| `http://json-schema.org/draft-06/schema#` | `https://json-schema.org/draft/2020-12/schema` |
| `http://json-schema.org/draft-07/schema#` | `https://json-schema.org/draft/2020-12/schema` |
| `https://json-schema.org/draft/2019-09/schema` | `https://json-schema.org/draft/2020-12/schema` |
| `https://json-schema.org/draft/2020-12/schema` | unchanged |
| `$schema:` absent | `https://json-schema.org/draft/2020-12/schema` (inject) |

#### Draft 4 specific changes

- `id` → `$id`. The keyword renamed in Draft 6.
- `definitions` → `$defs`. The block renamed in Draft 2019-09. Update all internal `$ref: "#/definitions/<Name>"` pointers to `$ref: "#/$defs/<Name>"` (or to standalone files if Step 2 extracted them).
- `exclusiveMinimum: true` + `minimum: N` → `exclusiveMinimum: N` (single numeric). Same for `exclusiveMaximum`.
- `id` values that were absolute URLs or fragment URIs become URN form per Step 4.

#### Draft 6 / 7 specific changes

- `definitions` → `$defs` (same as Draft 4).
- `exclusiveMinimum` / `exclusiveMaximum` are already numeric in Draft 6+; no change needed.
- `const`, `if`, `then`, `else`, `propertyNames` are valid in Draft 7 and 2020-12; carry over unchanged.

#### Draft 2019-09 specific changes

- `$schema` URI bump to 2020-12.
- `unevaluatedProperties` and `unevaluatedItems` are already valid in 2020-12; carry over.

#### Vocabulary considerations

- Preserve `enum`, `pattern`, `format`, `minLength`, `maxLength`, `minimum`, `maximum`, `multipleOf`, `minItems`, `maxItems`, `uniqueItems`, `minProperties`, `maxProperties`.
- Preserve `if` / `then` / `else`, `allOf`, `anyOf`, `oneOf`, `not`.
- Preserve all `x-*` vendor extensions verbatim. Note their presence in the import report; never validate or transform them.
- Preserve `examples` (plural array). When the source uses `example` (singular), convert to `examples: [<value>]`.

#### Nullable handling

Imported schemas occasionally carry OpenAPI 3.0-style `nullable: true`. Convert to JSON Schema type unions:

```yaml
# Source (legacy OpenAPI-flavoured)
properties:
  nickname:
    type: string
    nullable: true

# Output (Draft 2020-12)
properties:
  nickname:
    type:
      - string
      - "null"
```

Remove `nullable` after conversion.

### Step 4 — Inject Specify metadata

For every schema file in `$SCHEMAS_DIR/`, inject Specify-required metadata where missing. Never overwrite values the source already provided unless they are demonstrably malformed (and even then, only with the operator's review).

| Field | Rule | Generation |
|---|---|---|
| `$schema` | `"https://json-schema.org/draft/2020-12/schema"` | Add if absent or upgrade per Step 3. |
| `$id` | `"urn:specify:schemas/<filename-without-extension>"` | If the source `$id` is already a URN of the form `urn:specify:schemas/<segment>` and the segment matches the filename, keep it. Otherwise rewrite to the canonical URN; surface the rewrite in the report as a normalisation entry. **Never reassign an `$id` that matches an existing baseline schema.** |
| `title` | PascalCase type name | Derive from filename: `user-registration.yaml` → `UserRegistration`. Do not overwrite existing `title`. |
| `description` | Non-empty string | If absent, set to `"[imported — description pending review]"` and surface in the import report. |
| `type` | the JSON Schema type | Required for non-trivial schemas. If the source is missing `type` but has `properties`, infer `type: object`. Surface inferred values in the report. |
| `additionalProperties` | (object schemas) | If absent, leave absent — do **not** inject a default. Surface in the report so the operator can confirm whether `additionalProperties: false` is appropriate. The author path defaults to `false`; the importer is conservative because flipping `additionalProperties` is a backwards-incompatible change. |

#### `$id` handling — special cases

| Source `$id` shape | Action |
|---|---|
| `urn:specify:schemas/<matches-filename>` | Keep verbatim |
| `urn:specify:schemas/<does-not-match-filename>` | Rewrite to match filename; surface as a `Metadata Injected` finding |
| Some other URN form (e.g. `urn:example:user`) | Rewrite to `urn:specify:` form; preserve the original in the report for traceability |
| HTTPS URL (e.g. `https://example.com/schemas/user`) | Rewrite to URN form; preserve the original in the report |
| Absent | Generate from the filename |
| Matches a baseline `$id` exactly **and** the schemas are structurally equivalent | Drop the imported file; rewrite incoming `$ref` to the baseline file |
| Matches a baseline `$id` but the schemas differ | Stop and emit a `[import — $id collision; resolve manually]` finding. Do not reassign either `$id` automatically. |

### Step 5 — Place files, validate, report

#### Place files

Move each schema file to `$SCHEMAS_DIR/`. Remove the original file when the operator placed it elsewhere under `$CONTRACTS_DIR/`. Create the `schemas/` subdirectory only when it will contain at least one file.

#### Validate

Run [`verifier.md`](./verifier.md) in `single` mode against `$SLICE_DIR` to confirm `$ref` resolution, metadata completeness, duplicate-`$id` checks, and cross-format compatibility. If the verifier reports issues, re-enter Steps 2–4 for targeted repair before producing the report.

#### Report

Produce a markdown import report:

```markdown
## Import Report (Schemas)

### Files Processed
- **Total input files:** N
- **Draft 4 → Draft 2020-12:** N upgraded
- **Draft 6 / 7 → Draft 2020-12:** N upgraded
- **Draft 2019-09 → Draft 2020-12:** N upgraded
- **Already Draft 2020-12:** N
- **JSON-only inputs converted to YAML:** N
- **Multi-type bundles decomposed:** N (extracted M sibling types)
- **Routed out (OpenAPI / AsyncAPI bundles):** N
- **Unrecognised (skipped):** N

### Decomposition
- `bundle.yaml` → extracted `user.yaml`, `address.yaml`, `phone-number.yaml`
- `bundle.yaml` deleted (root was a pure `definitions` map)

### Metadata Injected
- `contracts/schemas/user.yaml` — added `$id`, `$schema`
- `contracts/schemas/error-response.yaml` — placeholder `description: "[imported — description pending review]"` (replace before merge)

### `$id` Rewrites
- `user.yaml` — was `https://example.com/schemas/user`; now `urn:specify:schemas/user`

### Validation Result
All checks passed (N $ref pointers, N schemas verified).

### Manual Review Required
- `unknown-format.yaml` — no `$schema`, `$id`, `type`, or `properties`; cannot classify
- `user-billing.yaml` — `$id` collides with baseline `user.yaml` but shape differs; resolve manually
- `legacy.yaml` — `x-internal-tier` extension preserved but not validated
```

Report semantics:

- **Zero manual review items** is the ideal outcome — every file detected, decomposed, upgraded, and metadata-injected automatically.
- **Manual review items are expected for complex imports.** Vendor-specific constructs, `$id` collisions, and ambiguous classifications surface here rather than being silently resolved.
- **The validation result confirms internal consistency.** If the verifier reports issues, fix and re-run — do not finalise the report until the verifier passes.

## Edge cases

| Scenario | Handling |
|---|---|
| File contains a top-level array (`type: array`) with a `definitions` block | Treat the array as the file's main type; extract `definitions` entries normally. |
| File uses `$ref` to an external URL (`https://example.com/schemas/foo`) | Cannot auto-resolve. Flag in the report; never silently drop. Operator must either inline-import the external schema as a separate file or accept the dangling reference (verifier will fail). |
| Two imported files declare the same `$id` | Stop and surface as `[import — $id collision]`. The operator decides which to keep, rename, or merge. |
| Source schema has `additionalProperties: true` with no spec justification | Preserve verbatim; flag in the report. The author rule of "default to `false`" applies only to authored schemas, not imported ones. |
| Source uses `nullable: true` (OpenAPI-flavoured) | Convert to type union per Step 3 §Nullable handling. |
| Source uses Draft 4 `id` (no `$`) | Rename to `$id` per Step 3 §Draft 4 specific changes. |
| Source uses `definitions` block referenced internally only | Convert to `$defs`; keep file-local. |
| Source uses `definitions` block referenced from sibling files via external `$ref` | Decompose all entries to standalone files (Step 2). |
| Source has multiple YAML documents (`---` separators) | Process the first document; flag the rest in the report. |
| Source uses `$comment` field | Preserve verbatim; it is informational only. |
| `format` value is non-standard (`format: emoji`) | Preserve; `format` annotations are advisory in JSON Schema. Note in the report. |

## Hard rules

1. **No data loss.** Every property, constraint, and sub-type in the source must be present in the output. Information may be restructured but not silently dropped.
2. **Valid Draft 2020-12.** Every output file must parse against `https://json-schema.org/draft/2020-12/schema`.
3. **One type per file** after decomposition.
4. **Kebab-case `.yaml` filenames** for every output file.
5. **`$ref` resolution.** Every `$ref` in the output must resolve to a file in `$SCHEMAS_DIR/`, `$BASELINE_DIR/schemas/`, or to a sibling key inside the same file (`#/$defs/<name>`).
6. **`$id` stability.** Never reassign a baseline `$id` value. Surface collisions for human review.
7. **Baseline preservation.** Never modify any file in root `contracts/`.
8. **Route protocol bundles out.** OpenAPI / AsyncAPI / Swagger files are not normalised here — pass them to the appropriate protocol importer.

## Verification checklist

Before completing the import:

- [ ] Every input file classified — JSON Schema, protocol bundle (routed out), or unrecognised.
- [ ] All Draft 4 / 6 / 7 / 2019-09 files upgraded to Draft 2020-12.
- [ ] All multi-type bundles decomposed; one type per output file.
- [ ] All `$ref` pointers updated to point at standalone files (or `#/$defs/` for file-local sub-types).
- [ ] All schema files have `$schema`, `$id`, `title`, `description`, `type`.
- [ ] All `$id` values are URN form rooted at `urn:specify:schemas/`.
- [ ] No `$id` reassignment touches a baseline schema.
- [ ] Files placed in `$SCHEMAS_DIR/`; original locations cleaned up.
- [ ] [`verifier.md`](./verifier.md) (single mode) ran clean.
- [ ] Import report produced with per-file results, decomposition entries, `$id` rewrites, and manual-review items.
- [ ] No baseline files modified.

## See also

- [`../../references/json-schema-conventions.md`](../../references/json-schema-conventions.md) — target Draft 2020-12 conventions.
- [`../../references/artifact-structure.md`](../../references/artifact-structure.md) — directory layout for the post-import baseline shape.
- [`../../references/import-upgrade-policy.md`](../../references/import-upgrade-policy.md) — cross-format framework for format detection, upgrade targets, lossless-vs-lossy decisions, and "when to refuse and ask the operator" cases (including `$id` collisions and external `$ref`s).
- [`../../references/baseline-vs-delta.md`](../../references/baseline-vs-delta.md) — `$id` stability, baseline immutability, and the contract-given authorship pattern this importer realises.
- [`../../references/report-shape.md`](../../references/report-shape.md) — markdown shape for the import report produced at the end of Step 5.
- [`author.md`](./author.md) — sibling for spec-driven authoring.
- [`verifier.md`](./verifier.md) — sibling for validating imported output.
