# JSON Schema — Verifier

> **When to read this.** Read this when verifying a JSON Schema artefact — invoked by the contracts capability build brief in `single` mode after the author or importer sibling produces output, by `/change:execute` in `cross-project` mode after a producer's contract change merges (RFC-9 §3B), or directly by an operator running validation against existing artefacts. Skip this file when authoring (use [`author.md`](./author.md)) or normalising external documents (use [`importer.md`](./importer.md)).

The verifier is **read-only**. It MUST NOT generate, modify, or delete any files. Its sole output is a list of issues rendered as a validation report.

## Modes

The verifier accepts a `--mode {single, cross-project}` flag. The mode determines the report shape and the exit semantics.

| Mode | Caller | Trigger | Scope | Output |
|---|---|---|---|---|
| `single` (default) | contracts capability build brief in `/spec:build` | Post-author or post-import | One change's `contracts/schemas/` inside one project, plus the slice's and baseline's HTTP / messaging consumers | Markdown report for the verify-repair loop |
| `cross-project` | `/change:execute` post-merge step | Producer-side merge of a contract change touching schemas | Compare merged schemas against each consumer's tier-2 workspace clone | Structured YAML report consumed by the execute driver |

`single` mode feeds the brief's verify-repair loop and is the natural exit point for both author and importer runs. `cross-project` mode emits warnings the execute driver records on the merging change's `journal.yaml` — never halts the loop. Both modes share the read-only contract.

`--mode` was previously exposed as a top-level flag on the standalone validator skill in the (now retired) `contracts` plugin, invoked as `--mode {single, cross-project}` (RFC-10 §C.3). It is now an internal flag of the format-specific verifier; the surface area, algorithms, and output shapes are unchanged.

## Inputs

### `single` mode

Inferred from the active slice context — no positional arguments required:

```text
$SLICE_DIR          = .specify/slices/<slice-name>
$CHANGE_CONTRACTS    = $SLICE_DIR/contracts/
$CHANGE_SCHEMAS      = $CHANGE_CONTRACTS/schemas/
$BASELINE_CONTRACTS  = contracts/
$BASELINE_SCHEMAS    = $BASELINE_CONTRACTS/schemas/
$CHANGE_SPECS        = $SLICE_DIR/specs/
```

### `cross-project` mode

Caller passes the producer's updated schema path and the consumer's workspace clone path:

```text
$PRODUCER_CONTRACT  = <path-to-producer-schema>      # e.g. contracts/schemas/user.yaml
$CONSUMER_WORKSPACE = <path-to-consumer-workspace>   # e.g. .specify/workspace/mobile/
$CONSUMER_CONTRACTS = $CONSUMER_WORKSPACE/contracts/
```

`$PRODUCER_CONTRACT` is a path relative to the initiating repo root (typically a file under `contracts/schemas/` after a producer change merges). `$CONSUMER_WORKSPACE` is a tier-2 workspace clone — `specify workspace sync` materialises consumer clones at `.specify/workspace/<consumer-name>/`, and the consumer's view of central schemas lives at `$CONSUMER_CONTRACTS/schemas/`.

## Prerequisites

### `single` mode

- The author or importer sibling has completed and produced artefacts under `$CHANGE_SCHEMAS`.
- `.specify/project.yaml` exists (Specify is initialised).

If `$CHANGE_SCHEMAS` does not exist or contains no files, report all checks as passed — there is nothing to verify.

### `cross-project` mode

