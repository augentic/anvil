# AsyncAPI — Author

> **When to read this.** Read this when authoring or extending the AsyncAPI document for a Specify change — i.e. when the contracts adapter build brief during `/spec:build` selects the author intent, or an operator wants to add new evented interactions to the platform's messaging baseline. Skip this file when importing an external document (use [`importer.md`](./importer.md)) or when verifying an existing artefact (use [`verifier.md`](./verifier.md)).

## Inputs

```text
$SLICE_DIR     = .specify/slices/<slice-name>
$SPECS_DIR      = $SLICE_DIR/specs
$CONTRACTS_DIR  = $SLICE_DIR/contracts
$BASELINE_DIR   = contracts
```

## Authority hierarchy

When sources conflict, follow this strict precedence:

1. **This file** — author rules and hard constraints for AsyncAPI documents.
2. **Specify artefacts** (specs) — behavioural requirements drive the channels and operations.
3. **Format conventions** — [`../../references/asyncapi-conventions.md`](../../references/asyncapi-conventions.md), [`../../references/json-schema-conventions.md`](../../references/json-schema-conventions.md).
4. **Baseline contracts** (`contracts/messages/`) — existing platform vocabulary; never overwrite silently.
5. **LLM inference** — prohibited for unknowns; mark with `[unknown]` and surface in the alignment report.

If the specs and baseline disagree on a shape (e.g. the spec asserts a `partition_key` header that the baseline message omits), surface the mismatch in the alignment report's Warnings section. Never silently overwrite baseline channels or messages to match the specs — a human reviewer decides.

## The 4-step author algorithm

The author runs four steps end-to-end whenever the contracts adapter build brief asks for messaging coverage. Each step is a focused, independently-checkable phase; downstream steps assume the upstream output is well-formed.

### Step 1 — Read the baseline

Build an inventory of `$BASELINE_DIR/messages/`:

| Source | Extract |
|---|---|
| `messages/*.yaml` | AsyncAPI `channels` (key + `address`), `operations` (key + `action` + channel ref), `components.messages` (name + `contentType` + `payload` `$ref` + `headers`) |

For each channel, record:

- **Identity** — `(channelKey, address)` plus the operations that reference it.
- **Operations** — for each operation, `(operationId, action)` where `action` is `send` or `receive`.
- **Messages** — message name, payload schema `$ref`, header shape if any.
- **File** — relative path from root `contracts/`.

When `$BASELINE_DIR/messages/` is empty or absent, the baseline is empty — every spec interaction becomes delta. Record an empty inventory and proceed.

### Step 2 — Map specs to channels and operations

Read every `*.md` file under `$SPECS_DIR/` and harvest evented requirements. A spec scenario maps to a `channels.<name>` + `operations.<name>` pair when it describes:

- A topic, channel, or event name (`order.placed`, `user.registered`, `payment.received`).
- A message payload (field names, types, required-ness).
- Pub/sub direction (`publishes …`, `emits …`, `consumes …`, `subscribes to …`, `receives …`).
- Event triggers (`WHEN an order is confirmed, THEN publish …`).
- Optional message headers (correlation IDs, partition keys, trace context).

Build a structured list of **spec interactions**, one per channel:

```text
- channel:
    key: orderPlaced
    address: order.placed
  operations:
    - id: publishOrderPlaced
      action: send
    - id: consumeOrderPlaced
      action: receive
  message:
    name: OrderPlacedMessage
    payload: OrderPlaced (fields: order_id, customer_id, line_items[], total)
    headers: { correlation_id: string }
  source: specs/order-flow.md REQ-021
```

When a spec scenario references a payload type by name (e.g. "publish an `OrderPlaced` event"), check whether the type is defined in this slice's `$CONTRACTS_DIR/schemas/` or already in `$BASELINE_DIR/schemas/`. The schema is owned by the json-schema format skill — your job is only to wire the `$ref` correctly.

When the slice has **no specs** (e.g. an importer-only change followed by a normalisation pass), skip steps 2–4 and route to [`importer.md`](./importer.md).

### Step 3 — Compute the minimal delta

Compare each spec interaction from Step 2 against the baseline inventory from Step 1. Classify into one of three buckets:

#### Already covered

The baseline already defines a matching channel by `address` (the wire-level identity, not the camelCase key). Verify alignment:

- Channel `address` matches the spec's topic name.
- For each spec-asserted operation, the baseline has an operation with the same `action` (`send` or `receive`) targeting that channel.
- Message payload `$ref` resolves to a schema whose properties cover the fields the spec references.
- Message headers cover any spec-asserted envelope fields.

