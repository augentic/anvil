---
name: validator
description: "Validate internal consistency of API contract artifacts — $ref resolution, schema metadata completeness, and binding coverage."
license: MIT
allowed-tools: Read, Grep, Glob
---

# Contracts Validator

Validate internal consistency of contract artifacts after the writer completes. The validator does not generate or modify contract files — it reports issues for the brief's verify-repair loop to act on.

## Scope Rules

**The validator is read-only.** It MUST NOT generate, modify, or delete any files. Its sole output is a list of issues (file path + description) rendered as a validation report. The contracts brief's verify-repair loop feeds these issues back to the writer for correction.

## Derived Arguments

The validator infers its scope from the active change context:

```text
$CHANGE_DIR      = .specify/changes/<change-name>
$CHANGE_CONTRACTS = $CHANGE_DIR/contracts/
$BASELINE_CONTRACTS = .specify/contracts/
$CHANGE_SPECS    = $CHANGE_DIR/specs/
```

## Prerequisites

- The `/contracts:writer` has completed and produced artifacts under `$CHANGE_CONTRACTS`.
- `.specify/project.yaml` exists (Specify is initialized).

If `$CHANGE_CONTRACTS` does not exist or contains no files, report all checks as passed — there is nothing to validate.

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

## Algorithm

1. **Determine scope.**
   - Change directory contracts: `$CHANGE_CONTRACTS`
   - Baseline contracts: `$BASELINE_CONTRACTS`
   - Change specs: `$CHANGE_SPECS`
   - If `$CHANGE_CONTRACTS` does not exist or contains no contract files, report all checks as passed and stop.

2. **Run Check 1** (`$ref` resolution) on all OpenAPI files in `$CHANGE_CONTRACTS/http/` and all AsyncAPI files in `$CHANGE_CONTRACTS/messages/`.

3. **Run Check 2** (schema metadata) on all JSON Schema files in `$CHANGE_CONTRACTS/schemas/`.

4. **Run Check 3** (binding completeness) by cross-referencing spec scenarios in `$CHANGE_SPECS` with schema files and protocol bindings across both change and baseline directories.

5. **Collect findings** and produce the validation report.

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

## Edge Cases

| Scenario | Behavior |
|----------|----------|
| Change directory has no `contracts/` | Pass — nothing to validate |
| Baseline has contracts but change does not | Pass — validator only checks change-level artifacts |
| `$ref` target exists in baseline but not in change | Pass — baseline is a valid resolution target |
| `$ref` target exists in change but not in baseline | Pass — change-level schemas are valid resolution targets |
| Mixed resolution: some targets in baseline, some in change | Pass — both directories are valid resolution scope |
| No spec files in the change | Skip Check 3 (binding completeness) — no scenarios to cross-reference |
| Schema referenced only via `$ref` from other schemas | Exempt from Check 3 (shared vocabulary) |

## Guardrails

- **Read-only.** Never create, modify, or delete files.
- Report every issue with the file path and a description of the problem.
- When uncertain whether a schema is shared vocabulary or a standalone payload, use `WARN` rather than `FAIL`.
- Do not attempt to fix issues — report them for the verify-repair loop.

## Verification Checklist

Before completing validation:

- [ ] All `.yaml` files in `$CHANGE_CONTRACTS/http/` scanned for `$ref` resolution
- [ ] All `.yaml` files in `$CHANGE_CONTRACTS/messages/` scanned for `$ref` resolution
- [ ] All `.yaml` files in `$CHANGE_CONTRACTS/schemas/` checked for `$id`, `title`, `description`
- [ ] Spec scenarios cross-referenced against schema bindings (when specs exist)
- [ ] Shared vocabulary exemption applied correctly
- [ ] Validation report produced with per-check results and summary
- [ ] No files created or modified