- `$PRODUCER_CONTRACT` exists and is readable. Otherwise exit non-zero with a `cannot-read-producer-contract` diagnostic.
- `$CONSUMER_WORKSPACE` exists. If `$CONSUMER_CONTRACTS/schemas/` is absent (the consumer has never sync'd), emit a single `consumer-has-no-baseline` finding (severity `info`) and exit zero.

## Single-mode checks

Four independent checks run against `$CHANGE_SCHEMAS` and the artefacts that consume it. Run them in order; collect findings; produce a single markdown report at the end.

### Check 1 — `$ref` resolution

Every `$ref` in every schema file under `$CHANGE_SCHEMAS` must resolve. Three resolution scopes apply depending on the kind of `$ref`:

- **Cross-file refs to siblings** (`$ref: "<other-name>.yaml"`) — must resolve to a file in `$CHANGE_SCHEMAS` or in `$BASELINE_SCHEMAS`. Both are valid resolution scopes; mixed resolution (one delta + one baseline) is fine.
- **In-document refs** (`$ref: "#/$defs/<name>"`) — must resolve to a sibling key inside the same file's `$defs` map.
- **External URL refs** (`$ref: "https://..."`) — flagged as `WARN`. The verifier never chases external URLs.

For each `.yaml` file in `$CHANGE_SCHEMAS`:

1. Read the file and find every `$ref` value (top-level, nested in `properties`, nested in `items`, nested in `oneOf` / `anyOf` / `allOf`, nested in `$defs`).
2. Classify each `$ref` as cross-file, in-document, or external URL.
3. Resolve cross-file refs against `$CHANGE_SCHEMAS` and `$BASELINE_SCHEMAS`; resolve in-document refs against the same file's `$defs`.
4. Report any `$ref` that does not resolve.

Report format (one entry per failure):

```
FAIL: contracts/schemas/order.yaml — $ref "missing-type.yaml" does not resolve (checked change contracts/schemas/ and baseline contracts/schemas/)
FAIL: contracts/schemas/user.yaml — $ref "#/$defs/MissingSubType" does not resolve in-document
WARN: contracts/schemas/legacy.yaml — $ref "https://example.com/schemas/foo" is an external URL; not chased
```

### Check 2 — Metadata completeness

Every JSON Schema file in `$CHANGE_SCHEMAS` must have the required metadata fields defined in [`../../references/json-schema-conventions.md`](../../references/json-schema-conventions.md).

| Field | Rule |
|---|---|
| `$schema` | Present and equal to `"https://json-schema.org/draft/2020-12/schema"`. Older drafts emit `WARN` (importer should have upgraded). |
| `$id` | Present, well-formed URN of the shape `urn:specify:schemas/<segment>` where `<segment>` matches the kebab-case filename. |
| `title` | Present, non-empty, PascalCase, and corresponds to the filename (kebab-case → PascalCase round-trips). |
| `description` | Present, non-empty. The placeholder `"[imported — description pending review]"` counts as present but **emits a `WARN`** to surface the gap for the verify-repair loop before merge. |
| `type` | Present (almost always `object`; primitives are rare). |

Report format (one entry per failure):

```
FAIL: contracts/schemas/user-registration.yaml — missing required field "$id"
FAIL: contracts/schemas/error-response.yaml — "description" is empty
FAIL: contracts/schemas/user.yaml — "$id" is "urn:example:user"; expected "urn:specify:schemas/user"
FAIL: contracts/schemas/order.yaml — "title" is "order"; expected PascalCase ("Order")
WARN: contracts/schemas/payment.yaml — "$schema" is Draft 7; expected Draft 2020-12 (importer normalisation needed)
WARN: contracts/schemas/oauth-token.yaml — "description" is "[imported — description pending review]"; replace before merge
```

### Check 3 — Duplicate-`$id` detection

Across every schema file in `$CHANGE_SCHEMAS` plus every schema file in `$BASELINE_SCHEMAS`, the `$id` values must be globally unique. The author's filename → `$id` derivation guarantees this when the one-type-per-file rule holds, but importer paths and manual edits can break the invariant.

Algorithm:

1. Read every `.yaml` under `$CHANGE_SCHEMAS` and `$BASELINE_SCHEMAS`. Record `(filename, $id)` pairs.
2. Group by `$id`. Any group with more than one entry is a collision.
3. Classify each collision:

| Collision kind | Severity | Description |
|---|---|---|
| Two delta files share `$id` | `FAIL` | The slice is internally inconsistent. |
| Delta file shares `$id` with a baseline file but the **filenames** differ | `FAIL` | The author / importer reassigned a baseline `$id` (forbidden by the `$id` stability rule). |
| Delta file shares `$id` with a baseline file and the filenames match | `INFO` | Expected — the delta replaces the baseline file at merge time. |

Report format:

```
FAIL: contracts/schemas/user-billing.yaml and contracts/schemas/user-platform.yaml share $id "urn:specify:schemas/user-billing"
FAIL: contracts/schemas/oauth-token.yaml shares $id "urn:specify:schemas/auth-token" with contracts/schemas/auth-token.yaml (filenames differ; $id reassignment is forbidden)
INFO: contracts/schemas/user.yaml replaces contracts/schemas/user.yaml at merge ($id "urn:specify:schemas/user"); shape diff documented in alignment report
```

### Check 4 — Cross-format consumer compatibility

Every schema in `$CHANGE_SCHEMAS` is potentially referenced by an existing OpenAPI or AsyncAPI binding in the baseline. Changing the schema can silently break those bindings — and, transitively, every downstream consumer that generates code from them.

Resolution scope:

- **Producers of `$ref`** are the binding files in `$BASELINE_CONTRACTS/http/` and `$BASELINE_CONTRACTS/messages/`. The verifier inspects each binding's `$ref` values to discover which schemas it consumes.
- **Producers of `$ref`** in the slice directory (`$CHANGE_CONTRACTS/http/`, `$CHANGE_CONTRACTS/messages/`) are also inspected — a mixed-format change may be authoring its own bindings concurrently.

Algorithm:

1. **Build the consumer graph.** For each schema file `<name>.yaml` in `$CHANGE_SCHEMAS`, scan baseline and change-local bindings for `$ref` values that resolve to it. Record the list of consumers per schema.
2. **For each schema with at least one baseline consumer**, diff the delta schema against the baseline schema (both at `<name>.yaml`) and classify each property-level change:

| `change-kind` | Severity | Description |
|---|---|---|
| `removed-field` | `WARN` | A property the baseline schema defined is absent in the delta. Every binding that exposes this field will produce smaller payloads; consumer code reading it will see `undefined`. |
| `required-field-added` | `WARN` | A property became `required` in the delta. Every binding accepting this schema as a request body will reject prior consumer requests. |
| `type-narrowed` | `WARN` | A property's `type`, `format`, `enum`, `pattern`, or numeric range narrowed. Consumer values that were valid before may now be rejected. |
| `enum-value-removed` | `WARN` | A value disappeared from a property's `enum` array. Consumers emitting that value will be rejected. |
| `additional-properties-tightened` | `WARN` | The schema flipped from `additionalProperties: true` (or absent) to `additionalProperties: false`. Consumers passing extra fields will be rejected. |
| `optional-field-added` | (no warning) | Backwards-compatible additive change. |
| `enum-value-added` | (no warning) | Backwards-compatible additive change. |
| `description-changed` | (no warning) | Behavioural docstring drift; not a wire change. |

3. **For each schema with no consumers**, skip Check 4 — there is no binding-side risk surface inside the slice. The compatibility risk lives entirely in `cross-project` mode (downstream projects may have their own consumers).

Report format:

```
WARN: contracts/schemas/user.yaml — removed property `email`; baseline binding contracts/http/user-api.yaml exposes it on GET /users/{user_id} response (REQ-007)
WARN: contracts/schemas/order.yaml — added required property `currency`; baseline binding contracts/messages/order-events.yaml uses it as message payload (channel `order.placed`)
WARN: contracts/schemas/error-response.yaml — narrowed enum on `code` field (removed value `RATE_LIMITED`); 4 baseline bindings reference this schema
```

When the slice has **no specs**, Check 4 still runs — the binding consumers exist independently of the spec scenarios.

## Single-mode algorithm

1. **Determine scope.**
   - `$CHANGE_SCHEMAS`, `$BASELINE_SCHEMAS`.
   - `$CHANGE_CONTRACTS/http/`, `$CHANGE_CONTRACTS/messages/`, `$BASELINE_CONTRACTS/http/`, `$BASELINE_CONTRACTS/messages/` (for Check 4 consumer discovery).
   - If `$CHANGE_SCHEMAS` is empty or absent, report all checks as passed and stop.
2. **Run Check 1** (`$ref` resolution) on every `.yaml` file in `$CHANGE_SCHEMAS`.
3. **Run Check 2** (metadata completeness) on every `.yaml` file in `$CHANGE_SCHEMAS`.
4. **Run Check 3** (duplicate-`$id` detection) across `$CHANGE_SCHEMAS` ∪ `$BASELINE_SCHEMAS`.
5. **Run Check 4** (cross-format consumer compatibility) by walking the consumer graph and diffing delta schemas against their baseline equivalents.
6. **Collect findings** and produce the markdown validation report.

## Single-mode output format

When issues are found:

```markdown
## Validation Report (Schemas)

### $ref Resolution
- ✗ contracts/schemas/order.yaml — $ref "missing-type.yaml" does not resolve
- ✓ 18 of 19 $ref pointers resolve

### Metadata Completeness
- ✗ contracts/schemas/user-registration.yaml — missing "description"
- ⚠ contracts/schemas/payment.yaml — "$schema" is Draft 7; expected Draft 2020-12
- ✓ 5 of 7 schemas have complete metadata

### Duplicate $id
- ✓ All $id values unique within change ∪ baseline

### Cross-format Consumer Compatibility
- ⚠ contracts/schemas/user.yaml — removed property `email`; baseline binding contracts/http/user-api.yaml exposes it on GET /users/{user_id}
- ✓ 6 of 7 changed schemas are backwards-compatible

### Summary
- **Checks passed:** 1 of 4
- **Issues found:** 3 (1 fail, 2 warn)
```

When all checks pass:

```markdown
## Validation Report (Schemas)

All checks passed (19 $ref pointers, 7 schemas, 0 $id collisions, 0 backwards-incompatible changes).
```

`single` mode preserves its existing exit semantics: zero on clean reports, non-zero on read errors.

## Cross-project mode

`cross-project` mode runs **after** a producer's contract change merges. The execute driver (`/change:execute`) calls the verifier once per `(producer-schema, consumer-workspace)` pair to detect breaking changes that propagate downstream.

The mode is **non-fatal**: cross-project warnings never stop the execute loop. The driver records each warning on the merging change's `journal.yaml` (via `specify slice journal append`) and renders a warning block in the merge transcript so the operator can triage.

### Compatibility checks

For each `(producer-schema, consumer-workspace)` pair, compare the producer's updated schema against the consumer's last-known view of the same schema. Resolve the consumer's view in this order:

1. `$CONSUMER_CONTRACTS/schemas/<filename>` — the consumer's materialised baseline. This is what `specify workspace sync` populates from the central `contracts/`.
2. If absent, search `$CONSUMER_CONTRACTS/imports/` for a file with the same logical name (legacy import path used by some consumer clones).
3. If still absent, the consumer has no prior view — emit a single `consumer-has-no-baseline` finding and stop.

When both files are present, classify each delta into a `change-kind` (same vocabulary as Check 4 above):

| `change-kind` | Severity | Description |
|---|---|---|
| `removed-field` | `warning` | A property the consumer's view defined is no longer in the producer's schema. |
| `required-field-added` | `warning` | A property became `required`. Consumer requests built from the prior shape may be rejected by any binding consuming this schema. |
| `type-narrowed` | `warning` | A property's `type` / `format` / `enum` / range became stricter. |
| `enum-value-removed` | `warning` | A value disappeared from a property's `enum`. |
| `additional-properties-tightened` | `warning` | `additionalProperties` flipped from permissive to `false`. |
| `consumer-has-no-baseline` | `info` | The consumer's workspace clone has no prior view (first-time materialisation). No incompatibility — the consumer picks up the new shape on its next `workspace sync`. |
| `format-mismatch` | `warning` | The consumer's file at the same path is **not** a JSON Schema document (it has `openapi:` or `asyncapi:`). Emit and exit zero — the wrong verifier was invoked. |

Findings outside this table (additive optional fields, wider enums, additive `examples`) are **not warnings** — they are backwards-compatible.

### Cross-project algorithm

1. **Read inputs.**
   - Producer schema: parse `$PRODUCER_CONTRACT`. On read failure, exit non-zero with a `cannot-read-producer-contract` diagnostic.
   - Consumer view: locate the consumer's matching file under `$CONSUMER_CONTRACTS` using the resolution order above.
   - If no consumer view is found, emit one `consumer-has-no-baseline` finding and skip steps 2–4.
2. **Confirm format.** Read the top-level keys; the file must be a JSON Schema document (carries `$schema:` or `$id:` or has `properties:` at root). If it has `openapi:` or `asyncapi:` instead, emit a `format-mismatch` finding and exit zero.
3. **Run schema compatibility checks.** Walk `properties`, `required`, `enum`, `type`, `format`, `pattern`, range constraints, and `additionalProperties`. Classify each delta into a `change-kind` from the table above.
4. **Collect findings.** Each finding records `{ severity, contract, change-kind, locator, details }`.
5. **Emit the structured YAML report** (see below).

The verifier does not walk the consumer's spec, OpenAPI bindings, AsyncAPI bindings, or source code in this mode — that level of analysis is out of scope and would re-couple the verifier to the consumer's implementation. The conservative output is "the schema shape changed in a backwards-incompatible direction; the operator should triage."

## Cross-project output format

```yaml
mode: cross-project
producer:
  contract: contracts/schemas/user.yaml
consumer:
  workspace: .specify/workspace/mobile/
findings:
  - severity: warning
    contract: contracts/schemas/user.yaml
    change-kind: removed-field
    locator: properties.email
    details: >
      The producer's update removes the `email` property. The consumer's
      last-known view defines this property; consumer code reading it
      will receive `undefined` after the next workspace sync.
  - severity: warning
    contract: contracts/schemas/user.yaml
    change-kind: required-field-added
    locator: required
    details: >
      Producer adds `phone_number` to the required field list. Consumer
      payloads built from the prior shape will be rejected by any
      binding that uses this schema as a request body.
summary:
  total-findings: 2
  warnings: 2
  errors: 0
```

When no findings are produced (consumer's view matches the producer's update, or consumer has no prior view):

```yaml
mode: cross-project
producer:
  contract: contracts/schemas/user.yaml
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

`locator` strings are dot-separated paths into the schema document, following JSON Schema's natural traversal order:

- Top-level field changes: `properties.<field>`
- Nested object changes: `properties.<field>.properties.<nested>`
- Required list changes: `required`
- Enum value changes: `properties.<field>.enum`
- Range constraint changes: `properties.<field>.minimum` (or `maximum`, `exclusiveMinimum`, etc.)
- File-local sub-type changes: `$defs.<name>.properties.<field>`

Locators are emitted for human triage, not parsed.

### Cross-project exit semantics

`cross-project` mode exits **0** even when warnings are present. The mode is non-fatal by design (RFC-9 §3B). Exit non-zero only when:

- `$PRODUCER_CONTRACT` cannot be read (`cannot-read-producer-contract`).
- `$CONSUMER_WORKSPACE` cannot be reached (e.g. permission denied).
- The producer's schema is malformed YAML and cannot be parsed (`producer-contract-malformed`).

## Edge cases

### `single` mode

| Scenario | Behavior |
|---|---|
| Change directory has no `contracts/schemas/` | Pass — nothing to verify. |
| Baseline has schemas but change does not | Pass — verifier only checks change-level artefacts. |
| `$ref` target exists in baseline but not in change | Pass — baseline is a valid resolution target. |
| `$ref` target exists in change but not in baseline | Pass — change-level schemas are valid resolution targets. |
| Change adds a schema with no consumers in either change or baseline bindings | Skip Check 4 for that schema (no consumer surface inside the slice). |
| Schema declares `additionalProperties` neither true nor false | Pass — the field is genuinely optional. Authors default to `false`; importers preserve absence. |
| File-local `$defs` entry referenced only inside its parent | Pass — file-local sub-types are valid. |
| File contains a `$schema` URI for an older draft | Emit `WARN` recommending importer normalisation; do not fail. |

### `cross-project` mode

| Scenario | Behavior |
|---|---|
| `$CONSUMER_CONTRACTS/schemas/` does not exist (consumer never sync'd) | Emit `consumer-has-no-baseline` finding (severity `info`); exit 0. |
| Consumer's view matches the producer's update byte-for-byte | Empty `findings`; exit 0. |
| `$PRODUCER_CONTRACT` cannot be read | Exit non-zero with `cannot-read-producer-contract`. |
| Producer schema is malformed YAML | Exit non-zero with `producer-contract-malformed`. |
| Format mismatch (consumer file at the same path is OpenAPI / AsyncAPI) | Emit `format-mismatch` finding (severity `warning`); exit 0. |
| Consumer view contains additive properties the producer never defined | Pass silently — additive fields are the consumer's prerogative. |
| Consumer view uses an older draft URI than the producer | Treat as compatible — draft difference alone is not a breaking change. |

## Guardrails

- **Read-only.** Never create, modify, or delete files. Both modes share this contract.
- Report every issue with the file path and a description of the problem.
- Use `WARN` rather than `FAIL` (in `single` mode) when classification is ambiguous, e.g. when a schema is referenced by no bindings but might be shared vocabulary the spec brief just hasn't bound yet.
- Do not attempt to fix issues — report them. Repair belongs to the author or importer sibling.
- **Cross-project warnings are non-fatal.** Always exit 0 in `cross-project` mode unless the input cannot be read or parsed. The execute driver decides whether to halt; the verifier only reports.
- Do not walk consumer source code, specs, or generated bindings in `cross-project` mode. The consumer-side analysis stops at the schema file the consumer's workspace clone holds.

## Verification checklist

### `single` mode

Before completing the run:

- [ ] All `.yaml` files in `$CHANGE_SCHEMAS` scanned for `$ref` resolution.
- [ ] All `.yaml` files in `$CHANGE_SCHEMAS` checked for `$schema`, `$id`, `title`, `description`, `type`.
- [ ] All `$id` values across `$CHANGE_SCHEMAS` ∪ `$BASELINE_SCHEMAS` checked for duplicates.
- [ ] Cross-format consumer compatibility checked for every change-touched schema with at least one baseline or change-local binding consumer.
- [ ] Validation report produced with per-check results and summary.
- [ ] No files created or modified.

### `cross-project` mode

Before completing the run:

- [ ] `$PRODUCER_CONTRACT` parsed successfully (or reported as `cannot-read-producer-contract`).
- [ ] Consumer's matching view located under `$CONSUMER_CONTRACTS/schemas/` (or reported as `consumer-has-no-baseline`).
- [ ] Format confirmed as JSON Schema (or reported as `format-mismatch`).
- [ ] Schema compatibility checks ran (properties, required, enums, types, ranges, `additionalProperties`).
- [ ] Each delta classified into a known `change-kind`.
- [ ] YAML report emitted with `mode`, `producer`, `consumer`, `findings`, and `summary`.
- [ ] Exit status reflects exit-semantics rules (0 with findings; non-zero only on read failure).
- [ ] No files created or modified.

## See also

- [`../../references/json-schema-conventions.md`](../../references/json-schema-conventions.md) — Draft 2020-12 conventions, `$id` URN format, metadata rules.
- [`../../references/artifact-structure.md`](../../references/artifact-structure.md) — directory layout for the slice-local delta and the baseline.
- [`../../references/report-shape.md`](../../references/report-shape.md) — single-mode markdown report and cross-project YAML report formats this verifier emits, including severity levels and locator format.
- [`../../references/cross-project-compatibility.md`](../../references/cross-project-compatibility.md) — `change-kind` enumeration (used by both Check 4 and `--mode cross-project`), consumer-view resolution, breaking-change classification policy.
- [`author.md`](./author.md) — sibling for spec-driven authoring.
- [`importer.md`](./importer.md) — sibling for normalising external documents.
