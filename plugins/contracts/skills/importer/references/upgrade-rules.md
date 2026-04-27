# Upgrade Rules

Detailed version conversion rules for each upgrade path the importer supports. These rules are high-level guidance — the agent should use them alongside its knowledge of the OpenAPI, AsyncAPI, and JSON Schema specifications to handle edge cases.

## Swagger 2.0 → OpenAPI 3.1

This is the most complex upgrade path. Swagger 2.0 and OpenAPI 3.1 have significant structural differences.

### Top-Level Fields

| Swagger 2.0 | OpenAPI 3.1 | Notes |
|---|---|---|
| `swagger: "2.0"` | `openapi: "3.1.0"` | Direct replacement |
| `host` + `basePath` + `schemes` | `servers` | See §*Server Configuration* |
| `consumes` | Removed (per-operation `content`) | See §*Content Types* |
| `produces` | Removed (per-operation `content`) | See §*Content Types* |
| `definitions` | `components/schemas` (temporary) | Decomposed to external files in Step 4 |
| `parameters` (top-level) | `components/parameters` | Structural move |
| `responses` (top-level) | `components/responses` | Structural move |
| `securityDefinitions` | `components/securitySchemes` | Key rename + structural changes |
| `security` | `security` | No change |
| `tags` | `tags` | No change |
| `externalDocs` | `externalDocs` | No change |

### Server Configuration

Swagger 2.0 uses `host`, `basePath`, and `schemes` separately. Combine them into an OpenAPI 3.1 `servers` array:

```yaml
# Swagger 2.0
host: api.example.com
basePath: /v1
schemes:
  - https
  - http

# OpenAPI 3.1
servers:
  - url: https://api.example.com/v1
  - url: http://api.example.com/v1
```

Rules:
- Generate one `servers` entry per scheme.
- If `schemes` is absent, default to `https`.
- If `host` is absent, omit the `servers` block entirely (the contract defines shape, not deployment).
- If `basePath` is absent, use `/`.

### Content Types

Swagger 2.0 uses root-level `consumes` and `produces` arrays. In OpenAPI 3.1, content types are per-operation under the `content` wrapper.

```yaml
# Swagger 2.0 (root-level)
consumes:
  - application/json
produces:
  - application/json

# Swagger 2.0 (operation-level override)
paths:
  /upload:
    post:
      consumes:
        - multipart/form-data

# OpenAPI 3.1 (per-operation)
paths:
  /users:
    post:
      requestBody:
        content:
          application/json:
            schema:
              # ...
      responses:
        "200":
          content:
            application/json:
              schema:
                # ...
```

Resolution order for content types:
1. Operation-level `consumes`/`produces` (if present)
2. Root-level `consumes`/`produces`
3. Default to `application/json`

### Parameters

Swagger 2.0 uses a unified `parameters` array for path, query, header, form, and body parameters. OpenAPI 3.1 separates body parameters into `requestBody`.

**Path, query, and header parameters** — direct structural carry-over:

```yaml
# Swagger 2.0
parameters:
  - name: user_id
    in: path
    required: true
    type: string

# OpenAPI 3.1
parameters:
  - name: user_id
    in: path
    required: true
    schema:
      type: string
```

Note: Swagger 2.0 puts `type` directly on the parameter; OpenAPI 3.1 wraps it in a `schema` object.

**Body parameters** — converted to `requestBody`:

```yaml
# Swagger 2.0
parameters:
  - in: body
    name: body
    required: true
    schema:
      $ref: "#/definitions/UserRegistration"

# OpenAPI 3.1
requestBody:
  required: true
  content:
    application/json:
      schema:
        $ref: "#/components/schemas/UserRegistration"
```

**Form parameters** (`in: formData`) — converted to `requestBody` with `multipart/form-data` or `application/x-www-form-urlencoded`:

