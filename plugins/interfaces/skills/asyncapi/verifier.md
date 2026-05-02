# AsyncAPI — Verifier

> **When to read this.** Read this when verifying an AsyncAPI artefact — invoked by the contracts schema build brief in `single` mode after the author or importer sibling produces output, by `/spec:execute` in `cross-project` mode after a producer's contract change merges (RFC-9 §3B), or directly by an operator running validation against an existing artefact. Skip this file when authoring (use [`author.md`](./author.md)) or normalising an external document (use [`importer.md`](./importer.md)).

The verifier is **read-only**. It MUST NOT generate, modify, or delete any files. Its sole output is a list of issues rendered as a validation report.

## Modes

The verifier accepts a `--mode {single, cross-project}` flag. The mode determines the report shape and the exit semantics.

| Mode | Caller | Trigger | Scope | Output |
|---|---|---|---|---|
| `single` (default) | contracts schema build brief in `/spec:build` | Post-author or post-import | One change's `contracts/messages/` inside one project | Markdown report for the verify-repair loop |
| `cross-project` | `/spec:execute` post-merge step | Producer-side merge of an AsyncAPI contract change | Compare merged AsyncAPI against each consumer's tier-2 workspace clone | Structured YAML report consumed by the execute driver |

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
$PRODUCER_CONTRACT  = <path-to-producer-contract>     # e.g. .specify/contracts/messages/order-events.yaml
$CONSUMER_WORKSPACE = <path-to-consumer-workspace>    # e.g. .specify/workspace/mobile/
$CONSUMER_CONTRACTS = $CONSUMER_WORKSPACE/.specify/contracts/
```

`$PRODUCER_CONTRACT` is a path relative to the initiating repo root (typically a file under `.specify/contracts/messages/` after a producer change merges). `$CONSUMER_WORKSPACE` is a tier-2 workspace clone — `specify workspace sync` materialises consumer clones at `.specify/workspace/<consumer-name>/`, and the consumer's view of central contracts lives at `$CONSUMER_CONTRACTS`.

## Prerequisites

### `single` mode

- The author or importer sibling has completed and produced artefacts under `$CHANGE_CONTRACTS/messages/`.
- `.specify/project.yaml` exists (Specify is initialised).

If `$CHANGE_CONTRACTS/messages/` does not exist or contains no files, report all checks as passed — there is nothing to verify.

### `cross-project` mode

- `$PRODUCER_CONTRACT` exists and is readable. Otherwise exit non-zero with a `cannot-read-producer-contract` diagnostic.
- `$CONSUMER_WORKSPACE` exists. If `$CONSUMER_CONTRACTS` is absent (the consumer has never sync'd), emit a single `consumer-has-no-baseline` finding (severity `info`) and exit zero.

## Single-mode checks

Three independent checks run against `$CHANGE_CONTRACTS/messages/` and the schemas it references. Run them in order; collect findings; produce a single markdown report at the end.

### Check 1 — `$ref` resolution

All `$ref` pointers in AsyncAPI files must resolve. Two resolution scopes apply depending on the kind of `$ref`:

- **Cross-file payload refs** (`$ref: "../schemas/<name>.yaml"`) — must resolve to a file in `$CHANGE_CONTRACTS/schemas/` **or** `$BASELINE_CONTRACTS/schemas/`.
- **In-document refs** (`$ref: "#/components/messages/..."`, `$ref: "#/channels/..."`, `$ref: "#/components/messageTraits/..."`, `$ref: "#/components/operationTraits/..."`) — must resolve to a sibling key inside the same AsyncAPI file.

For each `.yaml` file in `$CHANGE_CONTRACTS/messages/`:

1. Read the file and find every `$ref` value (channel→message, operation→channel, operation→message, message→payload, message→trait, operation→trait).
2. Classify each `$ref` as cross-file or in-document.
3. Resolve cross-file refs against `$CHANGE_CONTRACTS` and `$BASELINE_CONTRACTS`; resolve in-document refs against the same file.
4. Report any `$ref` that does not resolve.

Report format (one entry per failure):

```
FAIL: contracts/messages/order-events.yaml — $ref "../schemas/missing-type.yaml" does not resolve (checked change contracts/schemas/ and baseline .specify/contracts/schemas/)
FAIL: contracts/messages/order-events.yaml — $ref "#/components/messages/MissingMessage" does not resolve in-document
```

The verifier does not chase external URL `$ref`s; it flags them as `WARN` (cross-format URL refs are out of scope).

### Check 2 — Message and schema metadata

Every message in `components/messages` and every JSON Schema file referenced by an AsyncAPI message must have the required metadata fields.

#### Message metadata

| Field | Rule |
|---|---|
| `name` | Present and PascalCase, matching the message key. |
| `contentType` | Present. Default to `application/json` when the spec does not require otherwise; flag any other value for human review unless explicitly justified. |
| `payload` | Present and either an inline schema (flagged for normalisation) or a `$ref` to `../schemas/`. |

#### Payload schema metadata

For every JSON Schema file in `$CHANGE_CONTRACTS/schemas/` referenced by a message payload:

| Field | Rule |
|---|---|
| `$id` | Present and a valid URI (URN format: `urn:specify:schemas/<name>`). |
| `title` | Present and non-empty, matching the type name. |
| `description` | Present and non-empty. The placeholder `"[imported — description pending review]"` counts as present but **emits a `WARN`** to surface the gap for the verify-repair loop. |

Report format (one entry per failure):

```
FAIL: contracts/messages/order-events.yaml — message "OrderPlacedMessage" missing required field "contentType"
FAIL: contracts/schemas/order-placed.yaml — missing required field "$id"
WARN: contracts/schemas/order-cancelled.yaml — "description" is "[imported — description pending review]"; replace before merge
```

### Check 3 — Binding completeness

Every schema that appears as a top-level message payload in a spec scenario must have at least one AsyncAPI message that references it via `$ref`, and at least one operation that points at the channel carrying that message.

Resolution scope for the binding:

- `$CHANGE_CONTRACTS/messages/` — channels and operations added by this change.
- `$BASELINE_CONTRACTS/messages/` — channels and operations already in the platform baseline.

#### Determining spec-referenced schemas

Read the `*.md` files under `$CHANGE_SPECS` and identify schemas that the spec mentions as message payloads:

- Pub/sub event payloads (e.g. "publish an `OrderPlaced` event").
- Stream record payloads (e.g. "emit a `MetricRecord` to the metrics topic").
- Command payloads (e.g. "send a `CancelOrder` command").

Cross-reference these against the schema files in `$CHANGE_CONTRACTS/schemas/`, the messages in `$CHANGE_CONTRACTS/messages/` + `$BASELINE_CONTRACTS/messages/`, and the operations in those same files.

#### Shared vocabulary exemption

Shared vocabulary types that appear only as `$ref` targets inside other schemas are exempt — they are reusable building blocks, not standalone payloads. A schema qualifies as shared vocabulary if both:

1. It is referenced via `$ref` from within other schema files (not directly from message payload definitions), AND
2. It does not appear as a top-level message payload in any spec scenario.

Common examples: `error-response.yaml`, `pagination.yaml`, `audit-metadata.yaml`.

Report format:

```
FAIL: contracts/schemas/order-placed.yaml — appears as message payload in spec scenario REQ-021 but has no AsyncAPI message binding
WARN: contracts/schemas/order-event-metadata.yaml — appears in spec but has no protocol binding (may be shared vocabulary — verify intent)
FAIL: contracts/messages/order-events.yaml — channel "orderPlaced" has no operation (neither send nor receive)
```

Use `FAIL` when the schema is unambiguously a top-level message payload in a spec scenario, or when a channel has no operations at all. Use `WARN` when classification is ambiguous — the verify-repair loop surfaces the warning for human review.

When the change has **no specs**, skip Check 3 — there are no scenarios to cross-reference. Record this in the report so the brief knows the check was deliberately bypassed.

### Check 4 — Identity & version (RFC-12)

For every top-level AsyncAPI document under `$CHANGE_CONTRACTS/messages/` (root key `asyncapi:`), enforce the RFC-12 §Validation rules:

1. **`info.version` MUST parse as SemVer.** Per [semver.org](https://semver.org), including optional prerelease labels (`1.0.0-draft.1`). Missing, non-string, or non-SemVer values are `FAIL`.
2. **`info.x-specify-id` (when present) MUST match `^[a-z][a-z0-9-]*$` and be ≤ 64 characters.** Format violations are `FAIL`.
3. **Within the change directory, `info.x-specify-id` values MUST be unique.** When two top-level AsyncAPI documents in `$CHANGE_CONTRACTS/messages/` declare the same id, both are `FAIL`.

The cross-repo uniqueness check (the same id declared by a top-level contract somewhere else under root `contracts/`) is **not** part of single mode — it is the CLI's job (`specify interface validate`), which runs after merge with the full baseline in scope. The skill only flags duplicates inside the change to keep the verifier deterministic and self-contained.

Report format (one entry per failure):

```
FAIL: contracts/messages/order-events.yaml — info.version `2024-01-15` is not valid SemVer
FAIL: contracts/messages/billing-events.yaml — info.x-specify-id `Billing-Events` must match `^[a-z][a-z0-9-]*$` and be ≤ 64 characters
FAIL: contracts/messages/admin-events.yaml — info.x-specify-id `shared` is also declared by contracts/messages/legacy-events.yaml in this change
```

## Single-mode algorithm

1. **Determine scope.**
   - `$CHANGE_CONTRACTS/messages/`, `$CHANGE_CONTRACTS/schemas/`.
   - `$BASELINE_CONTRACTS/messages/`, `$BASELINE_CONTRACTS/schemas/`.
   - `$CHANGE_SPECS/`.
   - If `$CHANGE_CONTRACTS/messages/` is empty or absent, report all checks as passed and stop.
2. **Run Check 1** ($ref resolution) on every `.yaml` file in `$CHANGE_CONTRACTS/messages/`.
3. **Run Check 2** (message and schema metadata) on every message in `components/messages` and every payload schema in `$CHANGE_CONTRACTS/schemas/` that an AsyncAPI message references.
4. **Run Check 3** (binding completeness) by cross-referencing spec scenarios with messages and operations across change and baseline. Skip if no specs.
5. **Run Check 4** (identity & version) on every top-level AsyncAPI document in `$CHANGE_CONTRACTS/messages/`.
6. **Collect findings** and produce the markdown validation report.

## Single-mode output format

When issues are found:

```markdown
## Validation Report (Messaging)

