---
id: build
description: Validate contract artifacts for structural correctness
needs: [specs, contracts, tasks]
tracks: tasks
---

The build phase for contract-only changes validates structural correctness of the contract artifacts. There is no code generation — the sole output is a validation pass/fail.

Arguments:
- CHANGE_ID: the name of this change (from specify status)

## Verification

Verification runs the verifier intent of each `/interfaces:*` format skill that owns artifacts in the change. Run only the formats that produced artifacts; skip the rest.

1. `/interfaces:json-schema` — verifier intent: `$ref` resolution and schema metadata (`$id`, `title`, `description`) for every JSON Schema file under `contracts/schemas/`.
2. `/interfaces:openapi` — verifier intent: `$ref` resolution across the OpenAPI delta and binding completeness (every spec-referenced HTTP schema has at least one binding).
3. `/interfaces:asyncapi` — verifier intent: `$ref` resolution across the AsyncAPI delta and binding completeness (every spec-referenced evented schema has at least one binding).

For mixed-format changes, the final verifier pass must check cross-format `$ref` consistency and report duplicate schema identities before the build can complete.

When this brief runs as the post-merge cross-project consumer check (RFC-9 §3B), thread `--mode cross-project` through the format verifier paths so they emit the cross-project compatibility report instead of the single-change report.

## Verify-repair loop (max 2 iterations)

If a verifier reports failures:
1. Re-enter the same format skill (`/interfaces:openapi`, `/interfaces:asyncapi`, or `/interfaces:json-schema`) with the verifier output for targeted repair via its author intent.
2. Re-run that format's verifier intent.
3. If still failing after 2 iterations, stop and surface issues for human review. Do not mark the task complete. Report the remaining failures with full output and escalate for guidance.

A clean verification pass with zero issues is the expected outcome.
