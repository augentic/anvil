---
id: contracts
description: Validate spec alignment with baseline contracts; generate delta for uncovered interactions
generates: contracts/**/*.yaml
needs: [specs]
---

Arguments (used by all skills):
- CHANGE_ID: the name of this change (from specify status)

## Algorithm

### Phase 1: Author

Classify the change's spec interactions into HTTP/resource interactions, evented/pub-sub/streaming interactions, and reusable payload schemas. When the spec contains both HTTP and evented interactions, run `/interfaces:json-schema` first to author shared payload schemas, then `/interfaces:openapi`, then `/interfaces:asyncapi`. Each format skill reads baseline contracts at `.specify/contracts/` and the change's specs under `specs/`, validates alignment between specs and baseline, and authors the minimal delta for uncovered interactions in its format.

1. /interfaces:json-schema — author the minimal JSON Schema delta for reusable payload vocabulary referenced by HTTP and/or evented interactions. Owns `$id` assignment, one-type-per-file decomposition, and schema-file naming for shared payloads. Skip when the change has no shared payload schemas.
2. /interfaces:openapi — author the minimal OpenAPI delta for HTTP/resource interactions. Reuse change-local or baseline `contracts/schemas/` files; do not author competing schemas under different filenames or `$id`s. Skip when the change has no HTTP interactions.
3. /interfaces:asyncapi — author the minimal AsyncAPI delta for evented, pub/sub, streaming, or WebSocket-style interactions. Follows the same schema-reuse rule as `/interfaces:openapi`. Skip when the change has no evented interactions.

### Phase 2: Verify

After all author passes complete, run the verifier path of each format skill that produced or extended artifacts:

4. /interfaces:json-schema — verifier intent: `$ref` resolution and schema metadata for the change-local schema files.
5. /interfaces:openapi — verifier intent: `$ref` resolution, schema metadata, and binding completeness for the OpenAPI delta.
6. /interfaces:asyncapi — verifier intent: `$ref` resolution, schema metadata, and binding completeness for the AsyncAPI delta.

For mixed-format changes, the final verifier pass must check cross-format `$ref` consistency and report duplicate schema identities before the brief can complete.

### Phase 3: Verify-repair loop (max 2 iterations)

If a verifier reports failures:
1. Re-enter the same format skill (`/interfaces:openapi`, `/interfaces:asyncapi`, or `/interfaces:json-schema`) with the verifier output, instructing it to make targeted repairs to the flagged issues only via its author intent.
2. Re-run the format's verifier intent to check the repairs.
3. If still failing after 2 repair iterations, stop and surface the remaining issues for human review. Do not mark the brief as complete.

A clean verification pass with zero issues is the expected outcome. When pre-existing baseline contracts cover the change's spec interactions, each format skill produces a small or empty delta and the verifiers confirm consistency — the brief completes quickly.

### No-op behavior

When the change's specs describe no API interactions (no HTTP endpoints, no message exchanges, no data types that would warrant contract artifacts), every format pass produces an empty delta and the verifiers have nothing to check. The brief completes as a no-op. This is normal for changes that modify internal logic without affecting API surfaces.
