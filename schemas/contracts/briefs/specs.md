---
id: specs
description: Define interface-level behavioral requirements for the contract
generates: specs/**/*.md
needs: [proposal]
---

## Baseline Contract Awareness

When root `contracts/` exists and contains files, read its contents as **read-only context** before writing specs. This ensures behavioral requirements conform to existing interface shapes rather than inventing new ones:

- **Endpoint conformance:** When baseline contracts define HTTP endpoints (in `contracts/http/`), write spec scenarios that reference the existing endpoint paths, HTTP methods, and status codes. Do not invent new endpoint paths when the baseline already defines one for the same interaction.
- **Payload conformance:** When baseline contracts define JSON Schema types (in `contracts/schemas/`), write spec scenarios whose data references are consistent with the existing field names, types, and required/optional status. Do not describe payload fields that contradict the schema.
- **Message conformance:** When baseline contracts define messaging channels (in `contracts/messages/`), write spec scenarios that reference the existing channel names and message structures.
- **Error conformance:** When baseline contracts define error responses, write error condition sections that are consistent with the contract's error types and status codes.

This is a **context hint, not a hard constraint**. When the change requires interactions not covered by the baseline contracts, write the spec scenarios naturally — the build brief downstream will generate or import the corresponding contract artifacts. The goal is consistency with existing contracts, not restriction to them.

When root `contracts/` does not exist, this section has no effect — proceed with spec authoring as normal.

When the plan entry has a `context` field containing `contracts/` paths, read only those specific contract files as conformance context rather than scanning the entire root `contracts/` directory.

---

Create one spec file per interface listed in the proposal's Interface Scope section. These are **interface-level** behavioral specs — they describe what the interface does, not what any implementation does internally.

Every contract change needs at least a lightweight spec file. Build consumes Specify artifacts as its primary source of truth; it should not read raw design documentation directly except when the proposal names source files that must be imported into the change-local `contracts/` tree.

## Authorship modes

### Generate from prose

When the proposal's Authorship Mode is **Generate from prose**, convert the source material into requirements that are detailed enough for the interface skills to author contract artifacts:

- **HTTP/resource interactions**: endpoint path, HTTP method, path/query/header parameters, request body name and fields, response body name and fields, status codes, and auth headers when they affect the wire contract.
- **Evented interactions**: topic or channel address, producer/consumer direction, event trigger, message name, payload name and fields, headers, correlation IDs, partition keys, and idempotency keys when present.
- **Shared payload vocabulary**: named types, fields, required/optional status, types, formats, enums, constraints, and reusable nested shapes.

Mark unavailable structural details with `[unknown]` rather than guessing. Build will surface unresolved unknowns in the relevant format skill's alignment report.

### Import existing contracts

When the proposal's Authorship Mode is **Import existing contracts**, write a lightweight behavioral spec that states the imported contract's observable purpose and scope. Treat the imported files listed in the proposal's Source Material section as the structural source of truth.

Do not redundantly restate every imported field. Capture enough behavior for reviewers and downstream consumers to understand what interface is being added or normalized, then let the build brief route the supplied files through the relevant `/contract:*` importer and verifier intents.

### Modify existing contracts

When the proposal's Authorship Mode is **Modify existing contracts**, describe only the intended behavioral delta from the current baseline. Use the existing spec folder name from `.specify/specs/<interface>/` when creating the delta spec at `specs/<interface>/spec.md`. Follow the delta structure (ADDED / MODIFIED / RENAMED / REMOVED sections) documented in the define skill's spec format conventions.

---

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

Focus on observable interface behavior: request/response shapes, status codes, message payloads, error conditions. Do not describe internal implementation logic, handler structure, or provider dependencies.
