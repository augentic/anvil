# Format Detection

Rules for identifying the format of an external contract file from its top-level YAML/JSON keys. The importer uses these rules to classify each input file before processing.

## Detection Priority

Read the file's top-level keys and apply the following checks **in order**. The first match wins:

| Priority | Key | Condition | Classification |
|----------|-----|-----------|----------------|
| 1 | `swagger` | Value is `"2.0"` | Swagger 2.0 |
| 2 | `openapi` | Value starts with `"3.0"` | OpenAPI 3.0.x |
| 3 | `openapi` | Value starts with `"3.1"` | OpenAPI 3.1.x |
| 4 | `asyncapi` | Value starts with `"2."` | AsyncAPI 2.x |
| 5 | `asyncapi` | Value starts with `"3.0"` | AsyncAPI 3.0.x |
| 6 | `$schema` | Present and none of the above keys exist | Standalone JSON Schema |
| 7 | — | None of the above | Unrecognized |

The priority order matters: a file with both `openapi` and `$schema` keys is classified as OpenAPI, not standalone JSON Schema.

## Format Signatures

### Swagger 2.0

```yaml
swagger: "2.0"
info:
  title: Example API
  version: "1.0"
host: api.example.com
basePath: /v1
schemes:
  - https
consumes:
  - application/json
produces:
  - application/json
paths:
  /users:
    post:
      summary: Create a user
      parameters:
        - in: body
          name: body
          schema:
            $ref: "#/definitions/UserRegistration"
      responses:
        201:
          description: Created
          schema:
            $ref: "#/definitions/User"
definitions:
  UserRegistration:
    type: object
    properties:
      email:
        type: string
      password:
        type: string
  User:
    type: object
    properties:
      id:
        type: string
      email:
        type: string
```

**Distinguishing features:**
- `swagger: "2.0"` at root
- `host`, `basePath`, `schemes` for server configuration
- `definitions` for schema definitions (not `components/schemas`)
- `consumes` / `produces` at root or operation level
- Body parameters use `in: body` with inline `schema`
- Response schemas are direct `schema` keys (not under `content`)

### OpenAPI 3.0.x

```yaml
openapi: "3.0.3"
info:
  title: Example API
  version: "1.0.0"
servers:
  - url: https://api.example.com/v1
paths:
  /users:
    post:
      summary: Create a user
      operationId: createUser
      requestBody:
        required: true
        content:
          application/json:
            schema:
              $ref: "#/components/schemas/UserRegistration"
      responses:
        "201":
          description: Created
          content:
            application/json:
              schema:
                $ref: "#/components/schemas/User"
components:
  schemas:
    UserRegistration:
      type: object
      properties:
        email:
          type: string
        nickname:
          type: string
          nullable: true
```

**Distinguishing features:**
- `openapi: "3.0.x"` at root (any 3.0 minor version: 3.0.0 through 3.0.4)
- `servers` array instead of `host`/`basePath`/`schemes`
- `requestBody` instead of body `parameters`
- `content` wrapper around response schemas
- `components/schemas` instead of `definitions`
- Uses `nullable: true` for optional types (not JSON Schema union types)
- `example` (singular) on schemas

### OpenAPI 3.1.x

```yaml
openapi: "3.1.0"
info:
  title: Example API
  version: "1.0.0"
  description: User registration and management.
servers:
  - url: https://api.example.com/v1
paths:
  /users:
    post:
      summary: Create a user
      operationId: createUser
      requestBody:
        required: true
        content:
          application/json:
            schema:
              $ref: "../schemas/user-registration.yaml"
      responses:
        "201":
          description: Created
          content:
            application/json:
              schema:
                $ref: "../schemas/user.yaml"
```

**Distinguishing features:**
- `openapi: "3.1.0"` (or any 3.1 minor version) at root
- Full JSON Schema Draft 2020-12 support
- Nullable uses type arrays: `type: ["string", "null"]` (not `nullable: true`)
- `examples` (plural array) on schemas
- `$ref` can appear alongside other keywords (sibling keywords)

