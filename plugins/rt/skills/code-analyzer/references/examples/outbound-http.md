# Example: Outbound HTTP Calls

## Scenario

Analyze a simple TypeScript service that receives messages and makes HTTP calls to an external API.

## Input Parameters

```bash
code-analyzer ./src/handler.ts $CHANGE_DIR
```

- **TypeScript Source**: `./src/handler.ts`
- **Specify Output**: `$CHANGE_DIR` (specs/ + design.md)

## TypeScript Source Code

```typescript
// src/handler.ts
interface Message {
    id: string;
    content: string;
    timestamp: number;
}

interface Output {
    status: string;
    data: unknown;
}

export async function processMessage(message: Message): Promise<Output> {
    const apiUrl = process.env.API_URL;
    const response = await fetch(`${apiUrl}/data`, {
        method: 'POST',
        headers: {
            'Content-Type': 'application/json'
        },
        body: JSON.stringify(message)
    });
    const data = await response.json();
    return { status: 'processed', data };
}
```

## Generated Specify Artifacts

```markdown
### Component

- Name: simple-handler
- Source paths: src/handler.ts
- Purpose summary: Processes incoming messages by forwarding them to an external API and returning the processed result.

### Structures

- Imports: (none - uses global fetch)
- Exports: processMessage
- Types:
  - `Message`:
    ```json
    {
      "id": "string",
      "content": "string",
      "timestamp": "number"
    }
    ```
  - `Output`:
    ```json
    {
      "status": "string",
      "data": "unknown"
    }
    ```
- Classes: (none)
- Functions:
  - `processMessage(message: Message): Promise<Output>`
- External dependencies: (none in package.json)

### Constants & Configuration

- `API_URL` — source: environment variable; value: runtime; semantics: Base URL for external API calls; required: yes

### Business Logic Blocks

### Block: processMessage

- **Return type**: Promise<Output>
- **Execution mode**: asynchronous (sequential operations)

- **Input Types**:
  ```
  message: Message
    {
      "id": "string",
      "content": "string",
      "timestamp": "number"
    }
  ```

- **Output Types**:
  ```
  Output
    {
      "status": "string",
      "data": "unknown"
    }
  ```

- **Algorithm**:
  ```
  1. [infrastructure] Retrieve API_URL from environment variable
  2. [mechanical] Construct URL by concatenating API_URL + "/data"
  3. [infrastructure] POST message JSON to constructed URL with Content-Type header
  4. [mechanical] Parse JSON response body
  5. [domain] Construct output object with status "processed" and response data
  6. [mechanical] Return output object
  ```

- **Error Handling**:
  - Environment variable missing (API_URL): Propagate as runtime error; halt execution
  - HTTP fetch error: Propagate; halt execution (no retry)
  - JSON parse error: Propagate; halt execution

- **State Mutations**:
  - (none)

- **Preconditions**:
  - [infrastructure] API_URL environment variable must be set
  - [domain] Message must have valid id, content, and timestamp fields

- **Postconditions**:
  - [domain] Returns Output object with status "processed"
  - [domain] Output.data contains parsed API response

- **Edge cases & failure modes**:
  - [infrastructure] API endpoint unreachable: fetch throws error
  - [infrastructure] API returns non-JSON response: json() throws parse error
  - [unknown] API_URL not set: behavior depends on runtime

- **Complexity / cost notes**:
  - Single HTTP round-trip per invocation
  - Network latency dependent on API_URL endpoint

- **Errors raised**:
  - TypeError: If API_URL is undefined
  - FetchError: If network request fails
  - SyntaxError: If response is not valid JSON

- **Unknowns**:
  - API response shape unknown — not present in source

### API Contracts

#### API Call: Post message to data endpoint

- **Endpoint URL pattern**: `${API_URL}/data`
- **HTTP Method**: POST
- **Request Headers**:
  - `Content-Type`: `application/json`
- **Request Body Shape**:
  ```json
  {
    "id": "string",
    "content": "string",
    "timestamp": "number"
  }
  ```
- **Response Body Shape**:
  ```json
  unknown — not present in source
  ```
- **Authentication Method**: none
- **Error Responses**:
  - unknown — not present in source
- **Retry behavior**: none
- **Timeout**: unknown — not specified

### External Service Dependencies

- **External API** — type: API
  - Operations: POST to `/data` endpoint
  - Connection details: Base URL from `API_URL` env var
  - Data formats: JSON request/response
  - Authentication: none visible in source

### Publication & Timing Patterns

- (none)

### Output Event Structures

- (none - function returns directly, does not publish events)

### Dependencies

- (none - uses built-in fetch)

### Notes

- Uses global fetch API (requires Node 18+ or polyfill)
- No error handling or retry logic
- API response shape not validated
```

## Key Observations

These artifacts demonstrate:

1. **Simple sequential async flow** - Steps execute one after another
2. **Environment configuration** - API_URL documented in Constants section
3. **Full type shapes** - Message and Output types shown with all fields
4. **Unknown handling** - API response shape marked as unknown since not in source
5. **No state mutations** - Pure function with no side effects
6. **Minimal error handling** - Errors propagate, no recovery

## What These Artifacts Enable

A code generator can use these artifacts to produce equivalent code in any language:

- **Rust**: Async function with reqwest HTTP client, env var loading
- **Python**: Async function with aiohttp, os.getenv for config
- **Go**: Function with net/http, os.Getenv for config
- **Java**: Method with HttpClient, System.getenv for config

The artifacts are language-agnostic and focus on **what** the code does, not **how** to implement it.
