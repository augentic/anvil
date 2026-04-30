# OpenAPI — Verifier

> **When to read this.** Read this when verifying an OpenAPI artefact — invoked by the contracts schema build brief in `single` mode after the author or importer sibling produces output, by `/spec:execute` in `cross-project` mode after a producer's contract change merges (RFC-9 §3B), or directly by an operator running validation against an existing artefact. Skip this file when authoring (use [`author.md`](./author.md)) or normalising an external document (use [`importer.md`](./importer.md)).

The verifier is **read-only**. It MUST NOT generate, modify, or delete any files. Its sole output is a list of issues rendered as a validation report.

## Modes

The verifier accepts a `--mode {single, cross-project}` flag. The mode determines the report shape and the exit semantics.

| Mode | Caller | Trigger | Scope | Output |
|---|---|---|---|---|
| `single` (default) | contracts schema build brief in `/spec:build` | Post-author or post-import | One change's `contracts/http/` inside one project | Markdown report for the verify-repair loop |
| `cross-project` | `/spec:execute` post-merge step | Producer-side merge of an OpenAPI contract change | Compare merged OpenAPI against each consumer's tier-2 workspace clone | Structured YAML report consumed by the execute driver |

`single` mode feeds the brief's verify-repair loop. `cross-project` mode emits warnings that the execute driver records in the merging change's `journal.yaml` — never halts the loop. Both modes share the read-only contract.

`--mode` was previously exposed as a top-level flag on the standalone validator skill in the (now retired) `contracts` plugin, invoked as `--mode {single, cross-project}` (RFC-10 §C.3). It is now an internal flag of the format-specific verifier; the surface area, algorithms, and output shapes are unchanged.

## Inputs

### `single` mode

Inferred from the active change context — no positional arguments required:

```text
$CHANGE_DIR          = .specify/changes/<change-name>
$CHANGE_CONTRACTS    = $CHANGE_DIR/contracts/
$BASELINE_CONTRACTS  = .specify/contracts/
$CHANGE_SPECS        = $CHANGE_DIR/specs/
```

### `cross-project` mode

Caller passes the producer's updated contract path and the consumer's workspace clone path:

```text
$PRODUCER_CONTRACT  = <path-to-producer-contract>     # e.g. .specify/contracts/http/user-api.yaml
$CONSUMER_WORKSPACE = <path-to-consumer-workspace>    # e.g. .specify/workspace/mobile/
$CONSUMER_CONTRACTS = $CONSUMER_WORKSPACE/.specify/contracts/
```

`$PRODUCER_CONTRACT` is a path relative to the initiating repo root (typically a file under `.specify/contracts/http/` after a producer change merges). `$CONSUMER_WORKSPACE` is a tier-2 workspace clone — `specify workspace sync` materialises consumer clones at `.specify/workspace/<consumer-name>/`, and the consumer's view of central contracts lives at `$CONSUMER_CONTRACTS`.

## Prerequisites

### `single` mode

- The author or importer sibling has completed and produced artefacts under `$CHANGE_CONTRACTS/http/`.
- `.specify/project.yaml` exists (Specify is initialised).

If `$CHANGE_CONTRACTS/http/` does not exist or contains no files, report all checks as passed — there is nothing to verify.

### `cross-project` mode

