# Context Gaps: Common Missing Details

When analyzing source code for artifact generation, these details are commonly missed or under-documented. Pay special attention to capturing these patterns.

> **Scope**: This document focuses on *what commonly gets missed* and *why it matters*, showing bad vs good artifact comparisons. For guidance on *how to recognize and translate language constructs into artifacts*, see the [Language Mapping Guide](language-mapping.md).
>
> | Topic | This doc (gaps) | Language Mapping |
> | ----- | --------------- | ---------------- |
> | Parallel vs sequential | Gap #1 | Async and Promises |
> | Error handling | Gap #2 | Error Handling |
> | Conditional branching | Gap #4 | Control Flow |
> | Type shapes | Gap #7 | Interfaces and Types |
> | Timing semantics | Gap #5 | Timing and Delays |
> | Configuration | Gap #6 | Environment and Configuration |
> | Data transformations | Gap #10 | Data Transformations |
> | Serialization mappings | Gap #12 | Serialization Decorators |

**Note**: Examples below use TypeScript syntax for consistency. The same patterns apply to Go, Python, Java, Rust, C#, and other languages -- only the syntax differs.

## 1. Parallel vs Sequential Execution

### The Gap

TypeScript's async/await makes it easy to miss whether operations run sequentially or in parallel.

### What to Capture

**Sequential execution:**
```typescript
const user = await fetchUser();     // Step 1
const orders = await fetchOrders(); // Step 2 (waits for step 1)
```

**Artifacts:**
```markdown
- **Execution mode**: asynchronous (sequential)
- **Algorithm**:
  1. [infrastructure] Fetch user data
  2. [infrastructure] Fetch orders data (executes after step 1 completes)
```

**Parallel execution:**
```typescript
const [user, orders] = await Promise.all([
    fetchUser(),
    fetchOrders()
]);
```

**Artifacts:**
```markdown
- **Execution mode**: asynchronous-parallel
- **Algorithm**:
  1. [infrastructure] Fetch user data (parallel with step 2)
  2. [infrastructure] Fetch orders data (parallel with step 1)
  3. [mechanical] Wait for steps 1-2 to complete
```

**Race conditions:**
```typescript
const result = await Promise.race([fetchFromCache(), fetchFromAPI()]);
```

**Artifacts:**
```markdown
- **Algorithm**:
  1. [infrastructure] Race: Fetch from cache OR fetch from API (first to complete wins)
  2. [mechanical] Use result from winning operation
```

### Why It Matters

Parallel operations have different performance characteristics, error handling, and resource usage than sequential operations.

## 2. Error Recovery Patterns

### The Gap

Listing possible errors without documenting what happens when they occur.

### What to Capture

**Don't just list errors:**
```markdown
❌ Errors: HTTP 404, JSON parse error, network timeout
```

**Document the flow:**
```markdown
✓ Error Handling:
  - HTTP 404: Catch and return default empty result; continue execution
  - JSON parse error: Propagate as ParseError; halt execution
  - Network timeout: Retry up to 3 times with exponential backoff (100ms, 200ms, 400ms); then propagate
```

### Retry Logic

**TypeScript:**
```typescript
for (let i = 0; i < 3; i++) {
    try {
        return await fetch(url);
    } catch (error) {
        if (i === 2) throw error;
        await delay(100 * Math.pow(2, i));
    }
}
```

**Artifacts:**
```markdown
- **Error Handling**:
  - Fetch error: Retry up to 3 times with exponential backoff (100ms, 200ms, 400ms); if all attempts fail, propagate error

- **Algorithm**:
  1. [infrastructure] Attempt HTTP fetch (max 3 attempts with backoff)
     a. On success: Return response; halt
     b. On final failure: Propagate error; halt
     c. On intermediate failure: Delay with backoff, retry
```

### Fallback Values

**TypeScript:**
```typescript
try {
    return await fetchFromAPI();
} catch (error) {
    return DEFAULT_VALUE;
}
```

**Artifacts:**
```markdown
- **Error Handling**:
  - API fetch error: Catch and return DEFAULT_VALUE; continue execution (no propagation)
```

### Why It Matters

Error handling determines whether failures are graceful or catastrophic, and whether the system continues or halts.

