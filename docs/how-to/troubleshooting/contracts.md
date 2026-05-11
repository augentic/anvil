# Contract issues

Use this page when a `/contract:*` skill (OpenAPI, AsyncAPI, JSON Schema) reports a verifier failure or alignment warning.

## Prerequisites

- A slice that has reached the contract phase (specs are defined; bindings or schemas exist under the slice's `contracts/` directory).
- The verifier output naming the offending file or schema.

## `$ref` resolution failures

**Symptom:** A format verifier (`/contract:openapi`, `/contract:asyncapi`, or `/contract:json-schema` running its verifier intent) reports that a `$ref` pointer does not resolve.

**Cause:** A schema file referenced from an OpenAPI or AsyncAPI binding does not exist in either the slice's `contracts/schemas/` or the baseline `contracts/schemas/`.

**Resolution:**
1. Check the `$ref` path in the binding file.
2. Verify the referenced schema file exists and the filename matches (kebab-case, `.yaml` extension).
3. If the schema is new, ensure the corresponding `/contract:*` skill's author intent generated it (typically `/contract:json-schema` for shared payloads). If it is a baseline schema, ensure the baseline is up to date.

## Schema metadata incomplete

**Symptom:** `/contract:json-schema` (verifier intent) reports missing `$id`, `title`, or `description` on a JSON Schema file.

**Cause:** The schema file was created without the required Specify metadata, or an imported external schema was not fully normalised.

**Resolution:**
1. Add the missing fields. `$id` must be `urn:specify:schemas/<filename-without-extension>`.
2. For imported schemas, re-run the relevant `/contract:*` skill's importer intent or add the metadata manually.

## Binding completeness failures

**Symptom:** A format verifier (`/contract:openapi` or `/contract:asyncapi` running its verifier intent) reports that a schema has no protocol binding.

**Cause:** A schema that appears as a top-level request/response body or message payload in a spec scenario has no corresponding OpenAPI path or AsyncAPI channel.

**Resolution:**
1. If the schema is a shared vocabulary type (e.g. `ErrorResponse`) used only via `$ref` from other schemas, it is exempt from this check -- verify the verifier is not misclassifying it.
2. If the schema should have a binding, ensure the relevant `/contract:*` skill's author intent (`/contract:openapi` for HTTP / resource APIs, `/contract:asyncapi` for evented / pub-sub / streaming) produced the corresponding binding file.

## Alignment warnings

**Symptom:** An `/contract:*` skill's author intent reports alignment warnings in the alignment report.

**Cause:** The slice's specs describe interactions that partially conflict with the baseline contracts -- e.g. a response schema missing a field that a spec scenario asserts, or a spec referencing a status code the baseline binding does not define.

**Resolution:**
1. Review each warning. The writer does not auto-resolve spec-vs-baseline conflicts.
2. If the spec is correct, update the baseline contract in a dedicated contract slice.
3. If the baseline is correct, update the spec to conform.