- `$PRODUCER_CONTRACT` exists and is readable. Otherwise exit non-zero with a `cannot-read-producer-contract` diagnostic.
- `$CONSUMER_WORKSPACE` exists. If `$CONSUMER_CONTRACTS` is absent (the consumer has never sync'd), emit a single `consumer-has-no-baseline` finding (severity `info`) and exit zero.

## Single-mode checks

Three independent checks run against `$CHANGE_CONTRACTS/http/` and the schemas it references. Run them in order; collect findings; produce a single markdown report at the end.

### Check 1 — `$ref` resolution

All `$ref` pointers in OpenAPI files must resolve to existing schema files. Resolution scope spans both the change directory and the baseline:

- `$CHANGE_CONTRACTS/schemas/`
- `$BASELINE_CONTRACTS/schemas/`

For each `.yaml` file in `$CHANGE_CONTRACTS/http/`:

1. Read the file and find every `$ref` value (request bodies, response bodies, parameters, security schemes that reference shared definitions).
2. For each `$ref`, resolve the path relative to the file's location (e.g. `../schemas/user-registration.yaml`).
3. Check whether the resolved target exists in `$CHANGE_CONTRACTS` **or** `$BASELINE_CONTRACTS`. Either is a valid resolution scope.
4. Report any `$ref` that does not resolve.

Report format (one entry per failure):

```
FAIL: contracts/http/user-api.yaml — $ref "../schemas/missing-type.yaml" does not resolve (checked change contracts/schemas/ and baseline .specify/contracts/schemas/)
```

`$ref` pointers using `#/components/...` (in-document) are also checked — they must resolve to a sibling key inside the same file. The verifier does not chase external URL `$ref`s; it flags them as `WARN` (cross-format URL refs are out of scope).

### Check 2 — Schema metadata

Every JSON Schema file in `$CHANGE_CONTRACTS/schemas/` referenced by an OpenAPI operation in `$CHANGE_CONTRACTS/http/` must have the required metadata fields defined in [`../../references/json-schema-conventions.md`](../../references/json-schema-conventions.md):

| Field | Rule |
|---|---|
| `$id` | Present and a valid URI (URN format: `urn:specify:schemas/<name>`). |
| `title` | Present and non-empty, matching the type name. |
| `description` | Present and non-empty. The placeholder `"[imported — description pending review]"` counts as present but **emits a `WARN`** to surface the gap for the verify-repair loop. |

Report format (one entry per failure):

```
FAIL: contracts/schemas/user-registration.yaml — missing required field "$id"
FAIL: contracts/schemas/error-response.yaml — "description" is empty
WARN: contracts/schemas/user.yaml — "description" is "[imported — description pending review]"; replace before merge
```

### Check 3 — Binding completeness

Every schema that appears as a top-level request body, response body, or parameter shape in a spec scenario must have at least one OpenAPI operation that references it.

Resolution scope for the binding:

- `$CHANGE_CONTRACTS/http/` — operations added by this change.
- `$BASELINE_CONTRACTS/http/` — operations already in the platform baseline.

#### Determining spec-referenced schemas

Read the `*.md` files under `$CHANGE_SPECS` and identify schemas that the spec mentions as:

- Request body payloads (e.g. "accept a `UserRegistration` payload").
- Response body payloads (e.g. "respond with a `User` payload").
- Parameter shape payloads (rare; usually inline).

Cross-reference these against the schema files in `$CHANGE_CONTRACTS/schemas/` and the operations in `$CHANGE_CONTRACTS/http/` and `$BASELINE_CONTRACTS/http/`.

#### Shared vocabulary exemption

Shared vocabulary types that appear only as `$ref` targets inside other schemas are exempt — they are reusable building blocks, not standalone payloads. A schema qualifies as shared vocabulary if both:

1. It is referenced via `$ref` from within other schema files (not directly from path / response definitions), AND
2. It does not appear as a top-level request / response body in any spec scenario.

Common examples: `error-response.yaml`, `pagination.yaml`.

Report format:

```
FAIL: contracts/schemas/user-registration.yaml — appears as request body in spec scenario REQ-001 but has no OpenAPI path binding
WARN: contracts/schemas/oauth-token.yaml — appears in spec but has no protocol binding (may be shared vocabulary — verify intent)
```

Use `FAIL` when the schema is unambiguously a top-level payload in a spec scenario. Use `WARN` when classification is ambiguous — the verify-repair loop surfaces the warning for human review.

When the change has **no specs**, skip Check 3 — there are no scenarios to cross-reference. Record this in the report so the brief knows the check was deliberately bypassed.

## Single-mode algorithm

1. **Determine scope.**
   - `$CHANGE_CONTRACTS/http/`, `$CHANGE_CONTRACTS/schemas/`.
   - `$BASELINE_CONTRACTS/http/`, `$BASELINE_CONTRACTS/schemas/`.
   - `$CHANGE_SPECS/`.
   - If `$CHANGE_CONTRACTS/http/` is empty or absent, report all checks as passed and stop.
2. **Run Check 1** ($ref resolution) on every `.yaml` file in `$CHANGE_CONTRACTS/http/`.
3. **Run Check 2** (schema metadata) on every `.yaml` file in `$CHANGE_CONTRACTS/schemas/` referenced by an OpenAPI operation.
4. **Run Check 3** (binding completeness) by cross-referencing spec scenarios with OpenAPI operations across change and baseline. Skip if no specs.
5. **Collect findings** and produce the markdown validation report.

## Single-mode output format

When issues are found:

```markdown
## Validation Report (HTTP)

### $ref Resolution
- ✗ contracts/http/user-api.yaml — $ref "../schemas/missing-type.yaml" does not resolve
- ✓ 11 of 12 $ref pointers resolve

### Schema Metadata
- ✗ contracts/schemas/user-registration.yaml — missing "description"
- ✓ 5 of 6 schemas have complete metadata

### Binding Completeness
- ✓ All spec-referenced schemas have OpenAPI bindings

### Summary
- **Checks passed:** 1 of 3
- **Issues found:** 2
```

When all checks pass:

```markdown
## Validation Report (HTTP)

All checks passed (12 $ref pointers, 6 schemas, 4 operations verified).
```

`single` mode preserves its existing exit semantics: zero on clean reports, non-zero on read errors.

## Cross-project mode

`cross-project` mode runs **after** a producer's contract change merges. The execute driver (`/spec:execute`) calls the verifier once per `(producer-contract, consumer-workspace)` pair to detect breaking changes that would propagate downstream.

The mode is **non-fatal**: cross-project warnings never stop the execute loop. The driver records each warning to the merged change's `journal.yaml` (via `specify change journal append`) and renders a warning block in the merge transcript so the operator can triage.

### Compatibility checks

For each `(producer-contract, consumer-workspace)` pair, compare the producer's updated contract against the consumer's last-known view of the same contract. Resolve the consumer's view in this order:

1. `$CONSUMER_CONTRACTS/<relative-path>` — the consumer's materialised baseline at the matching path. This is what `specify workspace sync` populates from the central `.specify/contracts/`.
2. If absent, search `$CONSUMER_CONTRACTS/imports/` for a file with the same logical name (legacy import path used by some consumer clones).
3. If still absent, the consumer has no prior view — emit a single `consumer-has-no-baseline` finding and stop. There is nothing to compare against.

When both files are present, classify each delta into a `change-kind`:

| `change-kind` | Severity | Description |
|---|---|---|
| `removed-field` | `warning` | A property the consumer's view defined is no longer in the producer's contract. Consumer code that reads the field will break. |
| `removed-endpoint` | `warning` | An OpenAPI path or operationId the consumer's view defined is gone. Consumer calls to it will fail. |
| `required-field-added` | `warning` | A new field is `required` in a request payload. Consumer requests built from the prior shape will be rejected. |
| `type-narrowed` | `warning` | A property's `type` (or `format`, `enum`, numeric range) became stricter. Consumer values that were valid before may now be rejected. |
| `status-code-removed` | `warning` | A response status code defined in the consumer's view is missing from the producer's update. Consumer error-handling for that code is dead. |
| `consumer-has-no-baseline` | `info` | The consumer's workspace clone has no prior view of this contract (first-time materialisation). No incompatibility — the consumer will pick up the new shape on its next `workspace sync`. |

Findings outside this table (additive optional fields, response field additions, new endpoints) are **not warnings** — they are backwards-compatible and the consumer keeps working unchanged.

### Cross-project algorithm

1. **Read inputs.**
   - Producer contract: parse `$PRODUCER_CONTRACT`. On read failure, exit non-zero with a `cannot-read-producer-contract` diagnostic.
   - Consumer view: locate the consumer's matching file under `$CONSUMER_CONTRACTS` using the resolution order above.
   - If no consumer view is found, emit one `consumer-has-no-baseline` finding and skip steps 2–4.
2. **Confirm format.** Read the top-level keys; the file must have `openapi: "3.x"`. If it has `asyncapi:` or `$schema:` instead, emit a `format-mismatch` finding and exit zero — the wrong verifier was invoked.
3. **Run OpenAPI compatibility checks.** Walk `paths[*][method]` in the consumer's view. For each operation, locate the matching `(path, method)` in the producer's contract. Classify removals, required-field additions in `requestBody`, type narrowings in request and response schemas, and missing response status codes.
4. **Collect findings.** Each finding records `{ severity, contract, change-kind, locator, details }`.
5. **Emit the structured YAML report** (see [Cross-project output format](#cross-project-output-format)).

The verifier does not walk the consumer's spec or source code in this mode — that level of analysis is out of scope and would re-couple the verifier to the consumer's implementation. The conservative output is "the wire shape changed in a backwards-incompatible direction; the operator should triage."

## Cross-project output format

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
    locator: paths./users/{user_id}.get.responses.200.content.application/json.schema.properties.email
    details: >
      The producer's update removes the `email` field from the
      GET /users/{user_id} response body. The consumer's last-known view
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

`locator` strings are dot-separated paths into the contract document, following OpenAPI's natural traversal order:

- Request fields: `paths.<path>.<method>.requestBody.content.<media-type>.schema.properties.<field>`
- Response fields: `paths.<path>.<method>.responses.<status>.content.<media-type>.schema.properties.<field>`
- Required-field changes: `paths.<path>.<method>.requestBody.content.<media-type>.schema.required`
- Removed endpoints: `paths.<path>.<method>`

Path segments containing dots (e.g. `application/json`) are kept verbatim — locators are emitted for human triage, not parsed.

### Cross-project exit semantics

`cross-project` mode exits **0** even when warnings are present. The mode is non-fatal by design (RFC-9 §3B). Exit non-zero only when:

- `$PRODUCER_CONTRACT` cannot be read (`cannot-read-producer-contract`).
- `$CONSUMER_WORKSPACE` cannot be reached (e.g. permission denied).
- The producer's contract is malformed and cannot be parsed (`producer-contract-malformed`).

## Edge cases

### `single` mode

| Scenario | Behavior |
|---|---|
| Change directory has no `contracts/http/` | Pass — nothing to verify. |
| Baseline has HTTP contracts but change does not | Pass — verifier only checks change-level artefacts. |
| `$ref` target exists in baseline but not in change | Pass — baseline is a valid resolution target. |
| `$ref` target exists in change but not in baseline | Pass — change-level schemas are valid resolution targets. |
| Mixed resolution: some targets in baseline, some in change | Pass — both directories are valid resolution scope. |
| No spec files in the change | Skip Check 3; record the skip in the report. |
| Schema referenced only via `$ref` from other schemas | Exempt from Check 3 (shared vocabulary). |
| Operation uses `components/schemas` (legacy) | `$ref` resolution still verified inside the document; emit `WARN` recommending importer normalisation. |

### `cross-project` mode

| Scenario | Behavior |
|---|---|
| `$CONSUMER_CONTRACTS` does not exist (consumer never sync'd) | Emit `consumer-has-no-baseline` finding (severity `info`); exit 0. |
| Consumer's view matches the producer's update byte-for-byte | Empty `findings`; exit 0. |
| `$PRODUCER_CONTRACT` cannot be read | Exit non-zero with `cannot-read-producer-contract`. |
| Producer contract is malformed YAML | Exit non-zero with `producer-contract-malformed`. |
| Format mismatch (consumer has AsyncAPI / JSON Schema at the same path) | Emit `format-mismatch` finding (severity `warning`); exit 0. |
| Consumer view contains additive fields the producer never defined | Pass silently — additive fields are the consumer's prerogative. |

## Guardrails

- **Read-only.** Never create, modify, or delete files. Both modes share this contract.
- Report every issue with the file path and a description of the problem.
- When uncertain whether a schema is shared vocabulary or a standalone payload, use `WARN` rather than `FAIL` (in `single` mode).
- Do not attempt to fix issues — report them. Repair belongs to the author or importer sibling.
- **Cross-project warnings are non-fatal.** Always exit 0 in `cross-project` mode unless the input cannot be read or parsed. The execute driver decides whether to halt; the verifier only reports.
- Do not walk consumer source code or specs in `cross-project` mode. The consumer-side analysis stops at the contract file the consumer's workspace clone holds.

## Verification checklist

### `single` mode

Before completing the run:

- [ ] All `.yaml` files in `$CHANGE_CONTRACTS/http/` scanned for `$ref` resolution.
- [ ] All `.yaml` files in `$CHANGE_CONTRACTS/schemas/` referenced by HTTP operations checked for `$id`, `title`, `description`.
- [ ] Spec scenarios cross-referenced against OpenAPI bindings (when specs exist).
- [ ] Shared vocabulary exemption applied correctly.
- [ ] Validation report produced with per-check results and summary.
- [ ] No files created or modified.

### `cross-project` mode

Before completing the run:

- [ ] `$PRODUCER_CONTRACT` parsed successfully (or reported as `cannot-read-producer-contract`).
- [ ] Consumer's matching view located under `$CONSUMER_CONTRACTS` (or reported as `consumer-has-no-baseline`).
- [ ] OpenAPI compatibility checks ran (paths, methods, request bodies, response bodies, status codes).
- [ ] Each delta classified into a known `change-kind`.
- [ ] YAML report emitted with `mode`, `producer`, `consumer`, `findings`, and `summary`.
- [ ] Exit status reflects exit-semantics rules (0 with findings; non-zero only on read failure).
- [ ] No files created or modified.

## See also

- [`../../references/openapi-conventions.md`](../../references/openapi-conventions.md) — OpenAPI 3.1 structure rules.
- [`../../references/json-schema-conventions.md`](../../references/json-schema-conventions.md) — schema metadata rules.
- [`../../references/artifact-structure.md`](../../references/artifact-structure.md) — directory layout for the change-local delta and the baseline.
- [`../../references/report-shape.md`](../../references/report-shape.md) — single-mode markdown report and cross-project YAML report formats this verifier emits, including severity levels and locator format.
- [`../../references/cross-project-compatibility.md`](../../references/cross-project-compatibility.md) — `change-kind` enumeration, consumer-view resolution, breaking-change classification policy.
- [`author.md`](./author.md) — sibling for spec-driven authoring.
- [`importer.md`](./importer.md) — sibling for normalising external documents.
