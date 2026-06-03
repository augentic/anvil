# AsyncAPI — Importer

> **When to read this.** Read this when an operator supplies an external AsyncAPI document and the contracts adapter build brief needs the file normalised onto Specify conventions under a slice's `contracts/messages/` directory. Skip this file when authoring from a spec (use [`author.md`](./author.md)) or when verifying an existing artefact (use [`verifier.md`](./verifier.md)).

## Inputs

```text
$SLICE_DIR     = .specify/slices/<slice-name>
$CONTRACTS_DIR  = $SLICE_DIR/contracts
$BASELINE_DIR   = contracts
```

**Input** — external AsyncAPI files placed by the operator anywhere under `$CONTRACTS_DIR/`. Files may be `.yaml`, `.yml`, or `.json`.

**Output** — normalised AsyncAPI 3.0.0 files in `$CONTRACTS_DIR/messages/`, with inline payloads decomposed into `$CONTRACTS_DIR/schemas/` and Specify metadata injected. The input files are replaced in-place with their normalised equivalents; decomposed schemas are added as new files.

## Authority hierarchy

When sources conflict:

1. **This file** — import rules and hard constraints.
2. **Format conventions** — [`../../references/asyncapi-conventions.md`](../../references/asyncapi-conventions.md), [`../../references/json-schema-conventions.md`](../../references/json-schema-conventions.md).
3. **Source contract** — the external file being imported. Preserve every channel, operation, message, and binding it defines; never silently drop information.
4. **LLM inference** — prohibited for unknowns; mark unrecognised constructs with `[import — manual review required]` and surface in the import report.

## The 5-step import algorithm

The importer runs five sequential steps. Each step assumes its predecessors completed cleanly; if a step cannot make progress on a file, mark the file as `manual review required` and continue with the rest.

### Step 1 — Scan and detect format

Walk `$CONTRACTS_DIR/` for `.yaml`, `.yml`, and `.json` files. For each file, read the top-level keys and classify:

| Detection signal | Classification | Target |
|---|---|---|
| `asyncapi: "2.x.x"` | AsyncAPI 2.x | AsyncAPI 3.0 |
| `asyncapi: "3.0.x"` | AsyncAPI 3.0.x | No version conversion |
| `openapi:` (any version) | **Out of scope.** Route to the `openapi` importer sub-flow. |
| `swagger: "2.0"` | **Out of scope.** Route to the `openapi` importer sub-flow. |
| `$schema:` (without `openapi`/`asyncapi`/`swagger`) | **Out of scope.** Route to the `json-schema` importer sub-flow. |
| None of the above | Unrecognised. Skip and flag for manual review. |

The detection signal is the top-level `asyncapi:` key. AsyncAPI minor versions in the 2.x range (2.0.0 through 2.6.0) all upgrade through the same conversion rules; detection only needs to distinguish 2.x from 3.0.

JSON files (`.json`) are converted to YAML during this step: read the JSON, re-serialise as YAML with `.yaml` extension, then continue normalisation.

When the operator placed the file outside `$CONTRACTS_DIR/messages/` (e.g. directly in `$CONTRACTS_DIR/`), record the move target — the file will be relocated in Step 4.

### Step 2 — Upgrade AsyncAPI 2.x → 3.0

The 2.x → 3.0 upgrade is a significant structural rework. The primary change is the separation of operations from channel items.

#### Version field

```yaml
# AsyncAPI 2.x
asyncapi: "2.6.0"

# AsyncAPI 3.0
asyncapi: "3.0.0"
```

#### Channel restructuring

AsyncAPI 2.x nests `publish` and `subscribe` operations under channel items. AsyncAPI 3.0 separates them into distinct `channels` and `operations` top-level sections.

