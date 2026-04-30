# Generation Fixture — Spec-First Pattern

Demonstrates the `/interfaces:json-schema` and `/interfaces:openapi` author and verifier behavior when no baseline contracts exist (the spec-first fallback pattern for single-repo services). The HTTP/resource specs in this fixture make `/interfaces:openapi` the primary format skill; `/interfaces:json-schema` runs first to author the shared payload schemas referenced by the OpenAPI delta.

## Scenario

A user registration API with three endpoints. The baseline at `.specify/contracts/` is empty. The writer reads the spec and generates the full contract set as the delta.

## Inputs

- `specs/user-registration/spec.md` — behavioral spec describing the user registration API
- Baseline: empty (no `.specify/contracts/` directory)

## Expected Outputs

- `expected/contracts/schemas/user-registration.yaml` — JSON Schema for the registration payload
- `expected/contracts/schemas/user.yaml` — JSON Schema for the user entity
- `expected/contracts/schemas/error-response.yaml` — JSON Schema for error responses
- `expected/contracts/http/user-api.yaml` — OpenAPI 3.1 binding with three endpoints

## Expected Verifier Output

Clean pass — all `$ref` pointers resolve, all schemas have metadata, all spec-referenced schemas have bindings.
