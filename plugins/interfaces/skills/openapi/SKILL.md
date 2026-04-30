---
name: interfaces-openapi
description: Authors, imports, and verifies OpenAPI 3.1 HTTP API contracts for Specify changes, including path operations, request and response schemas, parameters, auth, examples, and baseline deltas. Use when the contracts brief needs an HTTP API contract, when an operator supplies or asks for an OpenAPI document, or when verifying OpenAPI compatibility after a merge.
argument-hint: "[change-dir]"
---

# OpenAPI

Specialist for OpenAPI 3.1 HTTP API contracts on Specify changes. This skill owns three intents — authoring or extending the OpenAPI document for a change, importing or normalising an externally supplied OpenAPI document, and verifying an OpenAPI artefact (single-mode internal consistency or cross-project consumer compatibility).

The skill is OpenAPI-only. Shared payload schemas under `contracts/schemas/` are owned by the json-schema format skill (`/interfaces:json-schema`); evented contracts under `contracts/messages/` are owned by `/interfaces:asyncapi`.

## Critical Path (Quick Reference)

1. **Read the briefs and specs.** Open the active contracts brief and the change's `specs/` to identify what the change requires; read `.specify/contracts/http/` (the HTTP baseline) to know what already exists.
2. **Identify the intent.** Map the trigger to one of three sibling files using the [Intent dispatch](#intent-dispatch) table — author, importer, or verifier. Stop reading SKILL.md once the sibling is selected; load only the relevant sibling.
3. **Dispatch to the sibling.** Open and follow [`author.md`](./author.md), [`importer.md`](./importer.md), or [`verifier.md`](./verifier.md). Each sibling owns its complete algorithm, decision rules, and output format.
4. **Write outputs to `contracts/http/`.** Author and importer paths produce or normalise OpenAPI 3.1 YAML files under `$CHANGE_DIR/contracts/http/`. Decomposed payload schemas land under `$CHANGE_DIR/contracts/schemas/` (json-schema-skill territory) — never inline them.
5. **Run the verifier.** After authoring or importing, invoke the verifier sibling against the change directory to check `$ref` resolution, schema metadata completeness, and binding coverage.
6. **Surface diagnostics.** Render the markdown alignment / import / validation report (single mode) or the structured YAML compatibility report (cross-project mode) so the calling brief or operator can triage.
7. **Stay within `contracts/http/`.** Do not modify baseline files in `.specify/contracts/`, do not touch `contracts/messages/` or shared schemas beyond writing decomposed `$ref` targets, and do not invent constructs that the spec does not justify — mark unknowns with `[unknown]` instead.

## Invocation

```text
/interfaces:openapi <change-dir>
```

Optional internal flags (recognised by the verifier sibling):

- `--mode single` — default. Validate the change's OpenAPI artefacts in isolation against the specs and baseline. Read-only, markdown report.
- `--mode cross-project` — invoked by `/spec:execute` after a producer's contract change merges. Compares the merged OpenAPI document against each consumer's tier-2 workspace clone. Read-only, structured YAML report. See `verifier.md` §Cross-project mode.

When invoked from the `contracts` brief during `/spec:define`, `<change-dir>` is the active change directory; the brief routes the intent (author or importer) based on whether the operator dropped an external document into `contracts/http/`. When invoked post-merge by `/spec:execute`, the verifier sibling runs in `cross-project` mode against the producer's merged contract.

## Artifact layout

OpenAPI files live in two locations — the change-local delta and the platform baseline:

```text
.specify/
├── contracts/
│   └── http/
│       └── <api-domain>.yaml          # Baseline: merged contracts only
└── changes/<change-name>/
    └── contracts/
        ├── http/
        │   └── <api-domain>.yaml      # Change-local delta or normalised import
        └── schemas/
            └── <type>.yaml            # Owned by /interfaces:json-schema
```

Conventions enforced for every OpenAPI file in either location:

- **OpenAPI 3.1.0** — never 3.0.x. Importer upgrades 3.0 and Swagger 2.0 inputs. See [`openapi-conventions`](../../references/openapi-conventions.md).
- **Kebab-case `.yaml` filename** — named after the API domain (`user-api.yaml`, `billing-api.yaml`). One file may carry many related operations.
- **`$ref` to `../schemas/`** — every request body, response body, and parameter schema points at a standalone JSON Schema file. Inline schemas are forbidden in the baseline; the importer decomposes inline schemas into `contracts/schemas/` before the file enters the baseline.
- **Opaque file replacement** — the change-level `contracts/http/<domain>.yaml` replaces the baseline file wholesale. When extending an existing API domain, the delta file must contain both the existing operations and the new ones (the writer's algorithm reads the baseline and merges).

For the broader directory layout, see [`artifact-structure`](../../references/artifact-structure.md); for the cross-format minimal-delta rules and merge semantics, see [`baseline-vs-delta`](../../references/baseline-vs-delta.md).

## Intent dispatch

Pick the sibling that matches the trigger. Each sibling is a self-contained algorithm — load only the one selected.

| Intent | Trigger | Sibling |
|---|---|---|
| Author or extend the OpenAPI document from a spec | contracts brief during `/spec:define`; operator extending the baseline for new HTTP interactions | `author.md` |
| Import or normalise an external OpenAPI document | operator drops an OpenAPI file into a change's `contracts/http/` directory | `importer.md` |
| Verify internal consistency or run the cross-project consumer check | contracts brief post-merge (RFC-9 §3B); operator invoking validation against an existing OpenAPI artefact | `verifier.md` |

The three intents share a common artefact contract (paths, file naming, `$ref` discipline) but have distinct algorithms — never conflate them. An import must be followed by a verifier run before the brief considers the artefact ready for merge; an author run normally ends with a verifier run too.

## Sibling references

- [`author.md`](./author.md) — spec → operations mapping, schema reuse, baseline-delta computation, examples, security schemes.
- [`importer.md`](./importer.md) — format detection (Swagger 2.0 / OpenAPI 3.0.x / OpenAPI 3.1.x), upgrade rules to 3.1, inline-schema decomposition, Specify metadata injection.
- [`verifier.md`](./verifier.md) — `$ref` resolution, schema metadata completeness, binding coverage, single-mode and cross-project verifier modes.

## Shared format guidance

OpenAPI-specific conventions live in [`openapi-conventions`](../../references/openapi-conventions.md) — file structure, path/method rules, response codes, `operationId` patterns, content-type defaults, and the contract scope boundary (what stays in `design.md`, not in the contract). Read it before authoring or normalising operations; the sibling files link back to it where relevant.

For the cross-format directory layout, baseline-vs-delta rules, and merge semantics, see [`artifact-structure`](../../references/artifact-structure.md).

## Cross-format coordination

When a change touches more than one interface format (HTTP + events + shared schemas), the `contracts` brief invokes the format skills in this order:

1. `/interfaces:json-schema` first — the schema vocabulary is shared and must stabilise before the bindings reference it.
2. `/interfaces:openapi` — HTTP operations bind to the schemas authored above.
3. `/interfaces:asyncapi` — message channels bind to the same schemas.

This skill never writes files outside `contracts/http/` (and the change-local schema deltas it decomposes into `contracts/schemas/` during import). Channel-shaped intents are out of scope — route them to `/interfaces:asyncapi`.

## Hard rules

These constraints are non-negotiable for any of the three sibling paths:

1. **Valid OpenAPI 3.1.** Every output file must parse as OpenAPI 3.1.0. The importer is the only entry point that accepts older inputs.
2. **`$ref` discipline.** All schema references use relative file paths into `../schemas/`. No `#/components/schemas/...` pointers in the baseline. No inline domain types.
3. **`$id` stability.** Once a schema has a `$id`, do not change it. New schemas get new `$id` values; the writer and importer never reassign existing ones.
4. **Kebab-case filenames.** All `.yaml` files use kebab-case names; no PascalCase or snake_case variants.
5. **Baseline immutability.** Never modify files in `.specify/contracts/`. All output goes in the change-local `contracts/` directory.
6. **No invention.** When the spec does not provide enough detail to derive a shape, mark the gap with `[unknown]` in the alignment report rather than guessing. Importer flags unrecognised constructs with `[import — manual review required]`.
7. **Read-only verifier.** The verifier sibling must not create, modify, or delete any files in either mode.

## Output hygiene

- Only emit `.yaml` files under `$CHANGE_DIR/contracts/`.
- Create `contracts/http/`, `contracts/schemas/` only when they will contain at least one file.
- Do not modify any file outside `$CHANGE_DIR/contracts/`.
- Do not modify baseline files in `.specify/contracts/`.

## See also

- [`openapi-conventions`](../../references/openapi-conventions.md) — OpenAPI 3.1 structure, path/method conventions, `$ref → ../schemas/`.
- [`artifact-structure`](../../references/artifact-structure.md) — directory layout for `.specify/contracts/`.
- [`baseline-vs-delta`](../../references/baseline-vs-delta.md) — cross-format rules for computing the minimal delta between baseline and change-local files.
- [`import-upgrade-policy`](../../references/import-upgrade-policy.md) — shared framework for the importer sibling (format detection, upgrade targets, lossless vs lossy decisions).
- [`report-shape`](../../references/report-shape.md) — single-mode markdown and cross-project YAML report formats produced by the verifier sibling.
- [`cross-project-compatibility`](../../references/cross-project-compatibility.md) — `change-kind` vocabulary used by the verifier in `--mode cross-project`.
- [`json-schema-conventions`](../../references/json-schema-conventions.md) — payload schema rules (owned by `/interfaces:json-schema`; linked here so authors of OpenAPI files understand the schema files they reference).