## 3. State Mutation Scope

### The Gap

Documenting what is mutated without specifying when in the algorithm it occurs.

### What to Capture

**Don't just list mutations:**
```markdown
❌ State Mutations:
  - Write to cache
  - Publish to topic
```

**Show timing and conditions:**
```markdown
✓ State Mutations:
  - [infrastructure] Write to cache with key `user:{id}`, TTL 3600s (step 4, after API fetch)
  - [infrastructure] Publish to topic `user-updated` (step 6, only if cache write succeeds)
```

### Multiple Mutations in Flow

**TypeScript:**
```typescript
async function process(id: string) {
    const data = await fetchData(id);
    await cache.set(`data:${id}`, data, 3600);  // Mutation 1
    
    if (data.needsAudit) {
        await publish('audit', { id, timestamp: Date.now() });  // Mutation 2 (conditional)
    }
    
    return data;
}
```

**Artifacts:**
```markdown
- **State Mutations**:
  - [infrastructure] Write to cache with key `data:{id}`, value: fetched data, TTL: 3600s (step 2, unconditional)
  - [infrastructure] Publish message to topic `audit` (step 3, only if data.needsAudit is true)

- **Algorithm**:
  1. [infrastructure] Fetch data for ID
  2. [infrastructure] Write data to cache
     a. Key: `data:{id}`
     b. TTL: 3600s
  3. [domain] Check if data requires audit
     a. If data.needsAudit is true:
        i. [infrastructure] Publish audit message to topic
     b. Else:
        i. Skip audit
  4. [mechanical] Return data
```

### Why It Matters

Understanding when and under what conditions state changes helps with debugging, testing, and understanding side effects.

## 4. Conditional Branch Completeness

### The Gap

Missing branches or not documenting early returns.

### What to Capture

**Ensure all branches documented:**

**TypeScript:**
```typescript
function validate(data: Data): Result {
    if (!data.id) {
        return { success: false, error: 'Missing ID' };
    }
    
    if (data.amount < 0) {
        return { success: false, error: 'Negative amount' };
    }
    
    return { success: true, data };
}
```

**Artifacts:**
```markdown
- **Algorithm**:
  1. [domain] Validate data.id exists
     a. If ID missing:
        i.  [domain] Return failure with error "Missing ID"
        ii. Halt execution (early return)
     b. Else:
        i. Continue to step 2
  2. [domain] Validate data.amount is non-negative
     a. If amount < 0:
        i.  [domain] Return failure with error "Negative amount"
        ii. Halt execution (early return)
     b. Else:
        i. Continue to step 3
  3. [domain] Return success with data
```

### Ternary and Nested Conditionals

**TypeScript:**
```typescript
const status = isActive ? (isPremium ? 'premium-active' : 'active') : 'inactive';
```

**Artifacts:**
```markdown
- **Algorithm**:
  1. [mechanical] Determine status value:
     a. If isActive is true:
        i.  If isPremium is true: Set status to "premium-active"
        ii. Else: Set status to "active"
     b. Else:
        i. Set status to "inactive"
```

### Why It Matters

Incomplete branch documentation leads to missing logic in generated code.

## 5. Timing Semantics

### The Gap

Documenting that a delay exists without explaining its purpose or relationship to other operations.

### What to Capture

**Don't just list delays:**
```markdown
❌ Timing: 5000ms delay
```

**Explain relationship:**
```markdown
✓ Timing:
  - Delay 5000ms between publishing to `events-topic` and `audit-topic`
  - Purpose: Allow downstream consumers to process event before audit record arrives
```

### Delays Between Operations

**TypeScript:**
```typescript
await publisher.send('events-topic', event);
await delay(5000);
await publisher.send('audit-topic', audit);
```

**Artifacts:**
```markdown
- **Publication & Timing**:
  - Publish to `events-topic`: 1 message, immediate
  - Delay: 5000ms (between event and audit publish)
  - Publish to `audit-topic`: 1 message, after 5s delay
  - Purpose: Ensure event is processed before audit record

- **Algorithm**:
  1. [infrastructure] Publish event to `events-topic`
  2. [mechanical] Delay 5000ms
  3. [infrastructure] Publish audit to `audit-topic`
```

### Recurring Operations

