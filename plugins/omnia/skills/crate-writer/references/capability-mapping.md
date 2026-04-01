# Specify Artifact Capability to Omnia Provider Trait Mapping

This document defines how to translate the platform-agnostic **Source Capabilities Summary** (from design.md), **External Services** (from design.md), and concrete cues in **Business Logic** into Omnia SDK provider trait bounds for generated crates.

## Mapping Table

| Artifact Capability | Service Types | Omnia Trait | Import |
| --- | --- | --- | --- |
| Configuration | (any env var or config value in artifacts) | `Config` | `use omnia_sdk::Config;` |
| Outbound HTTP | type: API | `HttpRequest` | `use omnia_sdk::HttpRequest;` |
| Message publishing | type: message broker | `Publish` | `use omnia_sdk::{Publish, Message};` |
| Key-value state | type: cache | `StateStore` | `use omnia_sdk::StateStore;` |
| Authentication/Identity | type: identity provider | `Identity` | `use omnia_sdk::Identity;` |
| SQL database access | type: database (SQL) | `TableStore` | `use omnia_sdk::TableStore;` |
| Real-time messaging | type: WebSocket | `Broadcast` | `use omnia_sdk::Broadcast;` |
| Blob/file storage | type: blob store | `Blobstore` | `use omnia_sdk::Blobstore;` |
| Document/table storage | type: document store, type: managed table store | `DocumentStore` | `use omnia_sdk::DocumentStore;` |

## Hard Rules

1. **Config is always included.** Virtually all handlers need at least one config value. If the artifacts have any environment variable or configuration reference, include `Config`.

2. **Managed table stores map to `DocumentStore`, never `HttpRequest` or `TableStore`.** When design.md External Services lists a service with type `managed table store` (Azure Table Storage, Cosmos DB table API), the Omnia trait is `DocumentStore`. The Omnia runtime provides a native Azure Table Storage adapter behind the `DocumentStore` trait. `TableStore` is for SQL databases only.

3. **Databases always map to `TableStore`, never `StateStore`.** When design.md External Services lists type `database` (PostgreSQL, MySQL, SQL Server), the Omnia trait is `TableStore`.

4. **`HttpRequest` is for external APIs only.** Do not use `HttpRequest` for managed data stores. If design.md documents an outbound HTTP call to a managed data store endpoint (e.g., `*.table.core.windows.net`), override and use the correct trait (`DocumentStore` for Azure Table Storage, `TableStore` for SQL databases, `Blobstore` for blob stores).

5. **Cache-aside requires both `StateStore` AND the data source trait.** When the artifacts describe a caching pattern where data is loaded from a data store on cache miss, the handler needs both `StateStore` (for the cache) and the appropriate data source trait (`DocumentStore` for Azure Table Storage and document databases, `TableStore` for SQL databases, `HttpRequest` for external APIs).

6. **Document databases map to `DocumentStore`, not `TableStore`.** When design.md External Services lists a service with type `document store` (Cosmos DB document API, MongoDB), the Omnia trait is `DocumentStore`. Use `TableStore` for tabular/row data and SQL queries; use `DocumentStore` for JSON document storage with key-based access and document queries.

7. **Blob storage maps to `Blobstore`, never `HttpRequest`.** When design.md External Services lists a service with type `blob store` (Azure Blob Storage, AWS S3, file storage), the Omnia trait is `Blobstore`. The Omnia runtime provides native adapters for blob storage behind this trait.

## Deriving Traits from Specify Artifacts

### Step 1: Read Source Capabilities Summary

The design.md **Source Capabilities Summary** section contains a checklist of generic capabilities. Map each checked capability using the table above.

### Step 2: Cross-reference External Services

For each entry in design.md **External Services**, verify the trait mapping:
- Check the service `type` field against the mapping table
- If a service has type `managed table store`, ensure `DocumentStore` is included (never `TableStore` or `HttpRequest`)
- If a service has type `cache` AND another service provides the underlying data, include both `StateStore` and the data source trait

### Step 3: Read Algorithm Steps

Scan design.md **Business Logic** for data access phrasing:
- `Table access: SELECT/INSERT/UPDATE/DELETE ...` → `TableStore`
- `Document: get/insert/put/delete/query ...` → `DocumentStore`
- `Blob: get_data/write_data/delete_object ...` → `Blobstore`
- `Cache: get/set/delete ...` → `StateStore`
- `Cache: get ... on miss query database/table` → `StateStore` + data source trait (`DocumentStore` or `TableStore`)

### Step 4: Apply to Handler Bounds

The handler's generic bounds are the **union** of all traits needed by the handler function and any internal functions it calls. Fewer bounds = more testable.

## Capability Details

For detailed usage patterns, API signatures, and examples for each Omnia trait, see [capabilities.md](capabilities.md).
