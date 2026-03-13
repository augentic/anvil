# Example: Branching and Caching

## Scenario

Service with multiple conditional branches, cache-or-fetch pattern, and early returns.

## Input Parameters

```bash
code-analyzer ./src/data-service.ts $CHANGE_DIR
```

- **TypeScript Source**: `./src/data-service.ts`
- **Specify Output**: `$CHANGE_DIR` (specs/ + design.md)

## TypeScript Source Code

```typescript
// src/data-service.ts
import { cache } from './cache';
import { fetchFromAPI } from './api-client';

interface InputData {
    id: string;
    forceRefresh?: boolean;
}

interface Result {
    success: boolean;
    data?: object;
    error?: string;
    source?: 'cache' | 'api';
}

export async function validateAndProcess(data: InputData): Promise<Result> {
    // Validation: check ID exists
    if (!data.id || data.id.length === 0) {
        return { success: false, error: 'Invalid ID' };
    }
    
    // Validation: check ID format
    if (!/^[A-Z0-9]{8}$/.test(data.id)) {
        return { success: false, error: 'ID must be 8 alphanumeric characters' };
    }
    
    // Force refresh bypasses cache
    if (data.forceRefresh) {
        const fresh = await fetchFromAPI(data.id);
        await cache.set(`data:${data.id}`, fresh, 3600);
        return { success: true, data: fresh, source: 'api' };
    }
    
    // Try cache first
    const cached = await cache.get(`data:${data.id}`);
    if (cached) {
        return { success: true, data: cached, source: 'cache' };
    }
    
    // Cache miss: fetch from API
    try {
        const fresh = await fetchFromAPI(data.id);
        await cache.set(`data:${data.id}`, fresh, 3600);
        return { success: true, data: fresh, source: 'api' };
    } catch (error) {
        return { success: false, error: `API fetch failed: ${error.message}` };
    }
}
```

## Generated Specify Artifacts