**TypeScript:**
```typescript
setInterval(() => heartbeat(), 30000);
```

**Artifacts:**
```markdown
- **Timing**:
  - Heartbeat operation repeats every 30000ms
  - Purpose: Maintain connection to monitoring service
```

### Why It Matters

Timing affects system behavior, performance, and debugging.

## 6. Configuration vs Constants

### The Gap

Not distinguishing between hardcoded values, environment variables, and runtime configuration.

### What to Capture

**TypeScript:**
```typescript
const MAX_RETRIES = 3;                              // Hardcoded
const API_URL = process.env.API_URL;                // Required env var
const TIMEOUT = parseInt(process.env.TIMEOUT || '5000');  // Optional with default
```

**Artifacts:**
```markdown
### Constants & Configuration

- `MAX_RETRIES` — source: hardcoded; value: 3; semantics: Maximum retry attempts
- `API_URL` — source: environment variable; value: runtime; semantics: Base API URL; required: yes
- `TIMEOUT` — source: environment variable; value: runtime; semantics: Request timeout in ms; required: no; default: 5000
```

### Why It Matters

Generated code needs to know what can be changed at runtime vs what is fixed at compile time.

## 7. Type Shape Completeness

### The Gap

Flattening nested types or not documenting all fields.

### What to Capture

**Don't flatten:**
```markdown
❌ Output: { id, name, profile }
```

**Show full nesting:**
```markdown
✓ Output Types:
  ```
  User
    {
      "id": "string",
      "name": "string",
      "profile": {
        "age": "number",
        "location": "string",
        "preferences": {
          "theme": "string",
          "notifications": "boolean"
        }
      }
    }
  ```
```

### Arrays and Collections

**TypeScript:**
```typescript
interface Response {
    items: Array<{
        id: string;
        tags: string[];
    }>;
}
```

**Artifacts:**
```markdown
- Types:
  - `Response`:
    ```json
    {
      "items": [
        {
          "id": "string",
          "tags": ["string"]
        }
      ]
    }
    ```
```

### Union Types

**TypeScript:**
```typescript
type Result = { success: true; data: Data } | { success: false; error: string };
```

**Artifacts:**
```markdown
- Types:
  - `Result`: Union type with 2 variants
    - Success variant: `{ success: true, data: Data }`
    - Failure variant: `{ success: false, error: string }`
```

### Why It Matters

Incomplete type information prevents accurate code generation.

## 8. External Service Context

### The Gap

Noting that an external service is used without documenting its interface and connection.

### What to Capture

**TypeScript:**
```typescript
const pool = new Pool({
    connectionString: process.env.DATABASE_URL
});
const result = await pool.query('SELECT * FROM users WHERE id = $1', [userId]);
```

**Artifacts:**
```markdown
### External Service Dependencies

- **PostgreSQL** — type: database
  - Operations: SELECT from `users` table
  - Connection details: Connection string from `DATABASE_URL` env var
  - Data formats: SQL rows with columns (id, name, email) mapped to User objects
  - Authentication: Username/password embedded in connection string
  - Query patterns:
    - Parameterized queries using `$1, $2` syntax
    - Returns array of row objects
```

### Message Brokers

**TypeScript:**
```typescript
const kafka = new Kafka({ brokers: [process.env.KAFKA_BROKER] });
const producer = kafka.producer();
await producer.send({
    topic: 'events',
    messages: [{ value: JSON.stringify(event) }]
});
```

**Artifacts:**
```markdown
### External Service Dependencies

- **Kafka** — type: message broker
  - Operations: Publish to topic `events`
  - Connection details: Broker address from `KAFKA_BROKER` env var
  - Data formats: JSON strings serialized from event objects
  - Authentication: unknown — not visible in source
  - Message structure:
    - Topic: `events`
    - Partition key: unknown — not specified
    - Payload: JSON serialized event
```

### Why It Matters

External services define deployment dependencies and integration points.

## 9. Loop Iteration Semantics

### The Gap

Not documenting whether loop count is deterministic or runtime-dependent.

### What to Capture

**Deterministic count:**
```typescript
for (let i = 0; i < 5; i++) {
    process(i);
}
```

