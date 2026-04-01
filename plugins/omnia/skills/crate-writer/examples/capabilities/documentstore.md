# DocumentStore Handler Patterns

This document covers the `DocumentStore` trait pattern used for JSON document storage operations in Omnia business logic crates. `DocumentStore` is backed by `omnia_wasi_jsondb` and covers Azure Table Storage, Cosmos DB (document API), MongoDB, and PoloDB.

**Demonstrates:** `DocumentStore` and `Config` capability traits

## Overview

The `DocumentStore` trait provides CRUD and query operations for JSON documents addressed by key. Documents are stored as `Document { id, data }` where `data` is a JSON-encoded `Vec<u8>`. Use `DocumentStore` when the natural data shape is a JSON document rather than a tabular row.

## Trait Definition

```rust
use omnia_sdk::document_store::{Document, QueryOptions, QueryResult};

pub trait DocumentStore: Send + Sync {
    fn get(&self, store: &str, id: &str)
        -> impl Future<Output = Result<Option<Document>>> + Send;
    fn insert(&self, store: &str, doc: &Document)
        -> impl Future<Output = Result<()>> + Send;
    fn put(&self, store: &str, doc: &Document)
        -> impl Future<Output = Result<()>> + Send;
    fn delete(&self, store: &str, id: &str)
        -> impl Future<Output = Result<bool>> + Send;
    fn query(&self, store: &str, options: QueryOptions)
        -> impl Future<Output = Result<QueryResult>> + Send;
}
```

For guest code, an empty `impl DocumentStore for Provider {}` is sufficient to use the default implementations that connect to WASI JSON DB resources.

## Key Types

```rust
/// Stored document: identifier plus JSON body bytes.
pub struct Document {
    pub id: String,
    pub data: Vec<u8>,
}

/// Options for querying documents.
pub struct QueryOptions {
    pub filter: Option<Filter>,
    pub order_by: Vec<SortField>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
    pub continuation: Option<String>,
}

/// Result of a query with optional next-page token.
pub struct QueryResult {
    pub documents: Vec<Document>,
    pub continuation: Option<String>,
}
```

## CRUD Patterns

### Get by ID

```rust
use omnia_sdk::document_store::Document;

let doc = DocumentStore::get(provider, &collection, &id)
    .await?
    .ok_or_else(|| bad_request!("document not found: {id}"))?;
let item: MyItem = serde_json::from_slice(&doc.data)
    .context("deserializing document")?;
```

### Insert (fail if exists)

```rust
let payload = serde_json::to_vec(&item).context("serializing item")?;
let doc = Document { id: item.id.clone(), data: payload };
DocumentStore::insert(provider, &collection, &doc).await?;
```

### Put (upsert)

```rust
let payload = serde_json::to_vec(&item).context("serializing item")?;
let doc = Document { id: item.id.clone(), data: payload };
DocumentStore::put(provider, &collection, &doc).await?;
```

### Delete

```rust
let removed = DocumentStore::delete(provider, &collection, &id).await?;
if !removed {
    return Err(bad_request!("document not found: {id}"));
}
```

### Query with Filter and Pagination

```rust
use omnia_sdk::document_store::{Filter, QueryOptions};

let options = QueryOptions {
    filter: Some(Filter::eq("status", "active")),
    order_by: vec![],
    limit: Some(50),
    offset: None,
    continuation: None,
};
let result = DocumentStore::query(provider, &collection, options).await?;

for doc in &result.documents {
    let item: MyItem = serde_json::from_slice(&doc.data)?;
    // process item...
}

// Continue pagination if there are more results
if let Some(token) = result.continuation {
    let next_page = QueryOptions {
        continuation: Some(token),
        ..Default::default()
    };
    let more = DocumentStore::query(provider, &collection, next_page).await?;
}
```

## Complete Handler Examples

### Get Document Handler

```rust
use anyhow::Context as _;
use omnia_sdk::{bad_request, Config, Context, DocumentStore, Error, Handler, Reply, Result};
use omnia_sdk::document_store::Document;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Customer {
    pub id: String,
    pub name: String,
    pub email: String,
    pub active: bool,
}

#[derive(Clone, Debug, Deserialize)]
pub struct GetCustomerRequest {
    pub id: String,
}

async fn get_customer<P: Config + DocumentStore>(
    _owner: &str, provider: &P, req: GetCustomerRequest,
) -> Result<Customer> {
    let collection = Config::get(provider, "CUSTOMER_COLLECTION")
        .await
        .context("getting CUSTOMER_COLLECTION")?;

    let doc = DocumentStore::get(provider, &collection, &req.id)
        .await
        .context("fetching customer document")?
        .ok_or_else(|| bad_request!("customer_not_found", "Customer not found: {}", req.id))?;

    serde_json::from_slice(&doc.data)
        .context("deserializing customer")
        .map_err(Into::into)
}

impl<P: Config + DocumentStore> Handler<P> for GetCustomerRequest {
    type Error = Error;
    type Input = String;
    type Output = Customer;

    async fn handle(self, ctx: Context<'_, P>) -> Result<Reply<Customer>> {
        Ok(get_customer(ctx.owner, ctx.provider, self).await?.into())
    }

    fn from_input(input: Self::Input) -> Result<Self> {
        Ok(Self { id: input })
    }
}
```

### Upsert Document Handler