### $ref Resolution
- ✗ contracts/messages/order-events.yaml — $ref "../schemas/missing-type.yaml" does not resolve
- ✓ 8 of 9 $ref pointers resolve

### Message & Schema Metadata
- ✗ contracts/messages/order-events.yaml — message "OrderCancelledMessage" missing "contentType"
- ✗ contracts/schemas/order-placed.yaml — "description" is empty
- ✓ 4 of 6 schemas have complete metadata

### Binding Completeness
- ✓ All spec-referenced payload schemas have AsyncAPI bindings

### Summary
- **Checks passed:** 1 of 3
- **Issues found:** 3
```

When all checks pass:

```markdown
## Validation Report (Messaging)

All checks passed (9 $ref pointers, 6 schemas, 4 channels, 5 operations verified).
```

`single` mode preserves its existing exit semantics: zero on clean reports, non-zero on read errors.

## Cross-project mode

`cross-project` mode runs **after** a producer's contract change merges. The execute driver (`/spec:execute`) calls the verifier once per `(producer-contract, consumer-workspace)` pair to detect breaking changes that would propagate downstream.

The mode is **non-fatal**: cross-project warnings never stop the execute loop. The driver records each warning to the merged change's `journal.yaml` (via `specify change journal append`) and renders a warning block in the merge transcript so the operator can triage.

### Compatibility checks

For each `(producer-contract, consumer-workspace)` pair, compare the producer's updated contract against the consumer's last-known view of the same contract. Resolve the consumer's view in this order:

1. `$CONSUMER_CONTRACTS/<relative-path>` — the consumer's materialised baseline at the matching path. This is what `specify workspace sync` populates from the central `.specify/contracts/`.
2. If absent, search `$CONSUMER_CONTRACTS/imports/` for a file with the same logical name (legacy import path used by some consumer clones).
3. If still absent, the consumer has no prior view — emit a single `consumer-has-no-baseline` finding and stop.

When both files are present, classify each delta into a `change-kind`:

| `change-kind` | Severity | Description |
|---|---|---|
| `removed-channel` | `warning` | A channel `address` (or its YAML key) defined in the consumer's view is gone from the producer's update. Consumer subscribers will receive nothing on that topic. |
| `removed-operation` | `warning` | An operation key defined in the consumer's view (e.g. `consumeOrderPlaced`) is missing. Consumer code that referenced the operation by id will break. |
| `removed-message` | `warning` | A message in `components/messages` defined in the consumer's view is gone. Channels that referenced the message and the consumer's deserializers will fail. |
| `removed-field` | `warning` | A property the consumer's view defined on a message payload is no longer present. Consumer code that reads the field will receive `undefined` after the next workspace sync. |
| `required-field-added` | `warning` | A new field is `required` in a message payload. Consumers producing the prior shape will be rejected by stricter validators (and producers consuming the prior shape will under-fill the schema). |
| `type-narrowed` | `warning` | A property's `type` (or `format`, `enum`, numeric range) became stricter on a payload field or message header. Consumer values that were valid before may now be rejected. |
| `action-flipped` | `warning` | An operation's `action` changed from `send` to `receive` or vice versa. Consumer code that produced or consumed messages will be wired the wrong way. |
| `content-type-changed` | `warning` | A message's `contentType` changed (e.g. `application/json` → `application/avro`). Consumer deserializers built for the prior content type will fail. |
| `consumer-has-no-baseline` | `info` | The consumer's workspace clone has no prior view of this contract (first-time materialisation). No incompatibility — the consumer will pick up the new shape on its next `workspace sync`. |

Findings outside this table (additive optional fields, new channels, new operations, additional messages) are **not warnings** — they are backwards-compatible and the consumer keeps working unchanged.

### Cross-project algorithm

1. **Read inputs.**
   - Producer contract: parse `$PRODUCER_CONTRACT`. On read failure, exit non-zero with a `cannot-read-producer-contract` diagnostic.
   - Consumer view: locate the consumer's matching file under `$CONSUMER_CONTRACTS` using the resolution order above.
   - If no consumer view is found, emit one `consumer-has-no-baseline` finding and skip steps 2–4.
2. **Confirm format.** Read the top-level keys; the file must have `asyncapi: "3.x"`. If it has `openapi:` or `$schema:` instead, emit a `format-mismatch` finding and exit zero — the wrong verifier was invoked.
3. **Run AsyncAPI compatibility checks.**
   - Walk `channels[*]` in the consumer's view; for each channel, locate the matching `address` in the producer's contract. Classify removals.
   - Walk `operations[*]` in the consumer's view; for each operation, locate the matching key in the producer's contract. Classify removals and `action` flips.
   - Walk `components.messages[*]`; for each message the consumer's view defined, check whether the producer still defines it and whether `contentType`, headers, and payload `$ref` shape are compatible.
   - Walk payload schemas reached through message `$ref`; classify removed properties, newly-required properties, and type narrowings on shared properties.
4. **Collect findings.** Each finding records `{ severity, contract, change-kind, locator, details }`.
5. **Emit the structured YAML report** (see [Cross-project output format](#cross-project-output-format)).

The verifier does not walk the consumer's spec or source code in this mode — that level of analysis is out of scope and would re-couple the verifier to the consumer's implementation. The conservative output is "the wire shape changed in a backwards-incompatible direction; the operator should triage."

## Cross-project output format

```yaml
mode: cross-project
producer:
  contract: contracts/messages/order-events.yaml