```yaml
# AsyncAPI 2.x
channels:
  user/registered:
    subscribe:
      operationId: onUserRegistered
      message:
        payload:
          $ref: "#/components/schemas/UserRegistered"
    publish:
      operationId: publishUserRegistered
      message:
        payload:
          $ref: "#/components/schemas/UserRegistered"

# AsyncAPI 3.0
channels:
  userRegistered:
    address: user.registered
    messages:
      userRegisteredMessage:
        $ref: "#/components/messages/UserRegisteredMessage"
operations:
  publishUserRegistered:
    action: send
    channel:
      $ref: "#/channels/userRegistered"
    messages:
      - $ref: "#/channels/userRegistered/messages/userRegisteredMessage"
  onUserRegistered:
    action: receive
    channel:
      $ref: "#/channels/userRegistered"
    messages:
      - $ref: "#/channels/userRegistered/messages/userRegisteredMessage"
components:
  messages:
    UserRegisteredMessage:
      name: UserRegisteredMessage
      contentType: application/json
      payload:
        $ref: "#/components/schemas/UserRegistered"
```

Step-by-step procedure:

1. **Create channel entries.** For each 2.x channel key:
   - Convert the channel key to a camelCase YAML key (e.g. `user/registered` → `userRegistered`, `order.placed` → `orderPlaced`).
   - Set `address` to the dot-notation equivalent of the channel key (e.g. `user/registered` → `user.registered`).
   - Create a `messages` map under the channel with `$ref` to the message definition.
2. **Create message definitions.** For each message found in 2.x channel operations:
   - Move the message definition to `components/messages`.
   - Name it `<PascalCaseEvent>Message` (e.g. `UserRegisteredMessage`).
   - Set `contentType: application/json` (or the original content type if specified).
   - Move the `payload` reference onto the message definition.
3. **Create operations.** For each 2.x `publish` / `subscribe`:

   | AsyncAPI 2.x | AsyncAPI 3.0 |
   |---|---|
   | `publish` | `action: send` |
   | `subscribe` | `action: receive` |

   - Use the original `operationId` as the operation key.
   - Set `channel` to `$ref: "#/channels/<channelKey>"`.
   - Set `messages` to reference the channel's message(s).
   - Carry over `summary`, `description`, `tags`, `bindings`, and `traits`.

#### Channel key conversion

| 2.x channel key | 3.0 YAML key | 3.0 address |
|---|---|---|
| `user/registered` | `userRegistered` | `user.registered` |
| `order.placed` | `orderPlaced` | `order.placed` |
| `payment/received` | `paymentReceived` | `payment.received` |
| `notification/email/sent` | `notificationEmailSent` | `notification.email.sent` |

Conversion rules:

- For the camelCase YAML key, drop `/` and `.` separators and capitalise each segment after the first.
- For the `address`, replace `/` with `.` and keep all segments lowercase.

#### Server references

AsyncAPI 2.x channels can reference specific servers. In 3.0, channel server references use `$ref` syntax and the server `url` field renames to `host`.

```yaml
# AsyncAPI 2.x
servers:
  production:
    url: broker.example.com
    protocol: kafka
channels:
  user/registered:
    servers:
      - production

# AsyncAPI 3.0
servers:
  production:
    host: broker.example.com
    protocol: kafka
channels:
  userRegistered:
    address: user.registered
    servers:
      - $ref: "#/servers/production"
```

Note: `url` becomes `host` (host only, no protocol prefix); `protocol` remains; channel server references switch to `$ref` form.

#### Traits

AsyncAPI 2.x `messageTraits` and `operationTraits` carry over to 3.0's `components/messageTraits` and `components/operationTraits`. The `$ref` paths change to reflect the new structure — message traits attach to the message definition, operation traits attach to the operation:

```yaml
# AsyncAPI 2.x
channels:
  user/registered:
    subscribe:
      message:
        traits:
          - $ref: "#/components/messageTraits/commonHeaders"
      traits:
        - $ref: "#/components/operationTraits/commonBinding"

# AsyncAPI 3.0
operations:
  onUserRegistered:
    traits:
      - $ref: "#/components/operationTraits/commonBinding"
components:
  messages:
    UserRegisteredMessage:
      traits:
        - $ref: "#/components/messageTraits/commonHeaders"
```

#### `$ref` path updates

| AsyncAPI 2.x | AsyncAPI 3.0 (after upgrade) | After Step 3 (decomposition) |
|---|---|---|
| `$ref: "#/components/schemas/Foo"` | `$ref: "#/components/schemas/Foo"` (temporary) | `$ref: "../schemas/foo.yaml"` |
| `$ref: "#/components/messages/Foo"` | `$ref: "#/components/messages/Foo"` (unchanged) | unchanged |
| `$ref: "#/components/messageTraits/Foo"` | `$ref: "#/components/messageTraits/Foo"` (unchanged) | unchanged |