If alignment fails, record a warning with `{ baseline_file, spec_requirement_id, discrepancy }` for the alignment report. **Do not regenerate covered channels** and **do not overwrite the baseline** — flag the mismatch and let a human resolve it.

#### New or modified

The spec describes a channel, operation, or message that is absent from the baseline, or a baseline file needs new channels on the same event domain. Add to the AsyncAPI delta:

- New `channels.<key>` entries with `address` derived from the spec's topic.
- New `operations.<id>` entries with `action: send` or `action: receive` per the spec's pub/sub direction.
- New `components.messages.<Name>` entries when the spec introduces a message the baseline lacks.
- New message headers when the spec asserts an envelope field.

When extending an existing event domain (e.g. adding `user.deleted` to `user-events.yaml` which already defines `user.registered` and `user.updated`), the delta file must contain **both the existing channels/operations/messages and the new ones**. Merge is opaque file replacement: the slice-level file replaces the baseline file wholesale, so omitting existing entries would silently delete them.

#### Normalisation

The baseline file lacks Specify-required metadata (`info.title`, `info.version`, `info.description`) or violates a convention (e.g. an inline payload that should have been decomposed). Propose a normalisation delta that adds the metadata or decomposes the payload without changing channel addresses or operation semantics. Surface as a separate section in the alignment report.

### Step 4 — Generate or update AsyncAPI files

For every event domain in the delta, write a file under `$CONTRACTS_DIR/messages/<domain>-events.yaml`. File naming follows kebab-case: `order-events.yaml`, `user-events.yaml`, `notification-events.yaml`. Group related channels into one file by domain (e.g. all user lifecycle events live together), never per-channel or per-operation.

Required structure (minimal):

```yaml
asyncapi: "3.0.0"
info:
  title: Order Events
  version: "1.0.0"
  description: Events published during order processing — placement, fulfillment, and cancellation.
channels:
  orderPlaced:
    address: order.placed
    messages:
      orderPlacedMessage:
        $ref: "#/components/messages/OrderPlacedMessage"
operations:
  publishOrderPlaced:
    action: send
    channel:
      $ref: "#/channels/orderPlaced"
    summary: Publish an order-placed event when a new order is confirmed.
    messages:
      - $ref: "#/channels/orderPlaced/messages/orderPlacedMessage"
components:
  messages:
    OrderPlacedMessage:
      name: OrderPlacedMessage
      contentType: application/json
      payload:
        $ref: "../schemas/order-placed.yaml"
```

The full structural rules — channel key conventions, address dot-notation, operation `action` values, message naming, content types, header conventions, and the contract scope boundary — live in [`../../references/asyncapi-conventions.md`](../../references/asyncapi-conventions.md). Read it before authoring; the rules below are AsyncAPI deltas and rules-of-thumb that complement the convention reference.

## Channel and operation modelling

The 3.0 model separates *what the channel is* from *who does what with it*. Author both halves explicitly:

- **Channel** (under `channels`) — declares an `address` (the wire-level topic name) and a `messages` map listing the messages that flow on the channel. The YAML key is camelCase (`orderPlaced`); the `address` is dot-notation (`order.placed`).
- **Operation** (under `operations`) — declares an `action` (`send` or `receive`), a `$ref` to the channel, and a `messages` array referencing the messages this side handles. One channel may have a `send` operation, a `receive` operation, or both — depending on whether the contract scope owns producer, consumer, or both roles.
- **Message** (under `components/messages`) — declares the wire shape (`name`, `contentType`, `payload` `$ref`, optional `headers`). Channels and operations reference messages via `$ref`, never by inlining.

Naming patterns:

| Element | Convention | Example |
|---|---|---|
| Channel key | camelCase, matches the event concept | `orderPlaced`, `userRegistered`, `paymentReceived` |
| Channel address | dot-notation, lower-case segments, past tense for events / present tense for commands | `order.placed`, `user.registered`, `notification.send` |
| Operation key | camelCase verb prefix + event concept | `publishOrderPlaced`, `consumeOrderPlaced`, `sendUserRegistered`, `receivePaymentReceived` |
| Message name | PascalCase + `Message` suffix | `OrderPlacedMessage`, `UserRegisteredMessage` |

Operation names must be unique across all AsyncAPI files in the contract tree.

## Schema reuse and `$ref` discipline

Shared payload schemas live in `contracts/schemas/` and are owned by the `json-schema` sub-flow. The author of an AsyncAPI file does **not** create or edit schema files — it only references them.