consumer:
  workspace: .specify/workspace/mobile/
findings:
  - severity: warning
    contract: contracts/messages/order-events.yaml
    change-kind: removed-field
    locator: components.messages.OrderPlacedMessage.payload.properties.currency
    details: >
      The producer's update removes the `currency` field from the
      OrderPlaced payload. The consumer's last-known view defines this
      field; consumer code that reads it will receive `undefined` after
      the next workspace sync.
  - severity: warning
    contract: contracts/messages/order-events.yaml
    change-kind: action-flipped
    locator: operations.consumeOrderPlaced.action
    details: >
      Producer flipped `consumeOrderPlaced.action` from `receive` to
      `send`. Consumer code wired as a subscriber will silently emit
      messages instead of consuming them.
summary:
  total-findings: 2
  warnings: 2
  errors: 0
```

When no findings are produced (the consumer's view matches the producer's update, or the consumer has no prior view):

```yaml
mode: cross-project
producer:
  contract: contracts/messages/order-events.yaml
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

`locator` strings are dot-separated paths into the contract document, following AsyncAPI's natural traversal order:

- Channel-scoped locators: `channels.<key>.address`, `channels.<key>.messages.<message-key>`.
- Operation-scoped locators: `operations.<key>.action`, `operations.<key>.channel`, `operations.<key>.messages`.
- Message-scoped locators: `components.messages.<MessageName>.contentType`, `components.messages.<MessageName>.headers.properties.<field>`, `components.messages.<MessageName>.payload.properties.<field>`.
- Required-field changes on payloads: `components.messages.<MessageName>.payload.required`.