```rust
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct UpsertCustomerRequest {
    pub id: String,
    pub name: String,
    pub email: String,
    pub active: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct UpsertCustomerResponse {
    pub id: String,
}

async fn upsert_customer<P: Config + DocumentStore>(
    _owner: &str, provider: &P, req: UpsertCustomerRequest,
) -> Result<UpsertCustomerResponse> {
    if req.name.trim().is_empty() {
        return Err(bad_request!("name cannot be empty"));
    }

    let collection = Config::get(provider, "CUSTOMER_COLLECTION")
        .await
        .context("getting CUSTOMER_COLLECTION")?;

    let customer = Customer {
        id: req.id.clone(),
        name: req.name,
        email: req.email,
        active: req.active,
    };

    let payload = serde_json::to_vec(&customer).context("serializing customer")?;
    let doc = Document { id: req.id.clone(), data: payload };
    DocumentStore::put(provider, &collection, &doc).await?;

    Ok(UpsertCustomerResponse { id: req.id })
}

impl<P: Config + DocumentStore> Handler<P> for UpsertCustomerRequest {
    type Error = Error;
    type Input = Vec<u8>;
    type Output = UpsertCustomerResponse;

    async fn handle(self, ctx: Context<'_, P>) -> Result<Reply<UpsertCustomerResponse>> {
        Ok(upsert_customer(ctx.owner, ctx.provider, self).await?.into())
    }

    fn from_input(input: Self::Input) -> Result<Self> {
        serde_json::from_slice(&input)
            .context("deserializing UpsertCustomerRequest")
            .map_err(Into::into)
    }
}
```

## Azure Table Storage Examples

Azure Table Storage entities are stored as JSON documents. Model `PartitionKey` and `RowKey` as regular fields in your domain type.

### Entity Definition (Azure Table Storage)

```rust
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RawVehicle {
    pub partition_key: String,
    pub row_key: String,
    pub vehicle_label: Option<String>,
    pub vehicle_type: Option<String>,
    pub seating_capacity: Option<String>,
    pub standing_capacity: Option<String>,
    pub tag: Option<String>,
}
```

### Fetch All Entities (Azure Table Storage)

```rust
use anyhow::Context as _;
use omnia_sdk::{bad_gateway, Config, DocumentStore, Result};
use omnia_sdk::document_store::QueryOptions;

async fn fetch_all_vehicles<P>(provider: &P) -> Result<Vec<RawVehicle>>
where
    P: Config + DocumentStore,
{
    let store = Config::get(provider, "FLEET_DOCUMENT_STORE")
        .await
        .context("getting FLEET_DOCUMENT_STORE config")?;

    let result = DocumentStore::query(provider, &store, QueryOptions::default())
        .await
        .map_err(|e| bad_gateway!("failed to fetch fleet data: {e}"))?;

    let vehicles: Vec<RawVehicle> = result
        .documents
        .into_iter()
        .map(|doc| serde_json::from_slice(&doc.data).context("deserializing vehicle"))
        .collect::<anyhow::Result<Vec<_>>>()?;

    Ok(vehicles)
}
```

### Fetch with Filter (Azure Table Storage)

```rust
use omnia_sdk::document_store::Filter;

async fn fetch_vehicles_by_type<P>(provider: &P, vehicle_type: &str) -> Result<Vec<RawVehicle>>
where
    P: Config + DocumentStore,
{
    let store = Config::get(provider, "FLEET_DOCUMENT_STORE")
        .await
        .context("getting FLEET_DOCUMENT_STORE config")?;

    let options = QueryOptions {
        filter: Some(Filter::eq("vehicleType", vehicle_type)),
        ..Default::default()
    };

    let result = DocumentStore::query(provider, &store, options)
        .await
        .map_err(|e| bad_gateway!("failed to fetch vehicles by type: {e}"))?;

    let vehicles: Vec<RawVehicle> = result
        .documents
        .into_iter()
        .map(|doc| serde_json::from_slice(&doc.data).context("deserializing vehicle"))
        .collect::<anyhow::Result<Vec<_>>>()?;

    Ok(vehicles)
}
```

### Cache-Aside with Azure Table Storage

When the legacy component loads data from Azure Table Storage on startup into an in-memory cache, the WASM translation is on-demand cache-aside: `StateStore` for caching + `DocumentStore` as the data source. See [statestore.md](./statestore.md#cache-aside-with-documentstore-on-demand-loading) for the complete cache-aside pattern.

## Required Imports

```rust
// DocumentStore trait
use omnia_sdk::DocumentStore;

// Document types
use omnia_sdk::document_store::{Document, Filter, QueryOptions, QueryResult, SortField};

// SDK types
use omnia_sdk::{bad_request, Config, Context, Error, Handler, Reply, Result};

// Other common imports
use anyhow::Context as _;
use serde::{Deserialize, Serialize};
```

## Key Rules

1. **Target Architecture**: DocumentStore handlers are designed for `wasm32-wasip2` only
2. **Serialize to bytes**: Document `data` is `Vec<u8>` — serialize domain types with `serde_json::to_vec` before storing
3. **Config for collection name**: Get collection/store name from `Config` trait
4. **Validation first**: Validate input parameters before performing document operations
5. **insert vs put**: Use `insert` when the document must not already exist (fail on duplicate); use `put` for upsert semantics
6. **Error mapping**: Map document errors to `omnia_sdk::Error` with context
7. **Pagination**: Use `continuation` tokens for large result sets; respect `limit`

## Choosing Between Storage Traits

| Data Shape | Trait | When |
|------------|-------|------|
| Tabular rows, SQL queries | `TableStore` | Relational data, SQL CRUD |
| JSON documents by key/query | `DocumentStore` | Azure Table Storage, Cosmos DB documents, MongoDB, flexible schema |
| Binary blobs by key | `Blobstore` | Files, images, large payloads |
| Small key-value cache entries | `StateStore` | Redis cache, session state, TTL-based expiry |

## References

- See [../../references/capabilities.md](../../references/capabilities.md) for trait definitions
- See [../../references/providers.md](../../references/providers.md) for provider bound composition
- See [../../references/error-handling.md](../../references/error-handling.md) for error conventions
