---
name: validator
description: "Validate internal consistency of API contract artifacts — $ref resolution, schema metadata completeness, and binding coverage."
license: MIT
allowed-tools: Read, Grep, Glob
---

# Contracts Validator

Validate consistency of contract artifacts. The validator does not generate or modify contract files — it reports issues so the caller can act on them.

The validator runs in two modes:

| Mode | Caller | Trigger | Scope |
|---|---|---|---|
| `single` (default) | `contracts` brief in the define pipeline | Post-`/contracts:writer` | One change's `contracts/` directory inside one project |
| `cross-project` | `/spec:execute` post-merge step (RFC-9 §3B) | Producer-side merge of a contract change | Compare a producer's updated contract against a consumer's workspace clone |

`single` mode is read-only and feeds the brief's verify-repair loop. `cross-project` mode is read-only and emits warnings the execute driver records in the merging change's `journal.yaml`. Both modes share the read-only contract; neither writes to disk.

## Scope Rules

**The validator is read-only.** It MUST NOT generate, modify, or delete any files. Its sole output is a list of issues rendered as a validation report.

In `single` mode the report is markdown for human review by the contracts brief's verify-repair loop. In `cross-project` mode the report is a structured YAML document the execute driver consumes (see [Cross-Project Mode](#cross-project-mode-rfc-9-3b)).

## Arguments

The validator accepts a `--mode {single, cross-project}` flag (default `single`) and one of two argument shapes depending on the chosen mode.

### `single` mode (default)

Inferred from the active change context — no positional arguments required:

```text
$CHANGE_DIR      = .specify/changes/<change-name>
$CHANGE_CONTRACTS = $CHANGE_DIR/contracts/
$BASELINE_CONTRACTS = .specify/contracts/
$CHANGE_SPECS    = $CHANGE_DIR/specs/
```

### `cross-project` mode

Caller passes the producer's updated contract path and the consumer's workspace clone path:

```text
$PRODUCER_CONTRACT  = <path-to-producer-contract>     # e.g. .specify/contracts/http/user-api.yaml
$CONSUMER_WORKSPACE = <path-to-consumer-workspace>    # e.g. .specify/workspace/mobile/
$CONSUMER_CONTRACTS = $CONSUMER_WORKSPACE/.specify/contracts/
```