**Artifacts:**
```markdown
- **Algorithm**:
  1. [mechanical] Loop 5 times (deterministic)
     a. For each iteration: Process index value
```

**Runtime-dependent count:**
```typescript
for (const item of items) {
    process(item);
}
```

**Artifacts:**
```markdown
- **Algorithm**:
  1. [mechanical] Iterate over items collection (count: runtime-dependent)
     a. For each item: Process item
```

### Why It Matters

Deterministic loops can be optimized differently than runtime-dependent loops.

## 10. Data Transformation Patterns

### The Gap

Not documenting collection operations (map, filter, reduce) with sufficient detail.

### What to Capture

**Map operations:**
```typescript
const transformed = items.map(item => ({ id: item.id, name: item.name.toUpperCase() }));
```

**Artifacts:**
```markdown
- **Algorithm**:
  1. [domain] Transform each item in items collection:
     a. Extract id field (preserve as-is)
     b. Extract name field and convert to uppercase
     c. Create new object with transformed fields
  2. [mechanical] Collect transformed items into new array
```

**Filter operations:**
```typescript
const active = users.filter(u => u.status === 'active' && u.lastLogin > cutoffDate);
```

**Artifacts:**
```markdown
- **Algorithm**:
  1. [domain] Filter users collection:
     a. Keep only items where status equals "active" AND lastLogin > cutoffDate
  2. [mechanical] Collect filtered items into new array
```

**Reduce operations:**
```typescript
const total = orders.reduce((sum, order) => sum + order.amount, 0);
```

**Artifacts:**
```markdown
- **Algorithm**:
  1. [mechanical] Initialize accumulator to 0
  2. [domain] Iterate over orders collection:
     a. For each order: Add order.amount to accumulator
  3. [mechanical] Return final accumulator value (total)
```

### Why It Matters

Collection transformations are common patterns that need accurate representation.

## 11. Active Subset vs Full Dataset

### The Gap

Source code often contains both a complete lookup table (all possible entries) and a runtime filter that limits which entries are actually processed. Capturing the full table without noting the active subset leads to generated code that processes far more data than intended.

### What to Capture

**TypeScript:**
```typescript
// Full mapping table (200 entries)
const CATEGORY_LABEL_MAP: Record<number, string> = {
    1: "Electronics", 2: "Clothing", 3: "Home", /* ... 197 more entries ... */
};

// Runtime filter (only 5 categories are active)
const ACTIVE_CATEGORIES = [1, 3, 7, 12, 45];

function processProduct(categoryId: number) {
    if (!ACTIVE_CATEGORIES.includes(categoryId)) return;
    const label = CATEGORY_LABEL_MAP[categoryId];
    // ...
}
```

**Bad Artifacts:**
```markdown
### Constants
- `CATEGORY_LABEL_MAP` — source: hardcoded; value: 200-entry mapping table; semantics: Maps category IDs to display labels
```

**Good Artifacts:**
```markdown
### Constants
- `ACTIVE_CATEGORIES` — source: hardcoded; value: [1, 3, 7, 12, 45]; semantics: Only these category IDs are processed at runtime
- `CATEGORY_LABEL_MAP` — source: hardcoded; value: { 1: "Electronics", 3: "Home", 7: "Garden", 12: "Sports", 45: "Automotive" }; semantics: Maps active category IDs to labels (full table has 200 entries but only active categories are used)
```

### Why It Matters

Including the full lookup table without the runtime filter creates generated code that processes entries the system was never intended to handle. Document both the full table's existence and the active subset that is actually used.

## 12. Serialization/Wire-Format Field Name Mappings

### The Gap

Source code uses library-specific decorators, annotations, or configuration to map code-level property names to wire-format field names (XML element names, JSON keys, etc.). Capturing only the code-level property names loses the information needed for the target implementation to correctly parse or emit the same wire format.

This also applies to custom converters/deserializers that transform values during parsing (e.g., `BooleanConverter` that converts the string `"true"` to a boolean). Without documenting the converter behavior, the code generator must guess the wire representation.

### What to Capture

