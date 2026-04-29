# External dependencies and publication patterns (Steps 4–5)

Step 4 captures every external service or system dependency by type. Step 5 captures publication and timing patterns with exact counts and metadata.

## Step 4 — External service dependencies

For each external service or system dependency, document thoroughly:

- Service name and type — use one of: `database`, `managed table store`, `message broker`, `cache`, `identity provider`, `API`, `WebSocket`
- Technology (e.g., PostgreSQL, Azure Table Storage, Redis, Kafka, Azure AD)
- Connection details visible in source
- Operations performed (read, write, publish, subscribe, query, token acquisition)
- Data formats (if different from internal types)
- Authentication method

### Service type classification

- **database**: SQL databases accessed via ORM, raw SQL, or repository patterns (PostgreSQL, MySQL, SQL Server, etc.)
- **managed table store**: Cloud-managed NoSQL/table storage services accessed via SDK or REST API (Azure Table Storage via `@azure/data-tables`/`TableClient`, Azure Cosmos DB, DynamoDB, etc.). Do NOT classify these as `API` — they are managed data stores, not external HTTP APIs.
- **cache**: Key-value stores used for caching or ephemeral state (Redis, Memcached, in-memory cache libraries)
- **message broker**: Message queues and event streaming (Kafka, RabbitMQ, Azure Service Bus, SQS)
- **identity provider**: Authentication/token services (Azure AD, OAuth providers, Auth0)
- **API**: External HTTP/REST/GraphQL APIs
- **WebSocket**: WebSocket connections for real-time messaging

## Step 5 — Publication & timing patterns

Document exactly:

- **Publication count**: The exact number of times each event is published (e.g., "2 times", NOT "twice with delays" which is ambiguous). Count by reading the loop bounds in the source code (e.g., `for _ in 0..2` means 2 publications).
- **Delay placement**: Whether the delay occurs BEFORE or AFTER each publication round. Document the exact loop structure: "sleep 5s then publish all events, repeated 2 times" is different from "publish, then sleep 5s, then publish again".
- **Payload identity**: Whether the published payload is IDENTICAL across rounds or modified between rounds (e.g., timestamps incremented). Most patterns publish identical payloads -- document explicitly if the source modifies the payload between iterations.
- Timing/delay operations with exact durations
- Retry patterns with counts and backoff
- Batch vs individual publication
- **Concurrent operations** (parallel vs sequential)
- **Message metadata**: For each published message, document all metadata beyond the payload:
  - Partition/routing key (e.g., `message.key = externalId`)
  - Custom headers (e.g., `message.headers["key"] = value`)
  - Topic construction pattern (e.g., `${env}-${TOPIC_CONSTANT}` vs full topic from config)

### Publication pattern example

```markdown
- **Publication pattern**: Publish all events 2 times with 5-second delay before each round
- **Loop structure**: `for round in 0..2 { sleep(5s); for each event { publish(event) } }`
- **Payload modification**: None -- identical event published each round
- **Purpose**: Signal departure from station for schedule adherence
```