`$PRODUCER_CONTRACT` is a path relative to the initiating repo root (typically a file under `.specify/contracts/` after the producer change's merge). `$CONSUMER_WORKSPACE` is a tier-2 workspace clone (see [Workspace Tiers](../../../../docs/explanation/workspace-tiers.md)) — `specify workspace sync` materialises consumer clones at `.specify/workspace/<consumer-name>/`, and the consumer's view of central contracts lives at `$CONSUMER_CONTRACTS`.

## Prerequisites

### `single` mode

- The `/contracts:writer` has completed and produced artifacts under `$CHANGE_CONTRACTS`.
- `.specify/project.yaml` exists (Specify is initialized).

If `$CHANGE_CONTRACTS` does not exist or contains no files, report all checks as passed — there is nothing to validate.

### `cross-project` mode

- `$PRODUCER_CONTRACT` exists and is readable (otherwise exit non-zero with a `cannot-read-producer-contract` diagnostic — see [Edge Cases](#edge-cases)).
- `$CONSUMER_WORKSPACE` exists. If `$CONSUMER_CONTRACTS` is absent (the consumer has not received any contracts yet), report no findings and exit zero — there is no consumer view to compare against.

## Check Categories

### Check 1: `$ref` Resolution

All `$ref` pointers in OpenAPI and AsyncAPI files must resolve. Resolution scope covers both the change directory and the baseline:

- `$CHANGE_CONTRACTS/schemas/`
- `$BASELINE_CONTRACTS/schemas/`

#### OpenAPI files (`$CHANGE_CONTRACTS/http/`)

For each `.yaml` file in `$CHANGE_CONTRACTS/http/`:

1. Read the file and find all `$ref` values.
2. For each `$ref`, resolve the path relative to the file's location (e.g. `../schemas/user-registration.yaml`).
3. Check whether the resolved target exists in `$CHANGE_CONTRACTS` OR `$BASELINE_CONTRACTS`.
4. Report any `$ref` that does not resolve to an existing file.

#### AsyncAPI files (`$CHANGE_CONTRACTS/messages/`)

For each `.yaml` file in `$CHANGE_CONTRACTS/messages/`:

1. Same process as OpenAPI files above.

#### Report format

Per failure:

```
FAIL: contracts/http/user-api.yaml — $ref "../schemas/missing-type.yaml" does not resolve (checked change contracts/schemas/ and baseline .specify/contracts/schemas/)
```

### Check 2: Schema Metadata

Every JSON Schema file in `$CHANGE_CONTRACTS/schemas/` must have the required metadata fields defined in [JSON Schema Conventions](../../references/json-schema-conventions.md):

| Field | Rule |
|-------|------|
| `$id` | Present and a valid URI (URN format: `urn:specify:schemas/<name>`) |
| `title` | Present and non-empty, matching the type name |
| `description` | Present and non-empty |

Read each `.yaml` file in `$CHANGE_CONTRACTS/schemas/` and verify these three fields.

#### Report format

Per failure:

```
FAIL: contracts/schemas/user-registration.yaml — missing required field "$id"
FAIL: contracts/schemas/error-response.yaml — "description" is empty
```

### Check 3: Binding Completeness

Every schema that appears as a top-level request body, response body, or message payload in a spec scenario must have at least one protocol binding:

- An OpenAPI path in `$CHANGE_CONTRACTS/http/` or `$BASELINE_CONTRACTS/http/` that references the schema (for HTTP interactions)
- An AsyncAPI channel in `$CHANGE_CONTRACTS/messages/` or `$BASELINE_CONTRACTS/messages/` that references the schema (for messaging interactions)

#### Shared vocabulary exemption

Shared vocabulary types that appear only as `$ref` targets inside other schemas are exempt. They are reusable building blocks, not standalone endpoints. A schema qualifies as shared vocabulary if:

1. It is referenced via `$ref` from within other schema files (not directly from path/channel definitions), AND
2. It does not appear as a top-level request/response body or message payload in any spec scenario

Common examples: `error-response.yaml`, `pagination.yaml`.

#### Determining spec-referenced schemas

Read the spec files under `$CHANGE_SPECS` and identify schemas that appear as:

- Request body payloads (e.g. "accept a `UserRegistration` payload")
- Response body payloads (e.g. "respond with a `User` payload")
- Message payloads (e.g. "publish an `OrderPlaced` event")

Cross-reference these against the schema files in `$CHANGE_CONTRACTS/schemas/` and the protocol bindings in `$CHANGE_CONTRACTS/http/`, `$CHANGE_CONTRACTS/messages/`, `$BASELINE_CONTRACTS/http/`, and `$BASELINE_CONTRACTS/messages/`.

#### Report format

Per failure:

```
FAIL: contracts/schemas/user-registration.yaml — appears as request body in spec scenario REQ-001 but has no OpenAPI path binding
WARN: contracts/schemas/oauth-token.yaml — appears in spec but has no protocol binding (may be shared vocabulary — verify intent)
```

Use `FAIL` when the schema is clearly a top-level payload in a spec scenario. Use `WARN` when the classification is ambiguous — the verify-repair loop will surface the warning for human review.

## Algorithm (`single` mode)

1. **Determine scope.**
   - Change directory contracts: `$CHANGE_CONTRACTS`
   - Baseline contracts: `$BASELINE_CONTRACTS`
   - Change specs: `$CHANGE_SPECS`
   - If `$CHANGE_CONTRACTS` does not exist or contains no contract files, report all checks as passed and stop.

2. **Run Check 1** (`$ref` resolution) on all OpenAPI files in `$CHANGE_CONTRACTS/http/` and all AsyncAPI files in `$CHANGE_CONTRACTS/messages/`.

3. **Run Check 2** (schema metadata) on all JSON Schema files in `$CHANGE_CONTRACTS/schemas/`.

4. **Run Check 3** (binding completeness) by cross-referencing spec scenarios in `$CHANGE_SPECS` with schema files and protocol bindings across both change and baseline directories.

5. **Collect findings** and produce the validation report.

## Cross-Project Mode (RFC-9 §3B)

`cross-project` mode runs **after** a producer's contract change merges. The execute driver (`/spec:execute`) calls the validator once per `(producer-contract, consumer-workspace)` pair to detect breaking changes that would propagate downstream.

The mode is **non-fatal**: cross-project warnings never stop the execute loop. The driver records each warning to the merged change's `journal.yaml` (via `specify change journal append`) and renders a warning block in the merge transcript so the operator can triage.

### Compatibility checks

For each `(producer-contract, consumer-workspace)` pair, compare the producer's updated contract against the consumer's last-known view of the same contract. Resolve the consumer's view in this order:

1. `$CONSUMER_CONTRACTS/<relative-path>` — the consumer's materialised baseline at the matching path. This is what `specify workspace sync` populates from the central `.specify/contracts/`.
2. If absent, search `$CONSUMER_CONTRACTS/imports/` for a file with the same logical name (legacy import path used by some consumer clones).
3. If still absent, the consumer has no prior view — emit a single informational finding (`change-kind: consumer-has-no-baseline`) and stop. There is nothing to compare against.

When both files are present, classify each delta into a `change-kind`:

| `change-kind` | Severity | Description |
|---|---|---|
| `removed-field` | `warning` | A property the consumer's view defined is no longer in the producer's contract. Consumer code that reads the field will break. |
| `removed-endpoint` | `warning` | An OpenAPI path or operationId the consumer's view defined is gone. Consumer calls to it will fail. |
| `removed-channel` | `warning` | An AsyncAPI channel or operation the consumer's view defined is gone. Consumer subscribers will receive nothing. |
| `required-field-added` | `warning` | A new field is `required` in a request payload. Consumer requests built from the prior shape will be rejected. |
| `type-narrowed` | `warning` | A property's `type` (or `format`, `enum`, numeric range) became stricter. Consumer values that were valid before may now be rejected. |
| `status-code-removed` | `warning` | A response status code defined in the consumer's view is missing from the producer's update. Consumer error-handling for that code is dead. |
| `consumer-has-no-baseline` | `info` | The consumer's workspace clone has no prior view of this contract (first-time materialisation). No incompatibility — the consumer will pick up the new shape on its next `workspace sync`. |

Findings outside this table (additive optional fields, response field additions, new endpoints) are **not warnings** — they are backwards-compatible and the consumer keeps working unchanged.

### Algorithm (`cross-project` mode)

1. **Read inputs.**
   - Producer contract: parse `$PRODUCER_CONTRACT`. On read failure, exit non-zero with a `cannot-read-producer-contract` diagnostic.
   - Consumer view: locate the consumer's matching file under `$CONSUMER_CONTRACTS` (see [Compatibility checks](#compatibility-checks) for the resolution order).
   - If no consumer view is found, emit one `consumer-has-no-baseline` finding and skip steps 2–4.

2. **Detect format.** Read the top-level keys to classify the contract:
   - `openapi:` → OpenAPI 3.x (HTTP).
   - `asyncapi:` → AsyncAPI 3.x (messaging).
   - `$schema:` or `$id:` → JSON Schema (payload).

3. **Run format-specific compatibility checks.**
   - **OpenAPI:** walk `paths[*][method]` in the consumer's view. For each operation, locate the matching `(path, method)` in the producer's contract. Classify removals, required-field additions in `requestBody`, type narrowings in request/response schemas, and missing response status codes.
   - **AsyncAPI:** walk `channels[*]` and `operations[*]` in the consumer's view. Classify removed channels, removed operations, and payload type narrowings.
   - **JSON Schema:** walk `properties` and `required`. Classify removed properties, newly-required properties, and type narrowings on shared properties.

4. **Collect findings.** Each finding records `{ severity, contract, change-kind, locator, details }`.

5. **Emit the structured YAML report** (see [Output Format — Cross-Project](#output-format--cross-project)).

The validator does not walk the consumer's spec / source code in this mode — that level of analysis is out of scope and would re-couple the validator to the consumer's implementation. The conservative output is "the wire shape changed in a backwards-incompatible direction; the operator should triage."

## Output Format

The validator produces a structured markdown report.

When issues are found:

```markdown
## Validation Report

### $ref Resolution
- ✗ contracts/http/user-api.yaml — $ref "../schemas/missing-type.yaml" does not resolve
- ✓ 11 of 12 $ref pointers resolve

### Schema Metadata
- ✗ contracts/schemas/user-registration.yaml — missing "description"
- ✓ 5 of 6 schemas have complete metadata

### Binding Completeness
- ✓ All spec-referenced schemas have protocol bindings

### Summary
- **Checks passed:** 2 of 3
- **Issues found:** 2
```

When all checks pass:

```markdown
## Validation Report

All checks passed (12 $ref pointers, 6 schemas, 4 bindings verified).
```

## Output Format — Cross-Project

`cross-project` mode emits a structured YAML report on stdout. The execute driver parses this report directly; the schema is stable and machine-readable.

```yaml
mode: cross-project
producer:
  contract: contracts/http/user-api.yaml
consumer:
  workspace: .specify/workspace/mobile/
findings:
  - severity: warning
    contract: contracts/http/user-api.yaml
    change-kind: removed-field
    locator: paths./users/{id}.get.responses.200.content.application/json.schema.properties.email
    details: >
      The producer's update removes the `email` field from the
      GET /users/{id} response body. The consumer's last-known view
      defines this field; consumer code that reads it will receive
      `undefined` after the next workspace sync.
  - severity: warning
    contract: contracts/http/user-api.yaml
    change-kind: required-field-added
    locator: paths./users.post.requestBody.content.application/json.schema.required
    details: >
      Producer adds `phone-number` to the required field list of
      POST /users. Consumer requests built from the prior shape will
      be rejected with HTTP 400.
summary:
  total-findings: 2
  warnings: 2
  errors: 0
```

When no findings are produced (the consumer's view matches the producer's update, or the consumer has no prior view):

```yaml
mode: cross-project
producer:
  contract: contracts/http/user-api.yaml
consumer:
  workspace: .specify/workspace/mobile/
findings: []
summary:
  total-findings: 0
  warnings: 0
  errors: 0
```

The report is well-formed even when empty — the execute driver always parses `summary.total-findings` to decide whether to render a warning block in the merge transcript.

### Locator format

`locator` strings are dot-separated paths into the contract document. The convention follows the OpenAPI / AsyncAPI / JSON Schema natural traversal order:

- OpenAPI request fields: `paths.<path>.<method>.requestBody.content.<media-type>.schema.properties.<field>`
- OpenAPI response fields: `paths.<path>.<method>.responses.<status>.content.<media-type>.schema.properties.<field>`
- AsyncAPI message fields: `channels.<name>.messages.<message-id>.payload.properties.<field>`
- JSON Schema fields: `properties.<field>` (with nested objects: `properties.<field>.properties.<nested>`)

Path segments containing dots (e.g. `application/json`) are kept verbatim — locators are not parsed by the validator, only emitted for human triage.

### Exit semantics

`cross-project` mode exits **0** even when warnings are present. The mode is non-fatal by design (RFC-9 §3B). Exit non-zero only when:

- `$PRODUCER_CONTRACT` cannot be read (`cannot-read-producer-contract`).
- `$CONSUMER_WORKSPACE` cannot be reached (e.g. permission denied).
- The producer's contract is malformed and cannot be parsed.

`single` mode preserves its existing exit semantics: zero on clean reports, non-zero on read errors.

## Edge Cases

### `single` mode

| Scenario | Behavior |
|----------|----------|
| Change directory has no `contracts/` | Pass — nothing to validate |
| Baseline has contracts but change does not | Pass — validator only checks change-level artifacts |
| `$ref` target exists in baseline but not in change | Pass — baseline is a valid resolution target |
| `$ref` target exists in change but not in baseline | Pass — change-level schemas are valid resolution targets |
| Mixed resolution: some targets in baseline, some in change | Pass — both directories are valid resolution scope |
| No spec files in the change | Skip Check 3 (binding completeness) — no scenarios to cross-reference |
| Schema referenced only via `$ref` from other schemas | Exempt from Check 3 (shared vocabulary) |

### `cross-project` mode

| Scenario | Behavior |
|----------|----------|
| `$CONSUMER_CONTRACTS` does not exist (consumer never sync'd) | Emit one `consumer-has-no-baseline` finding (severity `info`); exit 0 |
| Consumer's view matches the producer's update byte-for-byte | Empty `findings`; exit 0 |
| `$PRODUCER_CONTRACT` cannot be read | Exit non-zero with `cannot-read-producer-contract` diagnostic |
| Producer contract is malformed YAML | Exit non-zero with `producer-contract-malformed` diagnostic |
| Format mismatch (OpenAPI on producer, JSON Schema on consumer) | Emit one `format-mismatch` finding (severity `warning`); exit 0 |
| Consumer view contains additive fields the producer never defined | Pass silently — additive fields are the consumer's prerogative |

## Guardrails

- **Read-only.** Never create, modify, or delete files. Both modes share this contract.
- Report every issue with the file path and a description of the problem.
- When uncertain whether a schema is shared vocabulary or a standalone payload, use `WARN` rather than `FAIL` (in `single` mode).
- Do not attempt to fix issues — report them.
- **Cross-project warnings are non-fatal.** Always exit 0 in `cross-project` mode unless the input cannot be read or parsed. The execute driver decides whether to halt; the validator only reports.
- Do not walk consumer source code or specs in `cross-project` mode. The consumer-side analysis stops at the contract file the consumer's workspace clone holds.

## Verification Checklist

### `single` mode

Before completing validation:

- [ ] All `.yaml` files in `$CHANGE_CONTRACTS/http/` scanned for `$ref` resolution
- [ ] All `.yaml` files in `$CHANGE_CONTRACTS/messages/` scanned for `$ref` resolution
- [ ] All `.yaml` files in `$CHANGE_CONTRACTS/schemas/` checked for `$id`, `title`, `description`
- [ ] Spec scenarios cross-referenced against schema bindings (when specs exist)
- [ ] Shared vocabulary exemption applied correctly
- [ ] Validation report produced with per-check results and summary
- [ ] No files created or modified

### `cross-project` mode

Before completing validation:

- [ ] `$PRODUCER_CONTRACT` parsed successfully (or reported as `cannot-read-producer-contract`)
- [ ] Consumer's matching view located under `$CONSUMER_CONTRACTS` (or reported as `consumer-has-no-baseline`)
- [ ] Format-specific checks run (OpenAPI / AsyncAPI / JSON Schema)
- [ ] Each delta classified into a known `change-kind`
- [ ] YAML report emitted with `mode`, `producer`, `consumer`, `findings`, and `summary`
- [ ] Exit status reflects exit-semantics rules (0 with findings; non-zero only on read failure)
- [ ] No files created or modified