Path segments containing dots (e.g. an address like `order.placed`) are kept verbatim — locators are emitted for human triage, not parsed.

### Cross-project exit semantics

`cross-project` mode exits **0** even when warnings are present. The mode is non-fatal by design (RFC-9 §3B). Exit non-zero only when:

- `$PRODUCER_CONTRACT` cannot be read (`cannot-read-producer-contract`).
- `$CONSUMER_WORKSPACE` cannot be reached (e.g. permission denied).
- The producer's contract is malformed and cannot be parsed (`producer-contract-malformed`).

## Edge cases

### `single` mode

| Scenario | Behavior |
|---|---|
| Change directory has no `contracts/messages/` | Pass — nothing to verify. |
| Baseline has messaging contracts but change does not | Pass — verifier only checks change-level artefacts. |
| `$ref` target exists in baseline but not in change | Pass — baseline is a valid resolution target. |
| `$ref` target exists in change but not in baseline | Pass — change-level schemas are valid resolution targets. |
| Mixed resolution: some targets in baseline, some in change | Pass — both directories are valid resolution scope. |
| No spec files in the change | Skip Check 3; record the skip in the report. |
| Schema referenced only via `$ref` from other schemas | Exempt from Check 3 (shared vocabulary). |
| Channel uses inline payload (legacy from a Layer-1 import) | `$ref` resolution still verified inside the document; emit `WARN` recommending importer normalisation. |
| Channel declared with no operations | Emit `FAIL` — every channel must have at least one `send` or `receive` operation. |

