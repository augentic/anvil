# contracts.build — openapi sub-flow

Loaded by [../build.md](../build.md) Phase 2 when the slice carries HTTP / resource interactions. Specialist for OpenAPI 3.1 HTTP API contracts.

This sub-flow is OpenAPI-only. Shared payload schemas under `contracts/schemas/` are owned by the [json-schema sub-flow](json-schema.md); evented contracts under `contracts/messages/` are owned by the [asyncapi sub-flow](asyncapi.md).

## Critical path

1. **Read the briefs and specs.** Open [../build.md](../build.md) and the slice's `specs/<domain>/spec.md` files to identify what HTTP interactions the slice requires; read `contracts/http/` (the HTTP baseline) to know what already exists.
2. **Identify the intent.** Map the trigger to one of three sibling references using the [intent dispatch](#intent-dispatch) table — author, importer, or verifier. Stop reading sub-flow prose once the sibling is selected; load only the relevant sibling.
3. **Dispatch to the sibling.** Open and follow [`../../references/openapi/author.md`](../../references/openapi/author.md), [`../../references/openapi/importer.md`](../../references/openapi/importer.md), or [`../../references/openapi/verifier.md`](../../references/openapi/verifier.md). Each sibling owns its complete algorithm, decision rules, and output format.
4. **Write outputs to `contracts/http/`.** Author and importer paths produce or normalise OpenAPI 3.1 YAML files under `$SLICE_DIR/contracts/http/`. Decomposed payload schemas land under `$SLICE_DIR/contracts/schemas/` (json-schema-sub-flow territory) — never inline them.
5. **Run the verifier.** After authoring or importing, invoke the verifier sibling against the slice directory to check `$ref` resolution, schema metadata completeness, and binding coverage.
6. **Surface diagnostics.** Render the markdown alignment / import / validation report (single mode) or the contract-tool JSON envelope (cross-project mode). Cross-project consumer impact is deferred until a real consumer workflow exists.
7. **Stay within change-local `contracts/http/`.** Do not modify baseline files in root `contracts/`, do not touch `contracts/messages/` or shared schemas beyond writing decomposed `$ref` targets, and do not invent constructs that the spec does not justify — mark unknowns with `[unknown]` instead.

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
        └── <type>.yaml                # Owned by the json-schema sub-flow
```

Conventions enforced for every OpenAPI file in either location:

- **OpenAPI 3.1.0** — never 3.0.x. The importer upgrades 3.0 and Swagger 2.0 inputs. See [`../../references/openapi-conventions.md`](../../references/openapi-conventions.md).
- **Kebab-case `.yaml` filename** — named after the API domain (`user-api.yaml`, `billing-api.yaml`). One file may carry many related operations.
- **`$ref` to `../schemas/`** — every request body, response body, and parameter schema points at a standalone JSON Schema file. Inline schemas are forbidden in the baseline; the importer decomposes inline schemas into `contracts/schemas/` before the file enters the baseline.
- **Opaque file replacement** — the slice-level `contracts/http/<domain>.yaml` replaces the baseline file wholesale. When extending an existing API domain, the delta file must contain both the existing operations and the new ones (the writer's algorithm reads the baseline and merges).

For the broader directory layout, see [`../../references/artifact-structure.md`](../../references/artifact-structure.md); for the cross-format minimal-delta rules and merge semantics, see [`../../references/baseline-vs-delta.md`](../../references/baseline-vs-delta.md).

## Intent dispatch

Pick the sibling that matches the trigger. Each sibling is a self-contained algorithm — load only the one selected.

| Intent | Trigger | Sibling |
|---|---|---|
| Author or extend the OpenAPI document from a spec | build brief during `/spec:build`; operator extending the baseline for new HTTP interactions | [`../../references/openapi/author.md`](../../references/openapi/author.md) |
| Import or normalise an external OpenAPI document | operator drops an OpenAPI file into a slice's `contracts/http/` directory | [`../../references/openapi/importer.md`](../../references/openapi/importer.md) |
| Verify internal consistency or run merge-time baseline validation | build verification; post-merge contract baseline gate; operator invoking validation against an existing OpenAPI artefact | [`../../references/openapi/verifier.md`](../../references/openapi/verifier.md) |

The three intents share a common artefact contract (paths, file naming, `$ref` discipline) but have distinct algorithms — never conflate them. An import must be followed by a verifier run before the brief considers the artefact ready for merge; an author run normally ends with a verifier run too.

## Hard rules

These constraints are non-negotiable for any of the three sibling paths:

1. **Valid OpenAPI 3.1.** Every output file must parse as OpenAPI 3.1.0. The importer is the only entry point that accepts older inputs.
2. **`$ref` discipline.** All schema references use relative file paths into `../schemas/`. No `#/components/schemas/...` pointers in the baseline. No inline domain types.
3. **`$id` stability.** Once a schema has a `$id`, do not change it. New schemas get new `$id` values; the writer and importer never reassign existing ones.
4. **Kebab-case filenames.** All `.yaml` files use kebab-case names; no PascalCase or snake_case variants.
5. **Baseline immutability.** All output goes in the slice-local `contracts/` directory; baseline `contracts/` is read-only here.
6. **No invention.** When the spec does not provide enough detail to derive a shape, mark the gap with `[unknown]` in the alignment report rather than guessing. The importer flags unrecognised constructs with `[import — manual review required]`.
7. **Read-only verifier.** The verifier sibling must not create, modify, or delete any files in either mode.
