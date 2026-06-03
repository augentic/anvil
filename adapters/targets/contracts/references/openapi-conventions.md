# OpenAPI Conventions

Rules for OpenAPI binding files under `contracts/http/`. These files describe HTTP endpoints and wire request/response schemas to the shared JSON Schema payload definitions in `../schemas/`.

## Version

All OpenAPI files use **OpenAPI 3.1.0**. OpenAPI 3.1 has native JSON Schema support (Draft 2020-12), which means `$ref` pointers to external JSON Schema files work without translation or subsetting.

```yaml
openapi: "3.1.0"
```

Do not use OpenAPI 3.0.x — it uses a JSON Schema *subset* that diverges from the standard. If importing an existing OpenAPI 3.0 document, upgrade it to 3.1 first (see the importer skill).

## File Naming

- **Kebab-case** `.yaml` files named after the API domain: `user-api.yaml`, `billing-api.yaml`, `order-api.yaml`.
- A single file may contain **multiple related endpoints** — all CRUD operations for a resource typically live in one file (e.g. `POST /users`, `GET /users/{id}`, `DELETE /users/{id}` all in `user-api.yaml`).
- Split into separate files when APIs serve distinct business domains. Use judgment: `user-api.yaml` and `billing-api.yaml` are separate; `GET /users` and `POST /users` are not.

## Top-Level Structure

Every OpenAPI file must include these top-level keys:

```yaml
openapi: "3.1.0"

info:
  title: User API
  version: "1.0.0"
  description: User registration, authentication, and adapter management.

paths:
  /users:
    post:
      # ...
    get:
      # ...
  /users/{id}:
    get:
      # ...
    delete:
      # ...
```

### `info` Block

