---
id: specs
description: Define interface-level behavioral requirements for the contract
generates: specs/**/*.md
needs: [proposal]
---

## Baseline Contract Awareness

When `.specify/contracts/` exists and contains files, read its contents as **read-only context** before writing specs. This ensures behavioral requirements conform to existing interface shapes rather than inventing new ones:

- **Endpoint conformance:** When baseline contracts define HTTP endpoints (in `.specify/contracts/http/`), write spec scenarios that reference the existing endpoint paths, HTTP methods, and status codes. Do not invent new endpoint paths when the baseline already defines one for the same interaction.
- **Payload conformance:** When baseline contracts define JSON Schema types (in `.specify/contracts/schemas/`), write spec scenarios whose data references are consistent with the existing field names, types, and required/optional status. Do not describe payload fields that contradict the schema.
- **Message conformance:** When baseline contracts define messaging channels (in `.specify/contracts/messages/`), write spec scenarios that reference the existing channel names and message structures.
- **Error conformance:** When baseline contracts define error responses, write error condition sections that are consistent with the contract's error types and status codes.

This is a **context hint, not a hard constraint**. When the change requires interactions not covered by the baseline contracts, write the spec scenarios naturally — the `contracts` brief downstream will generate the corresponding contract artifacts. The goal is consistency with existing contracts, not restriction to them.

When `.specify/contracts/` does not exist, this section has no effect — proceed with spec authoring as normal.

When the plan entry has a `context` field containing `contracts/` paths, read only those specific contract files as conformance context rather than scanning the entire `.specify/contracts/` directory.

---

Create one spec file per interface listed in the proposal's Interface Scope section. These are **interface-level** behavioral specs — they describe what the interface does, not what any implementation does internally.

Use the exact interface name from the proposal (`specs/<interface>/spec.md`). Follow this structure:

```markdown
# <Interface Name> Specification

## Purpose

<1-2 sentences describing the interface>

### Requirement: <Endpoint or Channel Behavior>

ID: REQ-001

The <interface> SHALL <behavioral description>.

#### Scenario: <Happy Path>

- **WHEN** <request or message>
- **THEN** <response or outcome>

#### Scenario: <Error Case>

- **WHEN** <invalid request or failure condition>
- **THEN** <error response>

## Error Conditions

- <error type>: <description and trigger conditions>
```

Repeat `### Requirement:` blocks for each distinct behavior, incrementing `ID: REQ-XXX` for each new requirement.

**Modified Contracts**: When the proposal's Authorship Pattern is Modification, use the existing spec folder name from `.specify/specs/<interface>/` when creating the delta spec at `specs/<interface>/spec.md`. Follow the delta structure (ADDED / MODIFIED / RENAMED / REMOVED sections) documented in the define skill's spec format conventions.

Focus on observable interface behavior: request/response shapes, status codes, message payloads, error conditions. Do not describe internal implementation logic, handler structure, or provider dependencies.
