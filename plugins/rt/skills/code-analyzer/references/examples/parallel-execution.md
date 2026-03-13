# Example: Parallel Execution

## Scenario

Service with concurrent operations, authentication, error recovery, and message publishing.

## Input Parameters

```bash
code-analyzer ./services/event-processor $CHANGE_DIR
```

- **TypeScript Source**: `./services/event-processor`
- **Specify Output**: `$CHANGE_DIR` (specs/ + design.md)

## TypeScript Source Code

```typescript
// services/event-processor/index.ts
import { getAzureToken } from '@azure/identity';
import { Publish } from './publisher';

interface EventInput {
    id: string;
    type: string;
    data: object;
}

interface EnrichedEvent {
    id: string;
    type: string;
    data: object;
    enrichedData: object;
    metadata: object;
}

const config = {
    tenantId: process.env.AZURE_TENANT_ID!
};

export class EventProcessor {
    constructor(private publisher: Publish) {}
    
    async process(input: EventInput): Promise<void> {
        try {
            const token = await getAzureToken(config.tenantId);
            
            // Parallel operations
            const [enriched, metadata] = await Promise.all([
                this.enrichData(input, token),
                this.fetchMetadata(input.id)
            ]);
            
            await this.publisher.send('events-topic', enriched);
            await this.delay(5000);
            await this.publisher.send('audit-topic', { 
                timestamp: Date.now(), 
                metadata 
            });
        } catch (error) {
            if (error.code === 'AUTH_FAILED') {
                throw new Error('Authentication failed');
            }
            // Log and continue for other errors
            console.error('Processing error:', error);
        }
    }
    
    private async enrichData(input: EventInput, token: string): Promise<EnrichedEvent> {
        // Implementation details...
    }
    
    private async fetchMetadata(id: string): Promise<object> {
        // Implementation details...
    }
    
    private async delay(ms: number): Promise<void> {
        return new Promise(resolve => setTimeout(resolve, ms));
    }
}
```

## Generated Specify Artifacts (Excerpt)

```markdown
### Component

- Name: event-processor
- Source paths: services/event-processor/index.ts, services/event-processor/publisher.ts
- Purpose summary: Processes events by enriching them with external data and publishing to message topics with audit trail.

### Constants & Configuration

- `AZURE_TENANT_ID` — source: environment variable; value: runtime; semantics: Azure AD tenant ID for authentication; required: yes

### Business Logic Blocks

### Block: EventProcessor.process

- **Return type**: Promise<void>
- **Execution mode**: asynchronous-parallel (steps 2-3 run concurrently)

- **Input Types**:
  ```
  input: EventInput
    {
      "id": "string",
      "type": "string",
      "data": "object"
    }
  ```

- **Output Types**:
  ```
  void (no return value)
  ```

- **Algorithm**:
  ```
  1. [infrastructure] Acquire Azure AD token for tenant (from AZURE_TENANT_ID)
  2. [domain] Enrich input data using external service with auth token (parallel with step 3)
  3. [infrastructure] Fetch metadata for input ID (parallel with step 2)
  4. [mechanical] Wait for steps 2-3 to complete
  5. [infrastructure] Publish enriched event to 'events-topic'
  6. [mechanical] Delay 5000 milliseconds
  7. [infrastructure] Publish audit record to 'audit-topic' with timestamp and metadata
  ```

- **Error Handling**:
  - AUTH_FAILED (error.code === 'AUTH_FAILED'): Catch, re-throw as Error with message "Authentication failed"; halt execution
  - All other errors: Catch, log to console with message "Processing error: <error>"; continue execution (no propagation, no halt)

- **State Mutations**:
  - [infrastructure] Publish message to 'events-topic' with enriched event data (step 5)
  - [infrastructure] Publish message to 'audit-topic' with audit record (step 7)

- **Preconditions**:
  - [infrastructure] AZURE_TENANT_ID environment variable must be set
  - [infrastructure] Publish instance must be initialized

- **Postconditions**:
  - [infrastructure] If successful: Two messages published (events-topic, audit-topic)
  - [infrastructure] If AUTH_FAILED: No messages published, error propagated
  - [infrastructure] If other error: Messages may be partially published (events-topic yes, audit-topic unknown)

- **Edge cases & failure modes**:
  - [infrastructure] Azure AD token acquisition fails: AUTH_FAILED error raised
  - [infrastructure] enrichData or fetchMetadata fails: Caught, logged, no propagation
  - [infrastructure] First publish succeeds but delay or second publish fails: Events published but audit may be missing

- **Complexity / cost notes**:
  - Two parallel HTTP calls (enrichData, fetchMetadata) - reduced latency
  - 5 second delay adds fixed latency to total execution time
  - Two message publishes (sequential)

- **Errors raised**:
  - Error with message "Authentication failed": When Azure AD auth fails

- **Unknowns**:
  - enrichData implementation details — not present in source (private method)
  - fetchMetadata implementation details — not present in source (private method)
  - Publish.send implementation — referenced symbol defined outside repo

### External Service Dependencies

- **Azure AD** — type: identity provider
  - Operations: Token acquisition for tenant
  - Connection details: Tenant ID from `AZURE_TENANT_ID` env var
  - Data formats: JWT access tokens
  - Authentication: Client credentials (assumed, not visible in source)

- **Message Broker** — type: message broker
  - Operations: Publish to topics 'events-topic', 'audit-topic'
  - Connection details: unknown — Publish constructor details not visible
  - Data formats: JSON serialized objects
  - Authentication: unknown — not visible in source

### Publication & Timing Patterns

- **Publication operations**:
  - Topic: `events-topic`
    - Message count: 1
    - Timing: Immediately after enrichment (step 5)
    - Payload: EnrichedEvent object
  - Topic: `audit-topic`
    - Message count: 1
    - Timing: 5000ms after events-topic publish (step 7)
    - Payload: `{ timestamp: number, metadata: object }`

- **Timing/delay operations**:
  - Delay 5000ms between event publish and audit publish
  - Purpose: unknown — not documented in source

- **Concurrency**:
  - Steps 2-3 (enrichData, fetchMetadata) execute in parallel via Promise.all
  - Step 4 waits for both to complete before continuing
  - Steps 5-7 execute sequentially
```

## Key Observations

These artifacts demonstrate:

1. **Parallel execution** - Promise.all clearly documented with execution mode
2. **Error handling flow** - Different behaviors for AUTH_FAILED vs other errors
3. **State mutations with timing** - Two publishes with 5s delay between them
4. **Partial failure scenarios** - Edge case where first publish succeeds but second fails
5. **Unknown private methods** - enrichData/fetchMetadata documented as unknown
6. **External service dependencies** - Azure AD and message broker documented

## What Makes This Complex

- **Concurrency**: Steps 2-3 run in parallel, affecting total execution time
- **Conditional error handling**: Some errors halt, others are logged and ignored
- **Timing dependency**: 5s delay creates temporal coupling between publishes
- **Partial mutations**: State changes may be incomplete if errors occur mid-flow

## Code Generation Implications

A code generator needs to:

1. Implement parallel operations (Promise.all → async/await in target language)
2. Handle selective error catching (catch specific error codes)
3. Implement delays (setTimeout → sleep/delay in target language)
4. Manage partial state mutations (consider rollback or compensation)
