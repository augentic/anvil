# External API surface documentation (Step 3)

Document every HTTP/API call by tracing the actual deserialization code, not type declarations. The runtime response shape is determined by how the code uses the response, not by interface declarations that may be broader.

## THINK before each call

Before documenting each API call, reason through:

1. What is the complete URL? (Is it hardcoded, from config, or dynamically constructed?)
2. What HTTP method? (GET, POST, PUT, PATCH, DELETE)
3. What headers are sent? (Authorization, Content-Type, custom headers)
4. What is the request body? (Full JSON/XML structure, not just described)
5. What does the response look like? (Trace through actual deserialization code, not type declarations)
6. How is the response parsed? (`response.json()`? XML parser? Text?)
7. What fields are actually accessed from the response? (This reveals the true shape)
8. What happens on errors? (Status codes, error response format, retry behavior)
9. Are there timeouts? (Explicit timeout values)
10. Is authentication required? (API keys, tokens, basic auth)

**Critical**: Trace actual deserialization, not type declarations. If code does `const allocated: string[] = await response.json()`, the response shape is `string[]`, not some broader interface type.

## ANALYZE: per-call documentation

For each external HTTP/API call:

- Endpoint URL pattern (EXACT path and query parameters as constructed in source)
- HTTP method
- Request headers (list each, including how values are obtained -- from config, hardcoded, etc.)
- Request body shape (exact JSON/XML structure)
- Response body shape (CRITICAL: capture full nesting)
- Authentication method (including where the identity/token name comes from -- config variable or hardcoded)
- Error responses (status codes and body shapes)
- **Retry behavior** (if present)
- **Timeout** (if specified)

**Trace actual deserialization, not type declarations.** When the source code parses an API response (e.g., `response.json()`, `JSON.parse()`), trace what the result is assigned to and how its fields are accessed. The runtime response shape is determined by how the code uses the response, not by interface declarations that may be broader. If the code does `const allocated: string[] = await response.json()`, the response shape is `string[]`, not the full interface type. Always follow the data from the HTTP response through parsing to usage to determine the true shape.

**Response shape documentation**: Include a concrete JSON example showing the actual response structure. This prevents downstream code generators from fabricating wrapper types.

```markdown
- **Response shape**: `string[]` (flat JSON array)
- **Example response**: `["NZ 1234", "NZ 5678"]`
- **Usage**: Each string is a vehicle label; spaces are stripped before use as partition key
```

**Authentication source**: When documenting how a token or identity is obtained, capture whether the identity name is hardcoded or comes from configuration:

```markdown
- **Auth**: Bearer token from identity provider
  - Identity name: from config `AZURE_IDENTITY` (NOT hardcoded)
  - Token acquisition: access token requested using identity name
```