Write the upgraded content back to the file. The file is now in AsyncAPI 3.0 form but may still contain inline payload schemas under `components/schemas`.

Files already at AsyncAPI 3.0 skip Step 2 entirely and proceed directly to Step 3.

### Step 3 — Decompose inline payloads

For every AsyncAPI file (whether upgraded or already at 3.0), scan for inline payload schemas and extract them to standalone files in `$CONTRACTS_DIR/schemas/`. The schemas are owned by the json-schema format skill once they land — the importer just creates them.

#### What counts as inline

- **`components/schemas/<Name>`** — definitions inherited from a 2.x `components/schemas` block or already in `components/schemas` on a 3.0 file.
- **Inline message payloads** under `components.messages.<Name>.payload` (no `$ref`).
- **Inline channel-level message payloads** under `channels.<key>.messages.<name>.payload` when the file declares messages directly on the channel rather than in `components/messages`.

Schemas that are already `$ref` pointers to `../schemas/` are left untouched.

#### Filename derivation

| Context | Naming rule | Example |
|---|---|---|
| `components/schemas/<Name>` | Kebab-case the key | `OrderPlaced` → `order-placed.yaml` |
| Inline payload with `title:` | Kebab-case the title | `title: "Order Placed"` → `order-placed.yaml` |
| Inline payload, no `title`, on `components/messages/<MessageName>` | Strip the `Message` suffix from the message name and kebab-case the result | `OrderPlacedMessage` → `order-placed.yaml` |
| Inline payload, no `title`, on a channel | Use the channel concept (camelCase key → kebab-case) | `orderPlaced` → `order-placed.yaml` |

Disambiguation: when two extracted schemas would produce the same filename, append the event domain (`order-placed-billing.yaml` vs `order-placed.yaml`). Filenames are kebab-case with a `.yaml` extension; one type per file.

#### Baseline conflict check

Before writing each extracted schema, compare it to any existing file with the same name in `$BASELINE_DIR/schemas/`:

- **Structurally equivalent** (same `properties`, `required`, types) — drop the extracted file and replace inline references with `$ref` to the baseline file. No new schema file in the slice.
- **Differs structurally** — disambiguate by prefixing with the event domain (`order-placed-billing.yaml`) and write the new file.

#### Replacement

Write each extracted schema to `$CONTRACTS_DIR/schemas/<name>.yaml`. Replace the inline payload definition with:

```yaml
payload:
  $ref: "../schemas/<name>.yaml"
```

After decomposition, walk every `$ref` in the AsyncAPI document and rewrite `#/components/schemas/<Name>` to `../schemas/<name>.yaml`. When `components/schemas` is empty, remove the block. When `components` itself is empty, remove the block too — but preserve `components/messages`, `components/messageTraits`, and `components/operationTraits` if they hold definitions.

#### Headers stay inline

Message headers (`headers` on a message definition) are envelope metadata, not payload. They stay inline as a small object schema on the message — do not extract them to `../schemas/`.

#### Nested inline sub-payloads

When an extracted payload itself contains inline sub-schemas (nested objects):

- **Used only inside this parent** — keep it inline, optionally inside `$defs`.
- **Used elsewhere too** — extract to its own file and `$ref` from both locations.

### Step 4 — Inject Specify metadata

For every schema file in `$CONTRACTS_DIR/schemas/` (newly decomposed and pre-existing), inject Specify-required metadata where missing. Never overwrite values that the source already provided.

| Field | Rule | Generation |
|---|---|---|
| `$schema` | `"https://json-schema.org/draft/2020-12/schema"` | Add if absent. Update older draft URIs to 2020-12 (see [`../../references/json-schema-conventions.md`](../../references/json-schema-conventions.md)). |
| `$id` | `"urn:specify:schemas/<filename-without-extension>"` | Generate from the file path. Never reassign an existing `$id` that matches a baseline schema. |
| `title` | PascalCase type name | Derive from filename: `order-placed.yaml` → `OrderPlaced`. Do not overwrite existing `title`. |
| `description` | Non-empty string | If absent, set to `"[imported — description pending review]"` and surface in the import report. |