```yaml
# Swagger 2.0
parameters:
  - name: file
    in: formData
    type: file
  - name: description
    in: formData
    type: string

# OpenAPI 3.1
requestBody:
  content:
    multipart/form-data:
      schema:
        type: object
        properties:
          file:
            type: string
            format: binary
          description:
            type: string
```

### Response Schemas

```yaml
# Swagger 2.0
responses:
  200:
    description: Success
    schema:
      $ref: "#/definitions/User"

# OpenAPI 3.1
responses:
  "200":
    description: Success
    content:
      application/json:
        schema:
          $ref: "#/components/schemas/User"
```

Note: Swagger 2.0 uses integer response codes; OpenAPI 3.1 uses string response codes.

### `$ref` Path Updates

All `$ref` pointers that reference `#/definitions/` must be updated:

| Swagger 2.0 | Temporary (after upgrade) | After decomposition (Step 4) |
|---|---|---|
| `$ref: "#/definitions/User"` | `$ref: "#/components/schemas/User"` | `$ref: "../schemas/user.yaml"` |

### Type-Specific Mappings

| Swagger 2.0 | OpenAPI 3.1 / JSON Schema |
|---|---|
| `type: file` | `type: string`, `format: binary` |
| `type: integer`, `format: int32` | `type: integer`, `format: int32` (unchanged) |
| `type: integer`, `format: int64` | `type: integer`, `format: int64` (unchanged) |
| `type: number`, `format: float` | `type: number`, `format: float` (unchanged) |
| `type: number`, `format: double` | `type: number`, `format: double` (unchanged) |

### Security Definitions

```yaml
# Swagger 2.0
securityDefinitions:
  api_key:
    type: apiKey
    name: X-API-Key
    in: header
  oauth2:
    type: oauth2
    flow: accessCode
    authorizationUrl: https://auth.example.com/authorize
    tokenUrl: https://auth.example.com/token
    scopes:
      read: Read access
      write: Write access

# OpenAPI 3.1
components:
  securitySchemes:
    api_key:
      type: apiKey
      name: X-API-Key
      in: header
    oauth2:
      type: oauth2
      flows:
        authorizationCode:
          authorizationUrl: https://auth.example.com/authorize
          tokenUrl: https://auth.example.com/token
          scopes:
            read: Read access
            write: Write access
```

OAuth2 flow name mappings:

| Swagger 2.0 | OpenAPI 3.1 |
|---|---|
| `implicit` | `implicit` |
| `password` | `password` |
| `application` | `clientCredentials` |
| `accessCode` | `authorizationCode` |

---

## OpenAPI 3.0.x → OpenAPI 3.1

A less invasive upgrade. The primary changes are JSON Schema alignment and nullable handling.

### Version Field

```yaml
# OpenAPI 3.0
openapi: "3.0.3"

# OpenAPI 3.1
openapi: "3.1.0"
```

### Nullable Handling

OpenAPI 3.0 uses `nullable: true` as a schema keyword. OpenAPI 3.1 uses JSON Schema's type union:

```yaml
# OpenAPI 3.0
properties:
  nickname:
    type: string
    nullable: true
  age:
    type: integer
    nullable: true

# OpenAPI 3.1
properties:
  nickname:
    type:
      - string
      - "null"
  age:
    type:
      - integer
      - "null"
```

Remove the `nullable` keyword after conversion.

### Exclusive Min/Max

OpenAPI 3.0 (JSON Schema Draft 4 style):
```yaml
properties:
  age:
    type: integer
    minimum: 0
    exclusiveMinimum: true
    maximum: 150
    exclusiveMaximum: true
```

OpenAPI 3.1 (JSON Schema Draft 2020-12 style):
```yaml
properties:
  age:
    type: integer
    exclusiveMinimum: 0
    exclusiveMaximum: 150
```

In 3.0, `exclusiveMinimum`/`exclusiveMaximum` are booleans that modify `minimum`/`maximum`. In 3.1, they are standalone numeric values.

### Example → Examples

```yaml
# OpenAPI 3.0
properties:
  email:
    type: string
    example: "user@example.com"

# OpenAPI 3.1
properties:
  email:
    type: string
    examples:
      - "user@example.com"
```