### `cross-project` mode

| Scenario | Behavior |
|---|---|
| `$CONSUMER_CONTRACTS` does not exist (consumer never sync'd) | Emit `consumer-has-no-baseline` finding (severity `info`); exit 0. |
| Consumer's view matches the producer's update byte-for-byte | Empty `findings`; exit 0. |
| `$PRODUCER_CONTRACT` cannot be read | Exit non-zero with `cannot-read-producer-contract`. |
| Producer contract is malformed YAML | Exit non-zero with `producer-contract-malformed`. |
| Format mismatch (consumer has OpenAPI / JSON Schema at the same path) | Emit `format-mismatch` finding (severity `warning`); exit 0. |
| Consumer view contains additive fields the producer never defined | Pass silently — additive fields are the consumer's prerogative. |
| Channel address renamed but the YAML key is the same | Treat as `removed-channel` (the consumer's wire identity is the address, not the key). |

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

- [ ] All `.yaml` files in `$CHANGE_CONTRACTS/messages/` scanned for `$ref` resolution (cross-file and in-document).
- [ ] All messages in `components/messages` checked for `name`, `contentType`, `payload`.
- [ ] All `.yaml` files in `$CHANGE_CONTRACTS/schemas/` referenced by AsyncAPI messages checked for `$id`, `title`, `description`.
- [ ] Every channel has at least one `send` or `receive` operation.
- [ ] Spec scenarios cross-referenced against AsyncAPI bindings (when specs exist).
- [ ] Shared vocabulary exemption applied correctly.
- [ ] Identity & version (Check 4) enforced on every top-level AsyncAPI document in `$CHANGE_CONTRACTS/messages/`: SemVer `info.version`, kebab-case + ≤64-char `info.x-specify-id` when present, in-change uniqueness on declared ids.
- [ ] Validation report produced with per-check results and summary.
- [ ] No files created or modified.

### `cross-project` mode

Before completing the run:

- [ ] `$PRODUCER_CONTRACT` parsed successfully (or reported as `cannot-read-producer-contract`).
- [ ] Consumer's matching view located under `$CONSUMER_CONTRACTS` (or reported as `consumer-has-no-baseline`).
- [ ] AsyncAPI compatibility checks ran (channels, operations, messages, payload schemas, headers, content types).
- [ ] Each delta classified into a known `change-kind`.
- [ ] YAML report emitted with `mode`, `producer`, `consumer`, `findings`, and `summary`.
- [ ] Exit status reflects exit-semantics rules (0 with findings; non-zero only on read failure).
- [ ] No files created or modified.

## See also

- [`../../references/asyncapi-conventions.md`](../../references/asyncapi-conventions.md) — AsyncAPI 3.0 structure rules.
- [`../../references/json-schema-conventions.md`](../../references/json-schema-conventions.md) — schema metadata rules.
- [`../../references/artifact-structure.md`](../../references/artifact-structure.md) — directory layout for the change-local delta and the baseline.
- [`../../references/report-shape.md`](../../references/report-shape.md) — single-mode markdown report and cross-project YAML report formats this verifier emits, including severity levels and locator format.
- [`../../references/cross-project-compatibility.md`](../../references/cross-project-compatibility.md) — `change-kind` enumeration, consumer-view resolution, breaking-change classification policy.
- [`author.md`](./author.md) — sibling for spec-driven authoring.
- [`importer.md`](./importer.md) — sibling for normalising external documents.