**TypeScript:**
```typescript
import { JsonProperty, JsonConverter } from 'ta-json';

class TrainUpdate {
    @JsonProperty("horaEntrada")
    arrivalTime: number;

    @JsonProperty("haEntrado")
    @JsonConverter(BooleanConverter)  // converts string "true"/"false" to boolean
    hasArrived: boolean;
}

// BooleanConverter implementation
class BooleanConverter {
    serialize(value: boolean): string { return value ? "true" : "false"; }
    deserialize(value: string): boolean { return value === "true"; }
}
```

**Bad Artifacts:**
```markdown
- Types:
  - `TrainUpdate`:
    ```json
    {
      "arrivalTime": "number",
      "hasArrived": "boolean"
    }
    ```
```

**Good Artifacts:**
```markdown
- Types:
  - `TrainUpdate` (wire format: XML, root element: "ActualizarDatosTren"):
    ```json
    {
      "arrivalTime" (wire: "horaEntrada"): "number",
      "hasArrived" (wire: "haEntrado", converter: string "true"/"false" to boolean): "boolean"
    }
    ```
```

### Why It Matters

Without wire-format field names, the code generator must guess or hallucinate the serialization mapping. This leads to runtime parsing failures when the generated code encounters the actual wire data with different field names than expected. Similarly, undocumented converters cause the generator to invent conversion logic (e.g., assuming "S"/"N" for Spanish booleans instead of the actual "true"/"false" behavior).