Note: `example` (singular) is deprecated in 3.1 but still valid. The importer converts to `examples` (plural) for forward-compatibility. The singular `example` keyword on media type objects and parameter objects remains valid in 3.1 — only schema-level `example` is deprecated.

### Schema Object Changes

Additional 3.0 → 3.1 schema changes:

| OpenAPI 3.0 | OpenAPI 3.1 | Notes |
|---|---|---|
| `$ref` cannot have sibling keywords | `$ref` can have sibling keywords | No change needed; 3.1 is a superset |
| JSON Schema Draft 4 subset | JSON Schema Draft 2020-12 | `$schema` may be declared on schemas |
| No `const` keyword | `const` keyword available | No change needed for import |
| No `if`/`then`/`else` | `if`/`then`/`else` available | No change needed for import |
| `discriminator` | `discriminator` | No change needed |

### Webhooks

OpenAPI 3.1 introduces a `webhooks` top-level key. No conversion needed — this is new functionality, not a migration of existing content.

---

## AsyncAPI 2.x → AsyncAPI 3.0

A significant structural rework. The primary change is the separation of channels from operations.

### Version Field

```yaml
# AsyncAPI 2.x
asyncapi: "2.6.0"

# AsyncAPI 3.0
asyncapi: "3.0.0"
```

### Channel Restructuring

AsyncAPI 2.x nests operations (`publish`, `subscribe`) under channel items. AsyncAPI 3.0 separates them into distinct `channels` and `operations` sections.

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

Step-by-step:

1. **Create channel entries.** For each 2.x channel key:
   - Convert the channel key to a camelCase YAML key (e.g. `user/registered` → `userRegistered`).
   - Set `address` to the dot-notation equivalent of the channel key (e.g. `user/registered` → `user.registered`, `order.placed` → `order.placed`).
   - Create a `messages` map under the channel with a message reference.

2. **Create message definitions.** For each message found in 2.x channel operations:
   - Move the message definition to `components/messages`.
   - Name it `<PascalCaseEvent>Message` (e.g. `UserRegisteredMessage`).
   - Set `contentType: application/json` (or the original content type if specified).
   - Move the `payload` reference to the message definition.

3. **Create operations.** For each 2.x `publish`/`subscribe`:

   | AsyncAPI 2.x | AsyncAPI 3.0 |
   |---|---|
   | `publish` | `action: send` |
   | `subscribe` | `action: receive` |

   - Use the original `operationId` as the operation key.
   - Set `channel` to `$ref: "#/channels/<channelKey>"`.
   - Set `messages` to reference the channel's message(s).
   - Carry over `summary`, `description`, `tags`, `bindings`, and `traits`.

### Channel Key Conversion

AsyncAPI 2.x channel keys often use slash-separated or dot-separated paths as-is. The conversion normalizes them:

| 2.x Channel Key | 3.0 YAML Key | 3.0 Address |
|---|---|---|
| `user/registered` | `userRegistered` | `user.registered` |
| `order.placed` | `orderPlaced` | `order.placed` |
| `payment/received` | `paymentReceived` | `payment.received` |
| `notification/email/sent` | `notificationEmailSent` | `notification.email.sent` |

Conversion rules:
- Replace `/` and `.` separators with nothing for the camelCase key.
- Use `.` as the separator in the `address` value.

### Server References

AsyncAPI 2.x channels can reference specific servers. In 3.0, server references move from the channel to operations or remain at the top level.

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

Note the 3.0 server changes:
- `url` becomes `host` (host only, no protocol prefix).
- `protocol` remains.
- Channel server references use `$ref` syntax.

### Message Traits

AsyncAPI 2.x `messageTraits` carry over to 3.0's `components/messageTraits`. The `$ref` paths change to reflect the new structure:

```yaml
# AsyncAPI 2.x
channels:
  user/registered:
    subscribe:
      message:
        traits:
          - $ref: "#/components/messageTraits/commonHeaders"

# AsyncAPI 3.0
components:
  messages:
    UserRegisteredMessage:
      traits:
        - $ref: "#/components/messageTraits/commonHeaders"
```

### Operation Traits

Similarly, `operationTraits` carry over:

```yaml
# AsyncAPI 2.x
channels:
  user/registered:
    subscribe:
      traits:
        - $ref: "#/components/operationTraits/commonBinding"

# AsyncAPI 3.0
operations:
  onUserRegistered:
    traits:
      - $ref: "#/components/operationTraits/commonBinding"
```

### `$ref` Path Updates

| AsyncAPI 2.x | AsyncAPI 3.0 |
|---|---|
| `$ref: "#/components/schemas/Foo"` | `$ref: "#/components/schemas/Foo"` (temporary; decomposed in Step 4) |
| `$ref: "#/components/messages/Foo"` | `$ref: "#/components/messages/Foo"` (unchanged) |
| `$ref: "#/components/messageTraits/Foo"` | `$ref: "#/components/messageTraits/Foo"` (unchanged) |

After Step 4 (decomposition), schema `$ref` pointers become `$ref: "../schemas/foo.yaml"`.

---

## JSON Schema Draft Upgrades

When standalone JSON Schema files use older draft versions, the importer updates the `$schema` field and applies any necessary syntax changes.

### Common Adjustments

| Old Draft | Change | Notes |
|---|---|---|
| Draft 4 (`draft-04`) | `exclusiveMinimum` / `exclusiveMaximum` — boolean to numeric | Same as OpenAPI 3.0 → 3.1 |
| Draft 4 | `id` → `$id` | Keyword renamed |
| Draft 6 | `$id` already correct | No change needed |
| Draft 7 | `$id` already correct | May have `if`/`then`/`else` — no change needed |
| Draft 2019-09 | `$schema` URI update | `$defs` already correct |
| Draft 2020-12 | Already current | No change needed |

### `$schema` Value Mapping

| Old Value | New Value |
|---|---|
| `http://json-schema.org/draft-04/schema#` | `https://json-schema.org/draft/2020-12/schema` |
| `http://json-schema.org/draft-06/schema#` | `https://json-schema.org/draft/2020-12/schema` |
| `http://json-schema.org/draft-07/schema#` | `https://json-schema.org/draft/2020-12/schema` |
| `https://json-schema.org/draft/2019-09/schema` | `https://json-schema.org/draft/2020-12/schema` |
| `https://json-schema.org/draft/2020-12/schema` | No change |

### Draft 4 Specific: `definitions` → `$defs`

```yaml
# Draft 4
definitions:
  Address:
    type: object

# Draft 2020-12
$defs:
  Address:
    type: object
```

Update internal `$ref` pointers accordingly: `$ref: "#/definitions/Address"` → `$ref: "#/$defs/Address"`.

---

## General Upgrade Principles

1. **Preserve semantics.** The upgraded file must describe the same API as the original. Structural changes are allowed; behavioral changes are not.
2. **Preserve vendor extensions.** All `x-*` keys are carried through unchanged.
3. **Preserve comments where possible.** YAML comments may be lost during parsing and re-serialization — this is acceptable but should be noted in the import report if the source had significant comments.
4. **Preserve ordering.** Maintain the original key ordering where possible for readability and diffability.
5. **Handle unknowns conservatively.** When a construct has no clear mapping (e.g. a Swagger 2.0 vendor extension that implies behavior), preserve it as-is and flag it in the import report.

## See Also

- [format-detection.md](format-detection.md) — how to identify the input format
- [json-schema-conventions.md](../../../references/json-schema-conventions.md) — target JSON Schema conventions
- [openapi-conventions.md](../../../references/openapi-conventions.md) — target OpenAPI 3.1 conventions
- [asyncapi-conventions.md](../../../references/asyncapi-conventions.md) — target AsyncAPI 3.0 conventions
