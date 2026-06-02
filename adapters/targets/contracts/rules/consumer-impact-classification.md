---
id: IFACE-005
title: Consumer Impact Classification
severity: important
trigger: A contract verifier, reviewer, or alignment report describes compatibility risk without mapping the delta to the shared change-kind vocabulary and a consumer-facing action.
---

## Rule

Classify contract deltas with the shared consumer-impact vocabulary so operators can triage compatibility risk consistently across OpenAPI, AsyncAPI, and JSON Schema. Cross-project reports should use the RM-11 classifications `additive`, `breaking`, `ambiguous`, and `unverifiable`. Breaking deltas should map to a known `change-kind` such as `removed-field`, `required-field-added`, `type-narrowed`, `enum-value-removed`, `additional-properties-tightened`, `removed-endpoint`, `status-code-removed`, `removed-channel`, or `removed-operation`. Safe additive deltas should not be reported as warnings or failures.

Each finding should identify the producer project, consumer project, producer contract, consumer view or baseline being compared, affected locator, classification, optional `change-kind`, and expected operator action. Do not invent one-off categories when a known `change-kind` applies, and do not hide ambiguous or unverifiable compatibility risk behind a generic note.

## Look For

- Findings that say "breaking", "changed", or "incompatible" without a stable `change-kind`.
- Custom category names that duplicate the shared vocabulary.
- Warnings for safe additive changes such as optional fields, new endpoints, new channels, wider enum values, or documentation-only edits.
- Missing producer contract path, consumer baseline path, schema locator, operation id, channel address, or message locator.
- Cross-project checks that inspect consumer source code, mutate workspace clones, or transition plan state as part of RM-11 reporting.
- Ambiguous or unverifiable deltas silently dropped because they do not fit the deterministic classification table.

## Spec Guidance

When a spec intentionally asks for an incompatible interface change, require the proposal or contract alignment report to name the expected classification, `change-kind` categories where applicable, and the consumer migration action before implementation proceeds.