```markdown
### Component

- Name: data-service
- Source paths: src/data-service.ts, src/cache.ts, src/api-client.ts
- Purpose summary: Validates input IDs and retrieves data from cache or external API with cache-aside pattern.

### Constants & Configuration

- `CACHE_KEY_PREFIX` — source: hardcoded; value: "data:"; semantics: Prefix for cache keys
- `CACHE_TTL` — source: hardcoded; value: 3600; semantics: Cache time-to-live in seconds
- `ID_REGEX` — source: hardcoded; value: "^[A-Z0-9]{8}$"; semantics: Validation pattern for ID format

### Business Logic Blocks

### Block: validateAndProcess

- **Return type**: Promise<Result>
- **Execution mode**: asynchronous (sequential operations with conditional branching)

- **Input Types**:
  ```
  data: InputData
    {
      "id": "string",
      "forceRefresh?": "boolean"
    }
  ```

- **Output Types**:
  ```
  Result
    {
      "success": "boolean",
      "data?": "object",
      "error?": "string",
      "source?": "'cache' | 'api'"
    }
  ```

- **Algorithm**:
  ```
  1. [domain] Validate data.id field is present and non-empty
     a. If ID missing or empty:
        i.  [domain] Return failure result: { success: false, error: 'Invalid ID' }
        ii. Halt execution (early return)
     b. Else:
        i. Continue to step 2
  
  2. [domain] Validate data.id matches format: 8 alphanumeric uppercase characters
     a. If ID format invalid:
        i.  [domain] Return failure result: { success: false, error: 'ID must be 8 alphanumeric characters' }
        ii. Halt execution (early return)
     b. Else:
        i. Continue to step 3
  
  3. [mechanical] Check forceRefresh flag
     a. If forceRefresh is true:
        i.  [infrastructure] Fetch fresh data from API for ID (skip cache)
        ii. [infrastructure] Write data to cache with key `data:{id}`, TTL 3600s
        iii.[domain] Return success result: { success: true, data: fresh, source: 'api' }
        iv. Halt execution (early return)
     b. Else:
        i. Continue to step 4
  
  4. [infrastructure] Attempt to retrieve data from cache with key `data:{id}`
     a. If cache hit (cached data exists):
        i.  [domain] Return success result: { success: true, data: cached, source: 'cache' }
        ii. Halt execution (early return)
     b. Else (cache miss):
        i. Continue to step 5
  
  5. [infrastructure] Fetch fresh data from API for ID (within try block)
     a. On success:
        i.  [infrastructure] Write data to cache with key `data:{id}`, TTL 3600s
        ii. [domain] Return success result: { success: true, data: fresh, source: 'api' }
     b. On error:
        i. [domain] Return failure result: { success: false, error: 'API fetch failed: <message>' }
  ```

- **Error Handling**:
  - API fetch error (step 5): Catch and return failure result with error message; no propagation
  - Cache get/set errors: Propagate (not caught in source); halt execution

- **State Mutations**:
  - [infrastructure] Write to cache with key `data:{id}`, value: fresh API data, TTL: 3600s
    - Occurs in step 3a-ii (if forceRefresh=true)
    - Occurs in step 5a-i (if cache miss and API fetch succeeds)

- **Preconditions**:
  - [infrastructure] Cache client must be initialized
  - [infrastructure] API client must be available

- **Postconditions**:
  - [domain] Returns Result object with success flag
  - [domain] On success: data field is populated, source indicates origin
  - [domain] On failure: error field contains descriptive message
  - [infrastructure] On successful API fetch: cache is updated

- **Edge cases & failure modes**:
  - [domain] Empty ID string: Returns validation error (step 1)
  - [domain] Invalid ID format: Returns validation error (step 2)
  - [infrastructure] Cache unavailable: Cache operations propagate errors
  - [infrastructure] API unavailable: Returns failure result (step 5b)
  - [infrastructure] forceRefresh=true with API failure: Returns error, cache not updated

- **Complexity / cost notes**:
  - Best case (cache hit): 2 validation checks + 1 cache read
  - Worst case (cache miss): 2 validation checks + 1 cache read + 1 API call + 1 cache write
  - Force refresh case: 2 validation checks + 1 API call + 1 cache write (no cache read)

- **Errors raised**:
  - (none - all errors caught and converted to Result)

- **Unknowns**:
  - fetchFromAPI implementation — referenced symbol defined outside repo
  - cache.get/set implementation — referenced symbol defined outside repo
  - API response shape — not present in source (typed as object)

### External Service Dependencies

- **Cache** — type: cache
  - Operations: GET, SET with TTL
  - Connection details: unknown — not visible in source
  - Data formats: Serialized objects (format unknown)
  - Authentication: unknown — not visible in source

- **External API** — type: API
  - Operations: Fetch data by ID
  - Connection details: unknown — not visible in source
  - Data formats: Returns object (shape unknown)
  - Authentication: unknown — not visible in source

### Publication & Timing Patterns

- (none)

### Output Event Structures

- (none - function returns directly)

### Dependencies

- `cache` module — Cache client for get/set operations
- `api-client` module — API client with fetchFromAPI function

### Notes

- Implements cache-aside pattern (read-through cache with lazy loading)
- forceRefresh flag allows bypassing stale cache
- All API errors caught and converted to Result type (no propagation)
- Multiple early returns based on validation and cache status
- Cache write happens in two places: force refresh path and cache miss path
```

## Key Observations

These artifacts demonstrate:

1. **Multiple conditional branches** - 4 early return paths documented with hierarchy
2. **Cache-aside pattern** - Read from cache first, fetch on miss, update cache
3. **Force refresh logic** - Bypass cache when flag is set
4. **Validation sequence** - Two validation steps before business logic
5. **Selective error handling** - API errors caught, cache errors propagated
6. **Dual cache writes** - State mutation happens in 2 different branches

## Control Flow Paths

This function has **5 possible execution paths**:

1. **Invalid ID (empty)** → Step 1a → Return error
2. **Invalid ID (format)** → Steps 1b, 2a → Return error
3. **Force refresh** → Steps 1b, 2b, 3a → Fetch API → Update cache → Return success
4. **Cache hit** → Steps 1b, 2b, 3b, 4a → Return success from cache
5. **Cache miss** → Steps 1b, 2b, 3b, 4b, 5 → Fetch API → Update cache → Return success

The artifacts document all 5 paths with explicit branching.

## Code Generation Implications

A code generator needs to:

1. Implement multiple early returns (return in middle of function)
2. Handle optional fields (forceRefresh?)
3. Implement regex validation
4. Support cache-aside pattern
5. Convert caught errors to result objects (no exceptions in happy path)
6. Handle state mutations in multiple branches
