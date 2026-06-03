# AsyncAPI Conventions

Rules for AsyncAPI binding files under `contracts/messages/`. These files describe messaging channels and operations, wiring message payload schemas to the shared JSON Schema definitions in `../schemas/`.

## Version

All AsyncAPI files use **AsyncAPI 3.0.0**. AsyncAPI 3.0 separates channels from operations (unlike 2.x where operations were nested under channels) and has native JSON Schema support.

```yaml
asyncapi: "3.0.0"
```

Do not use AsyncAPI 2.x — it conflates channel definition with operation semantics and uses a JSON Schema subset. If importing an existing AsyncAPI 2.x document, upgrade it to 3.0 first (see the importer skill).

## File Naming

- **Kebab-case** `.yaml` files named after the event domain: `order-events.yaml`, `notification-events.yaml`, `user-events.yaml`.
- A single file may contain **multiple related channels** — all events for a domain typically live in one file (e.g. `user.registered`, `user.updated`, `user.deleted` all in `user-events.yaml`).
- Split into separate files when event domains are distinct. Use judgment: user lifecycle events belong together; user events and billing events do not.

## Top-Level Structure

Every AsyncAPI file must include these top-level keys:

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

### `info` Block

| Field | Rule |
|-------|------|
| `title` | Human-readable event domain name. Matches the file's domain (e.g. "Order Events" for `order-events.yaml`). |
| `version` | **MUST parse as SemVer per [semver.org](https://semver.org)** (contract identity/version validation), including optional prerelease labels (`1.0.0-draft.1`). Starts at `"1.0.0"` for new contracts. Bump rules (when to advance major / minor / patch) are skill-side judgement; the validator only checks that the value parses. A non-SemVer value (e.g. a `YYYY-MM-DD` date) is a hard validation failure under both the format verifier (single-mode Check 4) and the merge-time `specify tool run contract` gate. |
| `description` | Brief description of the event domain's purpose and the scenarios it covers. Derived from the spec's behavioral description. |
| `x-specify-id` | **Optional rename-stable identifier** (contract identity/version validation). When present, MUST match `^[a-z][a-z0-9-]*$` and be ≤ 64 characters; MUST be unique across every top-level contract under root `contracts/`. The id survives file moves and `info.version` bumps — once set on a contract, never change it. SHOULD be set on new top-level AsyncAPI documents (typically the file stem, e.g. `order-events` for `order-events.yaml`). Path-based references in `registry.yaml` remain canonical; the id is a hint, not a substitute. |

## Channel Conventions

### Channel Names

Channel `address` values use **dot-notation** reflecting the domain and event type:

```yaml
channels:
  userRegistered:
    address: user.registered
  userUpdated:
    address: user.updated
  orderPlaced:
    address: order.placed
  orderCancelled:
    address: order.cancelled
```

Rules:

- **Dot-separated segments**: `<domain>.<event>`. Lower-case, no hyphens within segments.
- **Past tense for events**: `user.registered`, `order.placed`, `payment.received` — describes what happened.
- **Present tense for commands**: `order.cancel`, `notification.send` — describes what should happen. Use only when the spec describes a command/request pattern rather than an event notification.
- **No versioning in channel names.** Version is in the `info.version` field, not the channel address.

### Channel Keys

The YAML key for each channel entry uses **camelCase** matching the event concept:

```yaml
channels:
  orderPlaced:        # camelCase key
    address: order.placed  # dot-notation address
```

## Operation Conventions

Every channel must have at least one operation. Operations describe who does what with the channel.

```yaml
operations:
  publishOrderPlaced:
    action: send
    channel:
      $ref: "#/channels/orderPlaced"
    summary: Publish an order-placed event when a new order is confirmed.
    messages:
      - $ref: "#/channels/orderPlaced/messages/orderPlacedMessage"

  consumeOrderPlaced:
    action: receive
    channel:
      $ref: "#/channels/orderPlaced"
    summary: Consume order-placed events for fulfillment processing.
    messages:
      - $ref: "#/channels/orderPlaced/messages/orderPlacedMessage"
```

### `action` Values

| Action | Meaning | When |
|--------|---------|------|
| `send` | The application publishes messages to this channel | Producer side |
| `receive` | The application consumes messages from this channel | Consumer side |

A contract may declare both `send` and `receive` operations for the same channel when the producer and consumer roles are both part of the contract's scope.

### Operation Naming

Use camelCase with a verb prefix indicating the action:

| Pattern | Example |
|---------|---------|
| Publish | `publishOrderPlaced`, `sendUserRegistered` |
| Consume | `consumeOrderPlaced`, `receiveUserRegistered` |

Operation names must be unique across all operations in all AsyncAPI files in the contract tree.

## `$ref` to `../schemas/`

**All message payload schemas must use `$ref` pointers to `../schemas/`.** Do not inline schema definitions.

The `$ref` appears on the message's `payload` field within `components/messages`:

```yaml
components:
  messages:
    OrderPlacedMessage:
      name: OrderPlacedMessage
      contentType: application/json
      payload:
        $ref: "../schemas/order-placed.yaml"
```

### Message Definitions

Messages are defined in `components/messages` and referenced from channels and operations via internal `$ref` pointers:

```yaml
channels:
  orderPlaced:
    address: order.placed
    messages:
      orderPlacedMessage:
        $ref: "#/components/messages/OrderPlacedMessage"

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
            description: Correlation ID for distributed tracing.
```

### Why No Inline Schemas

Same rationale as OpenAPI — a domain type used in both an HTTP response and a message payload is defined once in `schemas/`. See [openapi-conventions.md](openapi-conventions.md) for the full reasoning.

## Message Headers

When the spec describes message metadata (correlation IDs, partition keys, trace contexts), declare them in the message's `headers` field:

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
```

Headers describe the message envelope, not the payload. Payload shape belongs in the schema file.

## Content Type

Default to `application/json` for message payloads. Use `application/avro`, `application/protobuf`, or `application/octet-stream` only when the spec explicitly requires a non-JSON wire format.

## Scope Boundary

Contracts capture the *structural shape* of messaging interfaces — channel names, message schemas, operation semantics (send/receive). The following concerns stay in `design.md`, not in the contract:

- Ordering guarantees and partition strategies
- Retry policies and dead-letter queue configuration
- Delivery semantics (at-least-once, exactly-once)
- Consumer group configuration
- Message TTL and retention policies
- Broker-specific bindings (Kafka, RabbitMQ, Azure Service Bus)

Include broker-specific bindings in the contract only when they affect wire compatibility (e.g. a consumer must use a specific protocol binding to receive messages).

## See Also

- [json-schema-conventions.md](json-schema-conventions.md) -- Shared payload schema rules
- [openapi-conventions.md](openapi-conventions.md) -- OpenAPI 3.1 binding conventions
- [artifact-structure.md](artifact-structure.md) -- Directory layout and naming rules