For the AsyncAPI document itself, verify that `info.title`, `info.version`, and `info.description` are present. Inject `info.description: "[imported — description pending review]"` if missing.

**Contract normalisation rules for top-level AsyncAPI documents:**

- **`info.version` MUST be SemVer.** When the imported value does not parse as SemVer (e.g. `2024-01-15`, `v2`, `"1"`), do **not** auto-rewrite. Surface a `[manual review required]` entry in the import report naming the file and the offending value, and let the operator decide on the canonical SemVer string. The single-mode verifier (Check 4) and the merge-time `specify tool run contract` gate (the contracts adapter merge contract) will block on the unaltered value until the operator resolves it.
- **Preserve `info.x-specify-id` verbatim.** When the source carries `info.x-specify-id`, copy it through unchanged — even when the value violates the kebab-case format (the verifier flags the format issue with the file path, which is enough for the operator to fix). Never invent or auto-derive an id during import; new ids are an authoring decision.

For each message in `components/messages`, verify `name` and `contentType`. If `contentType` is absent, default to `application/json` and surface in the import report.

### Step 5 — Place files, validate, report

#### Place files

Move each file to its canonical subdirectory under `$CONTRACTS_DIR/`:

| File type | Target | Trigger |
|---|---|---|
| AsyncAPI files | `$CONTRACTS_DIR/messages/` | Top-level `asyncapi:` key |
| JSON Schema files (decomposed) | `$CONTRACTS_DIR/schemas/` | Step 3 output |

Remove the original file when the canonical location differs from where the operator placed it. Create subdirectories only when they will contain at least one file.

#### Validate

Run [`verifier.md`](./verifier.md) in `single` mode against `$SLICE_DIR` to confirm `$ref` resolution, message and schema metadata completeness, and binding coverage. If the verifier reports issues, re-enter Steps 3–4 for targeted repair before producing the report.

#### Report

Produce a markdown import report:

```markdown
## Import Report (Messaging)

### Files Processed
- **Total input files:** N
- **AsyncAPI 2.x → 3.0:** N
- **Already at AsyncAPI 3.0:** N
- **JSON-only inputs converted to YAML:** N
- **Unrecognised (skipped):** N

### Inline Payload Decomposition
- **Schemas extracted:** N
- **Baseline duplicates avoided:** N (matched existing baseline schemas)
- `components/schemas/OrderPlaced` → `contracts/schemas/order-placed.yaml`
- `channels/orderCancelled/messages/orderCancelled/payload` → `contracts/schemas/order-cancelled.yaml`

### Metadata Injected
- `contracts/schemas/order-placed.yaml` — added `$id`, `$schema`
- `contracts/messages/order-events.yaml` — added `info.description`, defaulted `contentType` on `OrderCancelledMessage`

### Validation Result
All checks passed (N $ref pointers, N schemas, N message bindings verified).

### Manual Review Required
- `unknown-format.yaml` — missing `asyncapi` key, no JSON Schema signature.
- `legacy-events.yaml` — `x-internal-routing-tier` extension preserved but not validated.
```

Report semantics:

- **Zero manual review items** is the ideal outcome — every file detected, upgraded, decomposed, and metadata-injected automatically.
- **Manual review items are expected for complex imports.** Vendor-specific constructs (broker bindings, custom traits) and unclassifiable files surface here rather than being silently dropped.
- **The validation result confirms internal consistency.** If the verifier reports issues, fix and re-run — do not finalise the report until the verifier passes.

## Edge cases