**Note:** An OpenAPI 3.1 file is already at the target version and does not need version conversion. It may still require inline schema decomposition and metadata injection.

### AsyncAPI 2.x

```yaml
asyncapi: "2.6.0"
info:
  title: Example Events
  version: "1.0.0"
channels:
  user/registered:
    subscribe:
      operationId: onUserRegistered
      message:
        payload:
          type: object
          properties:
            user_id:
              type: string
            email:
              type: string
            registered_at:
              type: string
              format: date-time
    publish:
      operationId: publishUserRegistered
      message:
        payload:
          $ref: "#/components/schemas/UserRegistered"
components:
  schemas:
    UserRegistered:
      type: object
      properties:
        user_id:
          type: string
        email:
          type: string
```

**Distinguishing features:**
- `asyncapi: "2.x.x"` at root (any 2.x version: 2.0.0 through 2.6.0)
- Operations (`publish`, `subscribe`) nested under channel items
- `message` directly under operation
- Channel addresses use the channel key itself (e.g. `user/registered`)
- `components/schemas` for shared type definitions

### AsyncAPI 3.0.x

```yaml
asyncapi: "3.0.0"
info:
  title: Example Events
  version: "1.0.0"
  description: User lifecycle events.
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
components:
  messages:
    UserRegisteredMessage:
      name: UserRegisteredMessage
      contentType: application/json
      payload:
        $ref: "../schemas/user-registered.yaml"
```

**Distinguishing features:**
- `asyncapi: "3.0.0"` at root
- Channels and operations are separate top-level sections
- `action: send` / `action: receive` instead of `publish` / `subscribe`
- Channel has `address` field for the topic name
- Messages defined in `components/messages`, referenced via `$ref`

**Note:** An AsyncAPI 3.0 file is already at the target version and does not need version conversion. It may still require inline schema decomposition and metadata injection.

### Standalone JSON Schema

```yaml
$schema: "https://json-schema.org/draft/2020-12/schema"
$id: "urn:example:user-registration"
title: UserRegistration
description: Payload for creating a new user account.
type: object
properties:
  email:
    type: string
    format: email
  display_name:
    type: string
  password:
    type: string
    format: password
required:
  - email
  - display_name
  - password
```

**Distinguishing features:**
- `$schema` key present (any JSON Schema draft URI)
- No `openapi`, `asyncapi`, or `swagger` key at root
- May have `$id`, `title`, `type`, `properties`, `required` at root
- May reference other schemas via `$ref`

**Note:** Standalone JSON Schema files may use older draft versions. The importer updates the `$schema` to `"https://json-schema.org/draft/2020-12/schema"` during metadata injection (Step 5).

### Unrecognized Format

A file that has none of the detection keys (`swagger`, `openapi`, `asyncapi`, `$schema`) is classified as unrecognized. Common causes:

- Raw YAML data files that are not API contracts
- Custom IDL formats (Smithy, Protobuf text format, RAML)
- Malformed or truncated files
- Files with a BOM or encoding issues that prevent key detection

**Action:** Skip the file and flag it in the import report for human review. Do not attempt to guess the format.

## JSON vs YAML

Both JSON (`.json`) and YAML (`.yaml`, `.yml`) are valid input formats. The detection rules apply to the parsed content regardless of the file extension. JSON files are converted to YAML during import (Step 3 or Step 4) since Specify conventions require `.yaml` files.

## Multiple Documents in One File

YAML supports multiple documents in a single file separated by `---`. If a file contains multiple YAML documents, each document is classified independently. In practice, multi-document contract files are rare — flag them in the import report and process only the first document.

## See Also

- [upgrade-rules.md](upgrade-rules.md) — version conversion rules for each detected format
- [json-schema-conventions.md](../../../references/json-schema-conventions.md) — target JSON Schema conventions
- [openapi-conventions.md](../../../references/openapi-conventions.md) — target OpenAPI 3.1 conventions
- [asyncapi-conventions.md](../../../references/asyncapi-conventions.md) — target AsyncAPI 3.0 conventions