- **Always `$ref`** message payloads to `../schemas/<type>.yaml`. The `$ref` lives on the message's `payload` field inside `components/messages`.
- **Never inline** a domain type. If the spec mentions a new payload type, route the schema work to the `json-schema` sub-flow (the contracts adapter build brief runs `json-schema` first per the cross-format ordering rule).
- **`$ref` resolution scope.** All payload `$ref` paths must resolve either to `$CONTRACTS_DIR/schemas/` (this slice's delta) or `$BASELINE_DIR/schemas/` (the platform baseline). The verifier flags any `$ref` that does not resolve.
- **Headers stay inline.** Message headers describe the envelope (correlation IDs, partition keys, trace context) and are typically a small map of primitive types. Inline them as a `headers` object on the message — do not extract them to `../schemas/`. The body is the payload; the envelope is not.
- **Internal `$ref`s for channel→message and operation→channel** stay on the `#/components/messages/...` and `#/channels/...` form — those are document-internal pointers, not cross-file schema references.

## Baseline-delta computation rules

AsyncAPI deltas fall into three categories — every entry in the delta belongs to exactly one:

| Category | Trigger | Effect on the delta file |
|---|---|---|
| **Channels / operations / messages added** | Address, operationId, or message name not in the baseline | New entry under the corresponding top-level block |
| **Modified** | Baseline entry, but the spec asserts a new field, header, or operation direction | Edit the baseline entry in-place inside the delta file (preserving every other property byte-for-byte) and surface the diff in the alignment report |
| **Removed** | Baseline entry that no spec scenario references and the slice explicitly deprecates it | **Out of scope.** AsyncAPI deltas have no remove semantics — removal is a manual baseline edit. Surface as a warning in the alignment report so a human can act |

Computation rules applied at file scope:

1. **One file per event domain.** Always read the matching baseline file first. The delta file replaces it wholesale at merge time, so it must contain every existing channel, operation, and message alongside the new ones.
2. **`info.version` MUST parse as SemVer (contract identity/version validation).** New top-level AsyncAPI documents MUST set `info.version` to a value that parses per [semver.org](https://semver.org), including optional prerelease labels (`1.0.0-draft.1`). Do not bump the baseline's `info.version` automatically — version policy is a platform decision, not an authoring decision. If the slice requires a version bump, the contracts adapter build brief flags it for human review. The verifier sibling enforces SemVer in single mode (Check 4), and `specify tool run contract` enforces it again at merge time on the baseline (the contracts adapter merge contract); a non-SemVer value is a hard validation failure at both gates.
3. **`info.x-specify-id` rename-stable identifier (contract identity/version validation).** SHOULD set `info.x-specify-id` on every new top-level AsyncAPI document to a kebab-case slug (typically the file stem; `^[a-z][a-z0-9-]*$`, ≤ 64 characters). The id is a hint that survives file moves and version bumps. MUST preserve any pre-existing `info.x-specify-id` when extending the baseline; MUST NOT change it across `info.version` bumps. Path-based references in `registry.yaml` remain canonical — the id is a rename-stable hint, not a substitute.
4. **Preserve channel addresses and operation keys verbatim.** When extending a baseline file, every existing `address` and `operationId` stays exactly as it is. Renaming an address breaks consumers; renaming an operation breaks tooling.
5. **Diff at the entry level.** When modifying an existing channel or message, change only the keys the spec asserts. Do not reformat or reorder unrelated keys — opaque file replacement means a re-ordered file looks like a wholesale rewrite to reviewers.

## Producers and consumers

A contract may declare both `send` and `receive` operations for the same channel when the producer and consumer roles both fall inside the contract's scope:

```yaml
operations:
  publishOrderPlaced:
    action: send
    channel: { $ref: "#/channels/orderPlaced" }
    summary: Publish an order-placed event when a new order is confirmed.
    messages:
      - $ref: "#/channels/orderPlaced/messages/orderPlacedMessage"

  consumeOrderPlaced:
    action: receive
    channel: { $ref: "#/channels/orderPlaced" }
    summary: Consume order-placed events for fulfillment processing.
    messages:
      - $ref: "#/channels/orderPlaced/messages/orderPlacedMessage"
```

When the contract scopes only one side, declare only the operation for that side. The verifier does not require both — but if the spec asserts a producer and a consumer, both must appear.

## Message headers

When the spec describes message envelope metadata (correlation IDs, partition keys, trace context, idempotency keys), declare them on the message's `headers` field as an inline object schema:

```yaml
components:
  messages:
    OrderPlacedMessage:
      name: OrderPlacedMessage
      contentType: application/json
      payload:
        $ref: "../schemas/order-placed.yaml"
      headers:
        type: object
        properties:
          correlation_id:
            type: string
            description: Distributed tracing correlation ID.
          partition_key:
            type: string
            description: Partition key for ordered delivery.
        required:
          - correlation_id
```

Headers describe the message envelope, not the payload. Payload shape belongs in the schema file under `../schemas/`.

## Alignment report

Every author run produces an alignment report alongside the delta files. The report is the primary output for the contracts adapter build brief — the YAML files are the artefact, but the report is how the brief decides whether the slice can proceed.

```markdown
## Alignment Report (Messaging)

### Coverage
- **Covered by baseline:** N channels (M with alignment warnings)
- **New (delta produced):** N channels, N operations, N messages
- **Normalisation:** N files updated with metadata

### Alignment Warnings
- `order.placed` channel: message payload missing `currency` field expected by spec scenario REQ-012
- `user.registered` channel: spec asserts a `consume` operation but baseline only declares `send`

### Generated Delta
- `contracts/messages/order-events.yaml` (updated — added `order.cancelled` channel)
- `contracts/messages/notification-events.yaml` (new)

### Normalisation
- `contracts/messages/order-events.yaml` (added `info.description`)
```

Report semantics:

- **Zero delta with zero warnings** is the expected outcome for an implementation slice in a contract-first workflow — specs already align with the pre-existing AsyncAPI document.
- **Warnings require human review.** The author never resolves spec-vs-baseline mismatches automatically.
- **A non-empty delta** is normal for contract-only changes and for spec-first changes where the baseline is empty.

After producing the report, run [`verifier.md`](./verifier.md) in `single` mode against `$SLICE_DIR` to validate `$ref` resolution, schema metadata, and binding coverage before declaring the artefact ready.

## Edge cases

| Scenario | Handling |
|---|---|
| Spec references a payload type not yet authored | Mark `[unknown]` in the report; the json-schema skill (called first by the contracts adapter build brief) should have produced the schema. If it did not, halt and surface the gap. |
| Spec describes a command pattern (`order.cancel`) rather than an event | Use present-tense address (`order.cancel`) and pair with `action: send` from the requester and `action: receive` from the handler — see the conventions reference. |
| Two specs claim the same channel address with different message shapes | Surface the conflict as a warning; do not write a delta until the specs are reconciled. |
| Baseline channel uses inline payload (legacy from a manual import) | Do not propagate the inline form into the delta. Run [`importer.md`](./importer.md) on the baseline file first, then re-author. |
| Spec describes broker-specific bindings (Kafka partition strategy, RabbitMQ queue policy) | Out of scope — those belong in `design.md`. Capture only structural shape (channel, message, headers) in the contract. See [`../../references/asyncapi-conventions.md`](../../references/asyncapi-conventions.md) §Scope Boundary. |
| Spec describes a non-JSON wire format (Avro, Protobuf) | Set `contentType` accordingly (`application/avro`, `application/protobuf`) and reference the payload schema as usual; the json-schema skill produces the schema regardless of wire format. |

## Verification checklist

Before declaring the author run complete:

- [ ] Every spec-described evented interaction maps to a `channel` + `operation` pair in either the baseline or the delta.
- [ ] All payload `$ref` pointers in the delta resolve into `$CONTRACTS_DIR/schemas/` or `$BASELINE_DIR/schemas/`.
- [ ] No payload schemas are inlined; every `payload` uses `$ref`.
- [ ] When extending a baseline file, every existing channel, operation, and message is preserved verbatim alongside the new ones.
- [ ] Channel keys are camelCase; addresses are dot-notation; operation keys are unique across the contract tree.
- [ ] Alignment report enumerates coverage, warnings, generated delta files, and normalisation entries.
- [ ] [`verifier.md`](./verifier.md) (single mode) ran clean against `$SLICE_DIR`.

## See also

- [`asyncapi-conventions`](../../references/asyncapi-conventions.md) — file structure, channel and operation conventions, message definitions, header rules.
- [`artifact-structure`](../../references/artifact-structure.md) — directory layout for the slice-local delta and the baseline.
- [`baseline-vs-delta`](../../references/baseline-vs-delta.md) — cross-format rules for the three authorship patterns, the already-covered / new-or-modified / normalisation classification, and the opaque-file-replacement merge contract.
- [`report-shape`](../../references/report-shape.md) — markdown shape for the alignment report produced by this author path.
- [`json-schema-conventions`](../../references/json-schema-conventions.md) — schema files referenced by the AsyncAPI document (owned by the `json-schema` sub-flow).
- [`importer.md`](./importer.md) — sibling for normalising external AsyncAPI documents.
- [`verifier.md`](./verifier.md) — sibling for validating the authored output.