See the [Language Mapping Guide - Serialization Decorators](./language-mapping.md#serialization-decorators-and-field-name-mappings) for per-language examples covering TypeScript, Java, Go, Python, C#, and XML-specific patterns.

## 13. Data Access (SQL / Table Storage / Cache)

### The Gap

The artifacts already have **Source Capabilities Summary** and **External Service Dependencies** (PostgreSQL, Azure Table Storage, Redis). Algorithm steps say things like "Run raw SQL query to compute shape WKT from gtfs.shapes", "Save entity to tripsChangesRepository", "Download fleet data from Azure Table Storage", "Redis cache invalidation", "publish to GTFS invalidate topic". Downstream code generators leave TODOs because the **algorithm does not give actionable cues**: which table/entity, which key pattern, which operation (select/insert/update/delete, get/set/delete).

Adding full SQL or every cache key into the artifacts would bloat them and worsen context limits.

**Relationship to §14:** §13 is about *how* to phrase data access in the artifacts (canonical wording so downstream generators can map to their framework's patterns). §14 is about *ensuring a Business Logic block exists* for every endpoint (including simple list ones). Both apply: blocks need to exist (§14) and their content needs actionable phrasing (§13).

### What to Capture (minimal, no bloat)

**Option A — Canonical phrasing in algorithm steps (preferred)**  
When the source uses ORM, raw SQL, managed table stores (Azure Table Storage / `@azure/data-tables`), or cache (Redis), write the step so a code generator can map it to a data access pattern:

- **Table/database access**: Covers **both** relational SQL databases **and** managed NoSQL table stores (Azure Table Storage, Cosmos DB). Prefer **ORM-style** phrasing (entity + operation + key columns). Use **raw SQL** phrasing only when source does: GeoSearch/spatial (e.g. PostGIS `ST_*`), nested subqueries, or complex transactional multi-statement flows. Phrase so generator can choose:
  - ORM (SQL): `[infrastructure] Table access: SELECT entity TripsChanges WHERE (trip_id, start_date, start_time)`; `INSERT trips_changes (entity TripsChanges)`; `UPDATE trips_changes WHERE (trip_id, start_date, start_time)`.
  - ORM (managed table store): `[infrastructure] Table access: SELECT entity RawVehicle FROM fleetdata (all rows)`; `[infrastructure] Table access: UPDATE entity RawVehicle in fleetdata WHERE (PartitionKey, RowKey)`.
  - Raw SQL (exceptions): `[infrastructure] Table access raw SQL (GeoSearch): shapes → shape_wkt by shape_id (ST_AsText/ST_Simplify)`; `[infrastructure] Table access raw SQL (nested subquery): …`; `[infrastructure] Table access raw SQL (transaction): …`.

  **IMPORTANT — Managed table stores are data access, NOT HTTP**: When the source uses `@azure/data-tables`, `TableClient`, `listEntities`, or constructs REST API calls to `*.table.core.windows.net`, always phrase as `Table access` — never as an outbound HTTP call. The source's SDK/HTTP access pattern is an implementation detail.

- **Cache access**: Name key pattern and operation. e.g.  
  `[infrastructure] Cache: get key ${APP_NAME}:gtfsChanges:${date}; on miss query database`  
  `[infrastructure] Cache: delete keys matching gtfsChanges:* (cache invalidation)`
- **Publish**: Topic and payload shape are already in Publication & Timing or External Services; algorithm step can stay high-level: "Publish trip invalidate event".

One line per logical data access is enough. Do **not** embed full SQL or full key lists in the artifacts.

**Option B — Optional one-line "Data access" per block**  
If algorithm steps stay narrative, add an optional subsection per Business Logic block (only when the block does DB/cache/publish):

```markdown
- **Data access** (optional, keep to 1–3 lines):
  - Table: table/entity, operation, key columns (e.g. `trips_changes SELECT by (trip_id, start_date, start_time)`)
  - Table: table/entity (managed table store) (e.g. `fleetdata SELECT all RawVehicle entities`)
  - Cache: key pattern, operation (e.g. `get/set gtfsChanges:{date}`; `delete gtfsChanges:*`)
```

**Infer from source**: When source uses raw SQL, extract table name and WHERE columns. When it uses a managed table store (`@azure/data-tables`), extract the table name and entity type. When it uses a cache layer (e.g. CacheRepository.deleteCachedQuery), extract the key pattern or namespace from the code and document that one line in the artifacts.

### Why It Matters

Without actionable data-access cues, downstream code generators correctly identify the capabilities needed but generate TODOs instead of concrete data access code. A small, consistent convention (canonical phrasing or optional one-line hints) lets generators map to their framework's patterns without inflating the artifacts.

---

## 14. Every Endpoint Needs a Business Logic Block

### The Gap

Downstream code generators map **Business Logic Blocks** → function implementation. If an endpoint has only an API Contract (URL, response shape) but no block, there is no algorithm to implement, so the function stays TODO — even when the logic is trivial (e.g. "query entity by key/date and return from cache"). Code-analyzer may focus on "interesting" logic and skip trivial controller→service one-liners.

### What to Capture

**Every controller endpoint must have a corresponding Business Logic block**, even when the behaviour is as simple as retrieving data from a database or cache. In each block use **canonical data-access phrasing** per §13 so downstream generators can produce concrete code. When auditing, cross-check API Contracts against blocks: every documented endpoint should have a block.

**Relationship to §13:** §13 defines *how* to phrase data access. §14 ensures a *block exists* for every endpoint. Both are needed for downstream code generators to produce complete implementations.

### Why It Matters

Without a block, the code generator has nothing to implement from. Ensuring every endpoint has a block (with §13-style phrasing) is sufficient for any downstream generator to produce a concrete implementation.

---

## Quick Reference: Missed Details Checklist

Before finalizing artifacts, verify you've captured:

- [ ] Sequential vs parallel execution (Promise.all, Promise.race)
- [ ] Error handling flow (propagate, catch-and-recover, retry with backoff)
- [ ] State mutation timing (when in algorithm, conditional or unconditional)
- [ ] All branches in conditionals (if/else, switch, ternary, early returns)
- [ ] Timing purpose and relationships (delays between operations)
- [ ] Configuration source (hardcoded, env var, config file, required/optional)
- [ ] Full nested type shapes (arrays, unions, optional fields)
- [ ] External service interfaces (connection, auth, data formats)
- [ ] Loop iteration counts (deterministic vs runtime-dependent)
- [ ] Collection operation details (map, filter, reduce transformations)
- [ ] Active subset vs full dataset (runtime filters on lookup tables)
- [ ] Serialization field name mappings (wire-format names from decorators/annotations/config)
- [ ] Custom converter/deserializer behavior documented
- [ ] Data access: table/entity + operation for SQL and managed table stores (Table access); key pattern + operation for cache (Cache access) — use canonical phrasing or optional 1-line hint per block; do not embed full SQL/key lists
- [ ] Managed data stores (Azure Table Storage, Cosmos DB) classified as managed table store, not API
- [ ] Every controller endpoint has a Business Logic block (even when logic is trivial, e.g. single DB/cache read) so downstream generators can produce complete implementations