| Scenario | Handling |
|---|---|
| Mixed input formats (AsyncAPI 2.x + AsyncAPI 3.0 + JSON Schema) in one directory | Process each file independently. JSON Schema files are out of scope — route them to the `json-schema` importer sub-flow. |
| AsyncAPI file `$ref`s a sibling file in `$CONTRACTS_DIR/` | Process the referenced file first; rewrite the `$ref` to the post-decomposition path. |
| AsyncAPI 2.x file with both `publish` and `subscribe` on the same channel | Generate two separate operations in 3.0 — one with `action: send`, one with `action: receive`. Carry both `operationId` values forward. |
| AsyncAPI 2.x channel key uses unusual separators (e.g. `user.registered.v2`) | Convert to camelCase key (`userRegisteredV2`); dot-notation address (`user.registered.v2`); flag the version segment in the report — channel addresses should not encode versions per the conventions reference. |
| Name collision during decomposition (two distinct payloads, same derived filename) | Disambiguate by prefixing with the source event domain (`order-events-error.yaml` vs `notification-events-error.yaml`). |
| Empty `components/schemas` after decomposition | Remove the block. Preserve `components/messages`, `components/messageTraits`, `components/operationTraits` if non-empty. |
| Vendor extensions (`x-*` keys) | Preserve verbatim during upgrade and decomposition. Note their presence in the report; never validate or transform them. |
| File contains multiple YAML documents (`---` separators) | Rare. Process the first document; flag the rest in the report for manual review. |
| Broker-specific bindings (Kafka, AMQP, MQTT) attached to channels or operations | Carry over verbatim under the existing `bindings` key. Note their presence in the report — broker semantics are out of contract scope but the structural shape is preserved. |
| Message uses `correlationId` field (2.x had a top-level field on the message) | Carry over to 3.0 as a `correlationId` on the message definition; the field shape is unchanged between versions. |

## Hard rules

1. **No data loss.** Every channel, operation, message, payload, header, trait, and binding in the source must be present in the output. Information may be restructured but not silently dropped.
2. **Valid AsyncAPI 3.0.** Every output file must parse as AsyncAPI 3.0.0.
3. **One type per schema file** after decomposition.
4. **Kebab-case `.yaml` filenames** for both AsyncAPI and decomposed schema files.
5. **`$ref` resolution.** Every `$ref` in the output must resolve to a file in `$CONTRACTS_DIR/schemas/`, `$BASELINE_DIR/schemas/`, or (for `components/messages`, `components/messageTraits`, `components/operationTraits`, `channels`) within the same AsyncAPI document.
6. **`$id` stability.** Never reassign a baseline `$id` value. New schemas get fresh `$id` values from the file path.
7. **Baseline preservation.** Never modify any file in root `contracts/`.

## Verification checklist

Before completing the import:

- [ ] Every input file classified — format detected or flagged for review.
- [ ] All AsyncAPI 2.x files upgraded to AsyncAPI 3.0.
- [ ] All inline payloads decomposed to `$CONTRACTS_DIR/schemas/`.
- [ ] All payload `$ref` pointers updated to use `../schemas/` convention.
- [ ] All schema files have `$id`, `$schema`, `title`, `description`.
- [ ] All AsyncAPI files have `info.title`, `info.version`, `info.description`.
- [ ] All messages have `name` and `contentType`.
- [ ] Files placed in correct subdirectories (`messages/`, `schemas/`).
- [ ] [`verifier.md`](./verifier.md) (single mode) ran clean.
- [ ] Import report produced with per-file results and manual-review items.
- [ ] No baseline files modified.

## See also

- [`../../references/asyncapi-conventions.md`](../../references/asyncapi-conventions.md) — target AsyncAPI 3.0 conventions.
- [`../../references/json-schema-conventions.md`](../../references/json-schema-conventions.md) — target JSON Schema conventions for decomposed payloads.
- [`../../references/artifact-structure.md`](../../references/artifact-structure.md) — directory layout for the post-import baseline shape.
- [`../../references/import-upgrade-policy.md`](../../references/import-upgrade-policy.md) — cross-format framework for format detection, upgrade targets, lossless-vs-lossy decisions, and "when to refuse and ask the operator" cases.
- [`../../references/baseline-vs-delta.md`](../../references/baseline-vs-delta.md) — `$id` stability, baseline immutability, and the contract-given authorship pattern this importer realises.
- [`../../references/report-shape.md`](../../references/report-shape.md) — markdown shape for the import report produced at the end of Step 5.
- [`author.md`](./author.md) — sibling for spec-driven authoring.
- [`verifier.md`](./verifier.md) — sibling for validating imported output.
