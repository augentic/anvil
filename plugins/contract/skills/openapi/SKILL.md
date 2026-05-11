---
name: contract-openapi
description: Authors, imports, and verifies OpenAPI 3.1 HTTP API contracts for Specify changes, including path operations, request and response schemas, parameters, auth, examples, and baseline deltas. Use when a contracts build needs an HTTP API contract, when an operator supplies or asks for an OpenAPI document, or when verifying OpenAPI compatibility after a merge.
argument-hint: "[slice-dir]"
---

# OpenAPI

Specialist for OpenAPI 3.1 HTTP API contracts on Specify changes. This skill owns three intents — authoring or extending the OpenAPI document for a slice, importing or normalising an externally supplied OpenAPI document, and verifying an OpenAPI artefact (single-mode internal consistency or merge-time baseline validation).

The skill is OpenAPI-only. Shared payload schemas under `contracts/schemas/` are owned by the json-schema format skill (`/contract:json-schema`); evented contracts under `contracts/messages/` are owned by `/contract:asyncapi`.

## Critical Path (Quick Reference)

1. **Read the briefs and specs.** Open the active contracts build brief and the slice's `specs/` to identify what the slice requires; read `contracts/http/` (the HTTP baseline) to know what already exists.
2. **Identify the intent.** Map the trigger to one of three sibling files using the [Intent dispatch](#intent-dispatch) table — author, importer, or verifier. Stop reading SKILL.md once the sibling is selected; load only the relevant sibling.
3. **Dispatch to the sibling.** Open and follow [`author.md`](./author.md), [`importer.md`](./importer.md), or [`verifier.md`](./verifier.md). Each sibling owns its complete algorithm, decision rules, and output format.
4. **Write outputs to `contracts/http/`.** Author and importer paths produce or normalise OpenAPI 3.1 YAML files under `$SLICE_DIR/contracts/http/`. Decomposed payload schemas land under `$SLICE_DIR/contracts/schemas/` (json-schema-skill territory) — never inline them.
5. **Run the verifier.** After authoring or importing, invoke the verifier sibling against the slice directory to check `$ref` resolution, schema metadata completeness, and binding coverage.
6. **Surface diagnostics.** Render the markdown alignment / import / validation report (single mode) or the contract-tool JSON envelope (cross-project mode) so the calling brief or operator can triage. Cross-project consumer impact is reported by `specify compatibility`.
7. **Stay within change-local `contracts/http/`.** Do not modify baseline files in root `contracts/`, do not touch `contracts/messages/` or shared schemas beyond writing decomposed `$ref` targets, and do not invent constructs that the spec does not justify — mark unknowns with `[unknown]` instead.

## Invocation

```text
/contract:openapi <slice-dir>
```

Optional internal positionals (recognised by the verifier sibling):

- `mode single` — default. Validate the slice's OpenAPI artefacts in isolation against the specs and baseline. Read-only, markdown report.
- `mode cross-project` — merge-time baseline validation delegate. Walks the merged `contracts/` directory through `specify tool run contract`; it does not compare consumer workspace clones. Read-only, JSON envelope. See `verifier.md` §Cross-project mode.

When invoked from the contracts capability build brief during `/spec:build`, `<slice-dir>` is the active slice directory; the brief routes the intent (author or importer) based on whether the operator supplied an external document for `contracts/http/`. Consumer-impact reporting is a CLI concern under `specify compatibility`.

## Artifact layout

OpenAPI files live in two locations — the slice-local delta and the platform baseline:

```text
contracts/
└── http/
    └── <api-domain>.yaml              # Baseline: merged contracts only

.specify/slices/<slice-name>/
└── contracts/
    ├── http/
    │   └── <api-domain>.yaml          # Slice-local delta or normalised import
    └── schemas/
        └── <type>.yaml                # Owned by /contract:json-schema
```

Conventions enforced for every OpenAPI file in either location:

- **OpenAPI 3.1.0** — never 3.0.x. Importer upgrades 3.0 and Swagger 2.0 inputs. See [`openapi-conventions`](../../references/openapi-conventions.md).
- **Kebab-case `.yaml` filename** — named after the API domain (`user-api.yaml`, `billing-api.yaml`). One file may carry many related operations.
- **`$ref` to `../schemas/`** — every request body, response body, and parameter schema points at a standalone JSON Schema file. Inline schemas are forbidden in the baseline; the importer decomposes inline schemas into `contracts/schemas/` before the file enters the baseline.
- **Opaque file replacement** — the slice-level `contracts/http/<domain>.yaml` replaces the baseline file wholesale. When extending an existing API domain, the delta file must contain both the existing operations and the new ones (the writer's algorithm reads the baseline and merges).

For the broader directory layout, see [`artifact-structure`](../../references/artifact-structure.md); for the cross-format minimal-delta rules and merge semantics, see [`baseline-vs-delta`](../../references/baseline-vs-delta.md).

## Intent dispatch

Pick the sibling that matches the trigger. Each sibling is a self-contained algorithm — load only the one selected.

| Intent | Trigger | Sibling |
|---|---|---|
| Author or extend the OpenAPI document from a spec | contracts capability build brief during `/spec:build`; operator extending the baseline for new HTTP interactions | `author.md` |
| Import or normalise an external OpenAPI document | operator drops an OpenAPI file into a slice's `contracts/http/` directory | `importer.md` |
| Verify internal consistency or run merge-time baseline validation | contracts capability build verification; post-merge contract baseline gate; operator invoking validation against an existing OpenAPI artefact | `verifier.md` |

The three intents share a common artefact contract (paths, file naming, `$ref` discipline) but have distinct algorithms — never conflate them. An import must be followed by a verifier run before the brief considers the artefact ready for merge; an author run normally ends with a verifier run too.

## Sibling references

- [`author.md`](./author.md) — spec → operations mapping, schema reuse, baseline-delta computation, examples, security schemes.
- [`importer.md`](./importer.md) — format detection (Swagger 2.0 / OpenAPI 3.0.x / OpenAPI 3.1.x), upgrade rules to 3.1, inline-schema decomposition, Specify metadata injection.
- [`verifier.md`](./verifier.md) — `$ref` resolution, schema metadata completeness, binding coverage, single-mode verifier behavior, and cross-project baseline validation delegation.

## Shared format guidance

OpenAPI-specific conventions live in [`openapi-conventions`](../../references/openapi-conventions.md) — file structure, path/method rules, response codes, `operationId` patterns, content-type defaults, and the contract scope boundary (what stays in `design.md`, not in the contract). Read it before authoring or normalising operations; the sibling files link back to it where relevant.

For the cross-format directory layout, baseline-vs-delta rules, and merge semantics, see [`artifact-structure`](../../references/artifact-structure.md).

## Cross-format coordination

When a slice touches more than one contract format (HTTP + events + shared schemas), the contracts capability build brief invokes the format skills in this order:

1. `/contract:json-schema` first — the schema vocabulary is shared and must stabilise before the bindings reference it.
2. `/contract:openapi` — HTTP operations bind to the schemas authored above.
3. `/contract:asyncapi` — message channels bind to the same schemas.

This skill never writes files outside `contracts/http/` (and the slice-local schema deltas it decomposes into `contracts/schemas/` during import). Channel-shaped intents are out of scope — route them to `/contract:asyncapi`.

## Hard rules

These constraints are non-negotiable for any of the three sibling paths:

1. **Valid OpenAPI 3.1.** Every output file must parse as OpenAPI 3.1.0. The importer is the only entry point that accepts older inputs.
2. **`$ref` discipline.** All schema references use relative file paths into `../schemas/`. No `#/components/schemas/...` pointers in the baseline. No inline domain types.
3. **`$id` stability.** Once a schema has a `$id`, do not change it. New schemas get new `$id` values; the writer and importer never reassign existing ones.
4. **Kebab-case filenames.** All `.yaml` files use kebab-case names; no PascalCase or snake_case variants.
5. **Baseline immutability.** Never modify files in root `contracts/`. All output goes in the slice-local `contracts/` directory.
6. **No invention.** When the spec does not provide enough detail to derive a shape, mark the gap with `[unknown]` in the alignment report rather than guessing. Importer flags unrecognised constructs with `[import — manual review required]`.
7. **Read-only verifier.** The verifier sibling must not create, modify, or delete any files in either mode.

## Output hygiene

- Only emit `.yaml` files under `$SLICE_DIR/contracts/`.
- Create `contracts/http/`, `contracts/schemas/` only when they will contain at least one file.
- Do not modify any file outside `$SLICE_DIR/contracts/`.
- Do not modify baseline files in root `contracts/`.

## See also

- [`openapi-conventions`](../../references/openapi-conventions.md) — OpenAPI 3.1 structure, path/method conventions, `$ref → ../schemas/`.
- [`artifact-structure`](../../references/artifact-structure.md) — directory layout for root `contracts/`.
- [`baseline-vs-delta`](../../references/baseline-vs-delta.md) — cross-format rules for computing the minimal delta between baseline and change-local files.
- [`import-upgrade-policy`](../../references/import-upgrade-policy.md) — shared framework for the importer sibling (format detection, upgrade targets, lossless vs lossy decisions).
- [`report-shape`](../../references/report-shape.md) — single-mode markdown, baseline validator JSON, and compatibility report JSON formats.
- [`cross-project-compatibility`](../../references/cross-project-compatibility.md) — RM-04 compatibility classifications and `change-kind` vocabulary used by `specify compatibility`.
- [`json-schema-conventions`](../../references/json-schema-conventions.md) — payload schema rules (owned by `/contract:json-schema`; linked here so authors of OpenAPI files understand the schema files they reference).