| Field | Rule |
|-------|------|
| `title` | Human-readable API name. Matches the file's domain (e.g. "User API" for `user-api.yaml`). |
| `version` | **MUST parse as SemVer per [semver.org](https://semver.org)** (contract identity/version validation), including optional prerelease labels (`1.0.0-draft.1`). Starts at `"1.0.0"` for new contracts. Bump rules (when to advance major / minor / patch) are skill-side judgement; the validator only checks that the value parses. A non-SemVer value (e.g. a `YYYY-MM-DD` date) is a hard validation failure under both the format verifier (single-mode Check 4) and the merge-time `specify tool run contract` gate. |
| `description` | Brief description of the API's purpose and scope. Derived from the spec's behavioral description of the adapter. |
| `x-specify-id` | **Optional rename-stable identifier** (contract identity/version validation). When present, MUST match `^[a-z][a-z0-9-]*$` and be ≤ 64 characters; MUST be unique across every top-level contract under root `contracts/`. The id survives file moves and `info.version` bumps — once set on a contract, never change it. SHOULD be set on new top-level OpenAPI documents (typically the file stem, e.g. `user-api` for `user-api.yaml`). Path-based references in `registry.yaml` remain canonical; the id is a hint, not a substitute. |

### `servers` (optional)

Omit `servers` unless the spec explicitly requires a base URL. Contract files define interface shapes, not deployment configuration. Runtime base URLs are an implementation concern.

## `$ref` to `../schemas/`

**All request body and response body schemas must use `$ref` pointers to `../schemas/`.** Do not inline schema definitions in the OpenAPI document.

```yaml
paths:
  /users:
    post:
      summary: Register a new user account.
      operationId: createUser
      requestBody:
        required: true
        content:
          application/json:
            schema:
              $ref: "../schemas/user-registration.yaml"
      responses:
        "201":
          description: User created successfully.
          content:
            application/json:
              schema:
                $ref: "../schemas/user.yaml"
        "400":
          description: Invalid input.
          content:
            application/json:
              schema:
                $ref: "../schemas/error-response.yaml"
        "409":
          description: Email already registered.
          content:
            application/json:
              schema:
                $ref: "../schemas/error-response.yaml"
```

### Why No Inline Schemas

- **Single source of truth.** A `UserRegistration` type used in both an HTTP endpoint and a message payload is defined once in `schemas/user-registration.yaml`.
- **Diffability.** Schema changes show up in the schema file diff, not buried inside an OpenAPI document.
- **Code generation.** Tools like `typify` generate Rust types from standalone JSON Schema files. Inline schemas require extraction before code generation.

### `components/schemas` Block

Do not use `components/schemas` in the OpenAPI document for domain types. Domain types live in `../schemas/`. The `components` block may be used for OpenAPI-specific constructs that are not domain types:

- `components/parameters` — shared path/query parameters
- `components/headers` — shared response headers
- `components/securitySchemes` — authentication schemes (when the contract must declare them)

## Path Conventions

- **Kebab-case** path segments: `/user-profiles`, not `/userProfiles` or `/user_profiles`.
- **Plural nouns** for collection resources: `/users`, `/orders`, `/invoices`.
- **Path parameters** use `{param_name}` with snake_case: `/users/{user_id}`, `/orders/{order_id}/items/{item_id}`.
- **No trailing slashes.**
- **No verbs in paths.** Use HTTP methods to express actions: `POST /users` (create), `DELETE /users/{user_id}` (remove). Not `POST /create-user`.

## Method Conventions

HTTP methods are **lowercase** in the YAML:

```yaml
paths:
  /users:
    get:
      # List users
    post:
      # Create user
  /users/{user_id}:
    get:
      # Get user by ID
    put:
      # Replace user
    patch:
      # Partial update
    delete:
      # Delete user
```

Standard method semantics:

| Method | Semantics | Typical Status Codes |
|--------|-----------|---------------------|
| `get` | Read / list | 200, 404 |
| `post` | Create | 201, 400, 409 |
| `put` | Full replace | 200, 400, 404 |
| `patch` | Partial update | 200, 400, 404 |
| `delete` | Remove | 204, 404 |

## Response Conventions

- Response codes are **strings**: `"200"`, `"201"`, `"400"`, `"404"`, `"409"`, `"500"`.
- Every endpoint must have **at least one success response** and **relevant error responses** derived from the spec's error conditions.
- Use `"204"` for successful operations with no response body (e.g. `delete`).
- Common error responses:

| Code | When |
|------|------|
| `"400"` | Invalid input, validation failure |
| `"401"` | Authentication required (when `securitySchemes` is declared) |
| `"403"` | Insufficient permissions |
| `"404"` | Resource not found |
| `"409"` | Conflict (duplicate, state violation) |
| `"500"` | Internal server error |

- Error responses should reference the shared `error-response.yaml` schema via `$ref` unless the spec defines a domain-specific error shape.

## `operationId`

Every operation **must** have an `operationId`. Use camelCase, verb-first:

| Pattern | Example |
|---------|---------|
| Create | `createUser`, `placeOrder` |
| Read one | `getUser`, `getOrder` |
| List | `listUsers`, `listOrders` |
| Update | `updateUser`, `updateOrderStatus` |
| Delete | `deleteUser`, `cancelOrder` |

`operationId` values must be unique across all operations in all OpenAPI files in the contract tree.

## Pagination (when applicable)

For list endpoints, use query parameters:

```yaml
/users:
  get:
    summary: List users.
    operationId: listUsers
    parameters:
      - name: limit
        in: query
        schema:
          type: integer
          minimum: 1
          maximum: 100
          default: 20
      - name: offset
        in: query
        schema:
          type: integer
          minimum: 0
          default: 0
```

Define a pagination wrapper schema in `../schemas/` if the spec uses cursor-based or token-based pagination.

## Content Types

- Default to `application/json` for request and response bodies.
- Use `multipart/form-data` for file uploads when the spec requires it.
- Use `application/octet-stream` for raw binary responses.

## Scope Boundary

Contracts capture the *structural shape* of HTTP interfaces — endpoint paths, methods, payload schemas, error codes. The following concerns stay in `design.md`, not in the contract:

- Authentication schemes and `securitySchemes` (implementation policy)
- Rate limiting
- Caching strategies
- Retry policies
- Versioning approaches (URL-based, header-based)

Include `securitySchemes` in the contract only when the spec explicitly requires it for interface compatibility (e.g. the consumer must send a specific auth header to pass request validation).

## See Also

- [json-schema-conventions.md](json-schema-conventions.md) -- Shared payload schema rules
- [asyncapi-conventions.md](asyncapi-conventions.md) -- AsyncAPI 3.0 binding conventions
- [artifact-structure.md](artifact-structure.md) -- Directory layout and naming rules
