# Language-to-Specify Mapping Guide

This guide explains how to map common programming language constructs to the language-agnostic Specify artifact format. Examples are provided primarily in TypeScript, but the principles apply to **any source language** (Go, Python, Rust, Java, C#, etc.).

> **Scope**: This document focuses on *how to recognize and translate* source-code patterns into artifacts. For guidance on *what commonly gets missed* and why, see the [Context Gaps Reference](context-gaps.md).

## Core Principle

The artifacts describe **what the code does**, not **how it should be implemented** in a target language. Avoid introducing language-specific concepts from either the source or target language.

## Language Detection

Before mapping, identify the source language from:
- File extensions: `.ts`, `.js`, `.go`, `.py`, `.rs`, `.java`, `.cs`, etc.
- Dependency files: `package.json`, `go.mod`, `requirements.txt`, `Cargo.toml`, `pom.xml`
- Syntax patterns: The same concepts exist across languages with different syntax

## Classes

**TypeScript Pattern:**
```typescript
export class EventProcessor {
    private config: Config;
    
    constructor(config: Config) {
        this.config = config;
    }
    
    async process(input: EventInput): Promise<void> {
        // method implementation
    }
}
```

**Specify Mapping:**
- **TS class** → One business logic block per public method
- **Constructor** → Document as preconditions or initialization logic
- **Instance properties** → Document only if relevant to method behavior
- **Private methods** → Document only if called by public methods (as sub-steps in algorithm)

**Example:**
```markdown
#### Symbol: EventProcessor.process

- **Preconditions**:
  - [infrastructure] Config object must be initialized with tenant ID
```

## Interfaces and Types

**TypeScript Pattern:**
```typescript
interface Message {
    id: string;
    content: string;
    metadata: {
        timestamp: number;
        source: string;
    };
}

type Result<T> = { success: true; data: T } | { success: false; error: string };
```

**Specify Mapping:**
- **TS interface** → Full type definition under "Structures" section
- **Preserve field names, types, and nesting exactly as in source**
- **Generic types** → Preserve with `<T>` notation and document constraints
- **Union types** → Document all variants

**Example:**
```markdown
### Structures

- Types:
  - `Message`:
    ```json
    {
      "id": "string",
      "content": "string",
      "metadata": {
        "timestamp": "number",
        "source": "string"
      }
    }
    ```
  - `Result<T>`: Union type
    - Success variant: `{ success: true, data: T }`
    - Failure variant: `{ success: false, error: string }`
```

## Serialization Decorators and Field Name Mappings

When source code deserializes data from a wire format (XML, JSON, CSV, etc.), library-specific decorators, annotations, or struct tags often rename fields between the wire representation and the code-level property. The artifacts must capture **both** names using the `(wire: "...")` annotation so the target implementation can produce correct serialization attributes.

### TypeScript (ta-json)

**Source Pattern:**
```typescript
import { JsonProperty, JsonConverter } from 'ta-json';

class TrainUpdate {
    @JsonProperty("horaEntrada")
    arrivalTime: number;

    @JsonProperty("haEntrado")
    @JsonConverter(BooleanConverter)  // converts string "true"/"false" to boolean
    hasArrived: boolean;

    @JsonProperty("pasoTren")
    changes: Change[];
}
```

**Specify Mapping:**
```markdown
- Types:
  - `TrainUpdate`:
    ```json
    {
      "arrivalTime" (wire: "horaEntrada"): "number",
      "hasArrived" (wire: "haEntrado", converter: string "true"/"false" to boolean): "boolean",
      "changes" (wire: "pasoTren"): "Change[]"
    }
    ```
```

### TypeScript (class-transformer)

**Source Pattern:**
```typescript
import { Expose } from 'class-transformer';

class User {
    @Expose({ name: 'user_name' })
    userName: string;
}
```

**Specify Mapping:**
```markdown
- Types:
  - `User`:
    ```json
    {
      "userName" (wire: "user_name"): "string"
    }
    ```
```

### Java (Jackson)

**Source Pattern:**
```java
public class Event {
    @JsonProperty("event_type")
    private String eventType;

    @JsonDeserialize(using = EpochToInstantDeserializer.class)
    @JsonProperty("created_at")
    private Instant createdAt;
}
```

**Specify Mapping:**
```markdown
- Types:
  - `Event`:
    ```json
    {
      "eventType" (wire: "event_type"): "string",
      "createdAt" (wire: "created_at", converter: epoch seconds to timestamp): "timestamp"
    }
    ```
```

### Go (struct tags)

**Source Pattern:**
```go
type Message struct {
    TrainID   string `json:"train_id" xml:"IdTren"`
    StationID int    `json:"station_id" xml:"estacion"`
}
```

**Specify Mapping:**
```markdown
- Types:
  - `Message` (wire formats: JSON and XML):
    ```json
    {
      "TrainID" (wire-json: "train_id", wire-xml: "IdTren"): "string",
      "StationID" (wire-json: "station_id", wire-xml: "estacion"): "number"
    }
    ```
```

### Python (pydantic)

**Source Pattern:**
```python
from pydantic import BaseModel, Field

class TrainUpdate(BaseModel):
    arrival_time: int = Field(alias="horaEntrada")
    has_arrived: bool = Field(alias="haEntrado")
```

**Specify Mapping:**
```markdown
- Types:
  - `TrainUpdate`:
    ```json
    {
      "arrival_time" (wire: "horaEntrada"): "number",
      "has_arrived" (wire: "haEntrado"): "boolean"
    }
    ```
```

### C# (System.Text.Json / Newtonsoft)

**Source Pattern:**
```csharp
public class Event
{
    [JsonPropertyName("event_type")]
    public string EventType { get; set; }
}
```

**Specify Mapping:**
```markdown
- Types:
  - `Event`:
    ```json
    {
      "EventType" (wire: "event_type"): "string"
    }
    ```
```

### XML-Specific Patterns

When the wire format is XML, additional metadata beyond field names must be captured:

**Root element names:**
```typescript
// x2js configuration
const x2js = new X2JS({ rootElement: "CCO" });
// or ta-json root decorator
@JsonObject("ActualizarDatosTren")
class TrainUpdate { ... }
```

**Artifacts:** Document as `(wire format: XML, root element: "ActualizarDatosTren")` on the type definition.

**Array-wrapping configuration:**
```typescript
const xmlOptions = { arrayAccessFormPaths: ["CCO.ActualizarDatosTren.pasoTren"] };
```

**Artifacts:** Document as `(wire: "pasoTren", array-wrapping: always array)` on the field to indicate the XML parser is configured to always treat this element as an array, even when only one child element is present.

**Namespace prefixes:**
```xml
<soap:Envelope xmlns:soap="http://schemas.xmlsoap.org/soap/envelope/">
```

**Artifacts:** Document as `(wire: "soap:Envelope", namespace: "http://schemas.xmlsoap.org/soap/envelope/")`.

### Custom Converters

When a decorator or annotation references a custom converter class, trace into the converter to document what it does:

| Converter | Behavior | Specify annotation |
|-----------|----------|---------------|
| `BooleanConverter` | Converts string `"true"`/`"false"` to boolean | `converter: string "true"/"false" to boolean` |
| `NumberConverter` | Converts string to number via `parseFloat` | `converter: string to number` |
| `ChangeTypeConverter` | Converts number to enum variant | `converter: number to ChangeType enum` |
| `EpochDeserializer` | Converts epoch seconds to timestamp | `converter: epoch seconds to timestamp` |

If the converter logic is not visible in the source, use: `converter: unknown — converter defined outside repo`.

## Async and Promises

### Sequential Operations

**TypeScript Pattern:**
```typescript
async function sequential() {
    const result1 = await fetchData();
    const result2 = await processData(result1);
    return result2;
}
```

**Specify Mapping:**
- **Execution mode**: asynchronous (sequential operations)
- **Algorithm**: Step 1, then step 2, then step 3

**Example:**
```markdown
- **Execution mode**: asynchronous (sequential operations)

- **Algorithm**:
  ```
  1. [infrastructure] Fetch data from external API
  2. [domain] Process fetched data
  3. [mechanical] Return processed result
  ```
```

### Parallel Operations

**TypeScript Pattern:**
```typescript
async function parallel() {
    const [result1, result2] = await Promise.all([
        fetchUser(),
        fetchOrders()
    ]);
    return merge(result1, result2);
}
```

**Specify Mapping:**
- **Execution mode**: asynchronous-parallel
- **Algorithm**: Document parallel steps explicitly

**Example:**
```markdown
- **Execution mode**: asynchronous-parallel (steps 1-2 run concurrently)

- **Algorithm**:
  ```
  1. [infrastructure] Fetch user data from API (parallel with step 2)
  2. [infrastructure] Fetch order data from API (parallel with step 1)
  3. [mechanical] Wait for steps 1-2 to complete
  4. [domain] Merge user and order data
  5. [mechanical] Return merged result
  ```
```

### Race Conditions

**TypeScript Pattern:**
```typescript
async function raceCondition() {
    const result = await Promise.race([
        fetchFromCache(),
        fetchFromAPI()
    ]);
    return result;
}
```

**Specify Mapping:**
- Document as first-to-complete with explicit race semantics

**Example:**
```markdown
- **Algorithm**:
  ```
  1. [infrastructure] Race: Fetch from cache OR fetch from API (whichever completes first)
     a. Cache fetch completes first: use cached data
     b. API fetch completes first: use API data
  2. [mechanical] Return result from winning race participant
  ```
```

## Control Flow

### If/Else Statements

**TypeScript Pattern:**
```typescript
function validate(data: Data): Result {
    if (!data.id) {
        return { success: false, error: 'Missing ID' };
    } else {
        return { success: true, data };
    }
}
```

**Specify Mapping:**
- Use hierarchical algorithm structure with branches
- Document early returns

**Example:**
```markdown
- **Algorithm**:
  ```
  1. [domain] Validate data ID
     a. If ID is missing:
        i.  [domain] Return failure result with error "Missing ID"
        ii. Halt execution (early return)
     b. Else:
        i. [domain] Return success result with data
  ```
```

### Switch/Case Statements

**TypeScript Pattern:**
```typescript
function handleStatus(status: string): string {
    switch (status) {
        case 'pending':
            return 'Processing';
        case 'completed':
            return 'Done';
        default:
            return 'Unknown';
    }
}
```

**Specify Mapping:**
- Document all cases including default

**Example:**
```markdown
- **Algorithm**:
  ```
  1. [mechanical] Match status value:
     a. Case "pending": Return "Processing"
     b. Case "completed": Return "Done"
     c. Default (all other values): Return "Unknown"
  ```
```

### Ternary Operators

**TypeScript Pattern:**
```typescript
const value = condition ? trueValue : falseValue;
```

**Specify Mapping:**
- Document as inline conditional

**Example:**
```markdown
- **Algorithm**:
  ```
  1. [mechanical] Set value: if condition is true then trueValue, else falseValue
  ```
```

## Loops

### For/ForEach Loops

**TypeScript Pattern:**
```typescript
for (const item of items) {
    processItem(item);
}
```

**Specify Mapping:**
- Document as iteration with runtime-dependent count if not deterministic

**Example:**
```markdown
- **Algorithm**:
  ```
  1. [mechanical] Iterate over items collection (count: runtime-dependent)
     a. For each item:
        i. [domain] Process item
  ```
```

### While Loops

**TypeScript Pattern:**
```typescript
while (hasMore) {
    const batch = fetchBatch();
    process(batch);
}
```

**Specify Mapping:**
- Document termination condition

**Example:**
```markdown
- **Algorithm**:
  ```
  1. [infrastructure] Loop while hasMore flag is true
     a. Fetch batch from source
     b. Process batch
     c. Update hasMore flag based on batch result
  ```
```

## Error Handling

### Try/Catch with Propagation

**TypeScript Pattern:**
```typescript
async function process() {
    try {
        const result = await riskyOperation();
        return result;
    } catch (error) {
        throw new Error('Processing failed');
    }
}
```

**Specify Mapping:**
- Document in Error Handling section

**Example:**
```markdown
- **Error Handling**:
  - Any error from riskyOperation: Catch and re-throw as Error with message "Processing failed"; halt execution
```

### Try/Catch with Recovery

**TypeScript Pattern:**
```typescript
async function processWithFallback() {
    try {
        return await fetchFromAPI();
    } catch (error) {
        console.warn('API failed, using cache');
        return await fetchFromCache();
    }
}
```

**Specify Mapping:**
- Document recovery action

**Example:**
```markdown
- **Error Handling**:
  - API fetch error: Catch, log warning "API failed, using cache", recover by fetching from cache; continue execution

- **Algorithm**:
  ```
  1. [infrastructure] Attempt to fetch data from API
     a. On success: Return API data
     b. On error: Continue to step 2
  2. [infrastructure] Log warning message
  3. [infrastructure] Fetch data from cache as fallback
  4. [mechanical] Return cached data
  ```
```

### Retry Logic

**TypeScript Pattern:**
```typescript
async function fetchWithRetry(url: string, maxRetries = 3) {
    for (let i = 0; i < maxRetries; i++) {
        try {
            return await fetch(url);
        } catch (error) {
            if (i === maxRetries - 1) throw error;
            await delay(100 * Math.pow(2, i)); // Exponential backoff
        }
    }
}
```

**Specify Mapping:**
- Document retry count and backoff strategy

**Example:**
```markdown
- **Error Handling**:
  - HTTP fetch error: Retry up to 3 times with exponential backoff (100ms, 200ms, 400ms); if all attempts fail, propagate error

- **Algorithm**:
  ```
  1. [infrastructure] Attempt HTTP fetch (max 3 attempts)
     a. On success: Return response
     b. On error after all retries: Propagate error; halt execution
     c. On error with retries remaining:
        i.  [mechanical] Delay with exponential backoff (100ms * 2^attempt)
        ii. Retry fetch
  ```
```

## Timing and Delays

### setTimeout

**TypeScript Pattern:**
```typescript
setTimeout(() => {
    publishAudit();
}, 5000);
```

**Specify Mapping:**
- Tag as `[infrastructure]` for timing operations

**Example:**
```markdown
- **Algorithm**:
  ```
  1. [infrastructure] Schedule publishAudit to execute after 5000ms delay
  ```
```

### setInterval

**TypeScript Pattern:**
```typescript
setInterval(() => {
    heartbeat();
}, 30000);
```

**Specify Mapping:**
- Document as recurring operation

**Example:**
```markdown
- **Algorithm**:
  ```
  1. [infrastructure] Schedule heartbeat to execute every 30000ms (repeating)
  ```
```

### Explicit Delays

**TypeScript Pattern:**
```typescript
await delay(1000);
```

**Specify Mapping:**
- Tag as `[mechanical]` for deterministic delays

**Example:**
```markdown
- **Algorithm**:
  ```
  1. [mechanical] Delay execution for 1000ms
  ```
```

## Data Transformations

### Map Operations

**TypeScript Pattern:**
```typescript
const results = items.map(item => transform(item));
```

**Specify Mapping:**
- Document as single transformation step with collection semantics

**Example:**
```markdown
- **Algorithm**:
  ```
  1. [domain] Transform each item in items collection using transform function
  2. [mechanical] Collect transformed results into new array
  ```
```

### Filter Operations

**TypeScript Pattern:**
```typescript
const active = users.filter(u => u.status === 'active');
```

**Specify Mapping:**
- Document filtering condition

**Example:**
```markdown
- **Algorithm**:
  ```
  1. [domain] Filter users collection: keep only items where status equals "active"
  2. [mechanical] Collect filtered results into new array
  ```
```

### Reduce Operations

**TypeScript Pattern:**
```typescript
const total = orders.reduce((sum, order) => sum + order.amount, 0);
```

**Specify Mapping:**
- Document accumulation logic

**Example:**
```markdown
- **Algorithm**:
  ```
  1. [mechanical] Initialize accumulator to 0
  2. [domain] Iterate over orders collection:
     a. For each order: Add order.amount to accumulator
  3. [mechanical] Return final accumulator value
  ```
```

## Environment and Configuration

### Environment Variables

**TypeScript Pattern:**
```typescript
const apiUrl = process.env.API_URL;
const timeout = parseInt(process.env.TIMEOUT_MS || '5000');
```

**Specify Mapping:**
- Document in Constants & Configuration section

**Example:**
```markdown
### Constants & Configuration

- `API_URL` — source: environment variable; value: runtime; semantics: Base URL for external API calls; required: yes
- `TIMEOUT_MS` — source: environment variable; value: runtime; semantics: HTTP request timeout in milliseconds; required: no; default: 5000
```

### Hardcoded Constants

**TypeScript Pattern:**
```typescript
const MAX_RETRIES = 3;
const DEFAULT_PAGE_SIZE = 100;
```

**Specify Mapping:**
- Document with literal values

**Example:**
```markdown
### Constants & Configuration

- `MAX_RETRIES` — source: hardcoded; value: 3; semantics: Maximum retry attempts for HTTP calls
- `DEFAULT_PAGE_SIZE` — source: hardcoded; value: 100; semantics: Number of items per page in paginated responses
```

## Common Patterns

### Middleware/Interceptors

**TypeScript Pattern:**
```typescript
app.use((req, res, next) => {
    req.timestamp = Date.now();
    next();
});
```

**Specify Mapping:**
- Document as preconditions for request handlers

**Example:**
```markdown
- **Preconditions**:
  - [infrastructure] Request object must be enriched with timestamp field (set to current time)
```

### Dependency Injection

**TypeScript Pattern:**
```typescript
class UserService {
    constructor(
        private db: Database,
        private cache: Cache
    ) {}
}
```

**Specify Mapping:**
- Document in External Service Dependencies

**Example:**
```markdown
### External Service Dependencies

- **Database** — type: database
  - Operations: query, insert, update
  - Connection details: Injected via constructor parameter
  - Authentication: unknown — not visible in source

- **Cache** — type: cache
  - Operations: get, set
  - Connection details: Injected via constructor parameter
```

## Edge Cases

### Unknown External Functions

**TypeScript Pattern:**
```typescript
const result = magicFunction(data); // Imported from external package
```

**Specify Mapping:**
- Use unknown token

**Example:**
```markdown
- **Algorithm**:
  ```
  1. [unknown] Call magicFunction(data) — unknown — referenced symbol defined outside repo
  2. [mechanical] Assign result to variable
  ```
```

### Dynamic Property Access

**TypeScript Pattern:**
```typescript
const value = obj[dynamicKey];
```

**Specify Mapping:**
- Document runtime-dependent access

**Example:**
```markdown
- **Algorithm**:
  ```
  1. [mechanical] Access property from obj using dynamicKey (key determined at runtime)
  ```
```

### Type Assertions/Casts

**TypeScript Pattern:**
```typescript
const user = response.data as User;
```

**Specify Mapping:**
- Document expected type, note assertion

**Example:**
```markdown
- **Algorithm**:
  ```
  1. [mechanical] Extract data field from response (expected type: User)
  ```
```
