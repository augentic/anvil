# Import / Upgrade Policy

Cross-format framework for normalising externally-supplied contracts onto Specify conventions. Each contracts build format sub-flow (`openapi`, `asyncapi`, `json-schema`) implements its own importer; this reference captures the rules they share — format detection, upgrade targets, lossless-vs-lossy decisions, and when to refuse and ask the operator.

## The four legs of every importer

Every format-specific importer runs through four conceptual phases. The format skills implement each phase with format-specific algorithms; the framework below is the shared contract.

| Phase | Question answered | Where it runs |
|---|---|---|
| **1. Detect** | What format is this file? Is it in scope for this skill? | First step of every importer; routes out-of-scope files to the right format skill. |
| **2. Upgrade** | Does the file use an older version that the skill must convert? | Format-specific (Swagger 2.0 → OpenAPI 3.1, AsyncAPI 2.x → 3.0, JSON Schema draft-04/6/7/2019-09 → 2020-12). |
| **3. Decompose** | Does the file inline content that Specify keeps as separate files? | OpenAPI / AsyncAPI extract inline schemas into `contracts/schemas/`; JSON Schema extracts multi-type bundles. |
| **4. Inject** | Does the file lack Specify-required metadata? | Final step before validation; adds `$id`, `$schema`, `title`, `description`, `info.*` defaults. |

The output of every importer is a set of files in the canonical Specify layout (see [`artifact-structure`](artifact-structure.md)) that pass the format-skill verifier.

## Format detection

Detection runs against the top-level YAML/JSON keys of each input file. The first match wins; key order matters because some files carry multiple signals (e.g. an OpenAPI bundle with embedded `$schema`).

| Priority | Signal | Classification | Owner |
|---|---|---|---|
| 1 | `swagger:` (any value) | Swagger 2.0 | `openapi` sub-flow |
| 2 | `openapi: "3.0.x"` | OpenAPI 3.0.x | `openapi` sub-flow |
| 3 | `openapi: "3.1.x"` | OpenAPI 3.1.x | `openapi` sub-flow |
| 4 | `asyncapi: "2.x"` | AsyncAPI 2.x | `asyncapi` sub-flow |
| 5 | `asyncapi: "3.0.x"` | AsyncAPI 3.0.x | `asyncapi` sub-flow |
| 6 | `$schema:` (no protocol key) | Standalone JSON Schema | `json-schema` sub-flow |
| 7 | `$id:` / `properties:` / `definitions:` / `$defs:` (no protocol key, no `$schema:`) | Probable JSON Schema (draft unknown) | `json-schema` sub-flow |
| 8 | None of the above | Unrecognised | Skip; flag for manual review |

Detection rules for every importer:

- **Detection is case-sensitive.** Do not normalise key casing before classification.
- **Protocol keys win over schema signals.** A file with both `openapi:` and `$schema:` is an OpenAPI bundle, not a standalone schema. The schema-skill importer routes such files out to the protocol importer.
- **Both YAML and JSON inputs are accepted.** JSON files are converted to YAML during normalisation (Specify uses `.yaml` exclusively in the baseline).
- **Unrecognised inputs are skipped, never guessed.** The importer flags them in the report and the operator decides.

## Upgrade targets

Specify pins each format to a single canonical version. Importers convert older inputs to the canonical version; never accept older versions in the output.

| Format | Older versions accepted as input | Canonical output version |
|---|---|---|
| OpenAPI | Swagger 2.0, OpenAPI 3.0.x | OpenAPI 3.1.0 |
| AsyncAPI | AsyncAPI 2.x | AsyncAPI 3.0.0 |
| JSON Schema | Draft 4, 6, 7, 2019-09 | Draft 2020-12 |

Format-specific upgrade mechanics (top-level field renames, structural rearrangements, type-system reconciliation) live in the format skills' `importer.md` files. The shared rules below apply to every upgrade path.

## Lossless vs lossy upgrades

Every importer's contract is **no data loss**. Every endpoint, channel, schema, property, security scheme, and operation in the input must be present in the output. Information may be **restructured**; it must not be **silently dropped**.

When a construct cannot be cleanly mapped to the target version:

| Situation | Action |
|---|---|
| Direct structural mapping exists (e.g. Swagger `definitions` → OpenAPI `components/schemas`) | Apply mechanically. Lossless. |
| Semantic mapping exists with a known idiomatic conversion (e.g. OpenAPI 3.0 `nullable: true` → 3.1 `type: ["string", "null"]`) | Apply mechanically. Lossless. |
| Construct exists in the source but not in the target (e.g. JSON Schema Draft 4 `id` keyword) | Rewrite to the target's equivalent (`$id`). Lossless. |
| Construct has no equivalent and the source semantics cannot be expressed | Preserve the construct verbatim **and** flag it as `[import — manual review required]` in the report. **Do not silently drop.** |
| Construct has multiple plausible mappings and the source does not disambiguate | Refuse to choose. Flag `[import — manual review required]` and let the operator pick. |
| Vendor extension (`x-*` keys) | Preserve verbatim. Note presence in the report; never validate or transform vendor extensions. |

The general principle: **preserve semantics, allow restructuring, never invent.** When the importer cannot make a mechanical decision, it surfaces the gap rather than guessing.

## When to refuse and ask the operator

Some inputs cannot be normalised by any importer running in isolation. The importer should refuse to produce output and surface the issue prominently. Common cases:

| Trigger | Example | Importer action |
|---|---|---|
| External `$ref` to a URL or absolute path outside the slice directory | `$ref: "https://example.com/schemas/user"` | Cannot auto-resolve. Flag in report. The operator must inline-import the external schema as a separate file or accept the dangling reference. |
| `$id` collision with a baseline schema where shapes differ | Imported `user.yaml` has `$id: "urn:specify:schemas/user"` matching the baseline `user.yaml`, but the property sets diverge | Stop and emit `[import — $id collision; resolve manually]`. The `$id` stability rule forbids automatic reassignment. |
| File with multiple YAML documents (`---` separators) | A single `.yaml` carrying two unrelated OpenAPI specs | Process the first document; flag the rest in the report. Do not silently merge. |
| File whose top-level format cannot be classified | Custom IDL, malformed YAML, BOM corruption | Skip; flag for manual review. Do not attempt to guess the format. |
| Multi-file bundle distributed across nested directories | An OpenAPI document with `$ref` chains spanning a vendor bundle | Process the entry-point file; chase `$ref`s only to siblings the operator placed in the slice directory. Flag every unresolved external `$ref`. |
| Construct that implies behaviour beyond wire shape (e.g. `x-rate-limit` extensions, custom auth flows) | Vendor extensions that look semantically meaningful | Preserve verbatim; flag as "preserved but not validated" in the report. The operator decides whether to lift the construct into `design.md`. |
| Conflict between source-declared `$id` / `title` and the filename Specify would assign | Source `title: "User Adapter"`, source filename `acct.yaml` | Prefer `title` (and rewrite the filename); flag as a normalisation entry. When neither resolves cleanly, request operator input. |

The importer never **silently** decides any of the above. The report's "Manual Review Required" section is the canonical surface for these decisions; the verifier confirms the importer did not paper over them.

## Specify metadata injection

Every output file must carry the Specify-required metadata fields, regardless of whether the source provided them. The format skills enumerate the per-format field set; the policy below is shared:

| Field | Rule | Source-vs-derived behaviour |
|---|---|---|
| `$schema` (schemas) | Pin to `https://json-schema.org/draft/2020-12/schema`. | Inject if absent; upgrade if older draft. |
| `$id` (schemas) | URN form `urn:specify:schemas/<filename-without-extension>`. | Generate from filename. **Never reassign** an `$id` that matches an existing baseline schema (see [`baseline-vs-delta`](baseline-vs-delta.md) — the `$id` stability rule). |
| `title` (schemas) | PascalCase type name from filename. | Derive from filename. Do not overwrite existing `title` values. |
| `description` (schemas, `info.description` on OpenAPI / AsyncAPI) | Non-empty string. | If absent, set to `"[imported — description pending review]"` and surface in the import report so the operator replaces it before merge. |
| `info.title` / `info.version` (OpenAPI / AsyncAPI) | Required by the format spec. `info.version` MUST parse as SemVer per [semver.org](https://semver.org) (contract identity/version validation). | Preserve from source verbatim; surface in the report if absent. **Do not auto-rewrite a non-SemVer `info.version`** (e.g. `2024-01-15`) — emit a `[manual review required]` entry naming the file and the offending value. The single-mode verifier (Check 4) and the merge-time `specify tool run contract` gate will block on the unaltered value until the operator resolves it. |
| `info.x-specify-id` (OpenAPI / AsyncAPI) | Optional rename-stable id (contract identity/version validation). When present, kebab-case (`^[a-z][a-z0-9-]*$`), ≤ 64 characters, unique across every top-level contract under root `contracts/`. | Preserve from source verbatim — even when malformed (the verifier flags the format issue with the file path, which is enough for the operator to fix). **Never invent or auto-derive an id during import** — new ids are an authoring decision. |

The `[imported — description pending review]` placeholder is reserved for the importer path. It appears in the verifier's output as a `WARN` to ensure the gap reaches a human before the slice merges.

## General upgrade principles

Every format-specific importer follows these principles:

1. **Preserve semantics.** The upgraded file must describe the same API as the source. Structural changes are allowed; behavioural changes are not.
2. **Preserve vendor extensions.** All `x-*` keys carry through unchanged.
3. **Preserve ordering where possible.** Maintain the original key ordering for readability and reviewability. The merge step uses opaque file replacement (see [`baseline-vs-delta`](baseline-vs-delta.md)), so re-ordered output looks like a wholesale rewrite.
4. **Preserve comments where possible.** YAML comments may be lost during parse / re-serialise — that is acceptable but should be noted in the import report when the source has significant comments.
5. **Handle unknowns conservatively.** When a construct has no clear mapping, preserve it as-is and flag it. Never guess.
6. **Defer to the verifier.** Every importer ends with a verifier run. If the verifier reports issues, re-enter Step 3 (decompose) or Step 4 (inject) for targeted repair before producing the final report.

## See also

- [`artifact-structure`](artifact-structure.md) — directory layout for the post-import baseline shape.
- [`baseline-vs-delta`](baseline-vs-delta.md) — `$id` stability and baseline-immutability rules that the importer obeys.
- [`report-shape`](report-shape.md) — import report structure, including the "Manual Review Required" section.
- Format-specific importers — `adapters/targets/contracts/references/{openapi,asyncapi,json-schema}/importer.md`.
