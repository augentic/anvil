# Blobstore Handler Patterns

This document covers the `Blobstore` trait pattern used for binary object storage operations in Omnia business logic crates. `Blobstore` is backed by `omnia_wasi_blobstore` and covers Azure Blob Storage, AWS S3, and other object storage services.

**Demonstrates:** `Blobstore` and `Config` capability traits

## Overview

The `Blobstore` trait provides operations for storing, retrieving, and managing binary objects (blobs) organized in containers. Use `Blobstore` for file uploads/downloads, report storage, image/media assets, or any binary content addressed by key.

## Trait Definition

```rust
pub trait Blobstore: Send + Sync {
    fn get_data(&self, container: &str, name: &str, start: u64, end: u64)
        -> impl Future<Output = Result<Option<Vec<u8>>>> + Send;
    fn write_data(&self, container: &str, name: &str, data: &[u8])
        -> impl Future<Output = Result<()>> + Send;
    fn delete_object(&self, container: &str, name: &str)
        -> impl Future<Output = Result<()>> + Send;
    fn has_object(&self, container: &str, name: &str)
        -> impl Future<Output = Result<bool>> + Send;
    fn list_objects(&self, container: &str)
        -> impl Future<Output = Result<Vec<String>>> + Send;
}
```

For guest code, an empty `impl Blobstore for Provider {}` is sufficient to use the default implementations that connect to WASI Blobstore resources.

## CRUD Patterns

### Write a Blob

```rust
let data = serde_json::to_vec(&report).context("serializing report")?;
Blobstore::write_data(provider, &container, &key, &data).await?;
```

### Read a Blob (full)

```rust
let bytes = Blobstore::get_data(provider, &container, &key, 0, 0)
    .await?
    .ok_or_else(|| bad_request!("blob not found: {key}"))?;
let report: Report = serde_json::from_slice(&bytes)
    .context("deserializing report")?;
```

### Check Existence

```rust
if Blobstore::has_object(provider, &container, &key).await? {
    tracing::info!("blob already exists: {key}");
}
```

### Delete a Blob

```rust
Blobstore::delete_object(provider, &container, &key).await?;
```

### List Objects in a Container

```rust
let keys = Blobstore::list_objects(provider, &container).await?;
tracing::info!("container has {} objects", keys.len());
```

## Complete Handler Examples

### Upload Blob Handler

```rust
use anyhow::Context as _;
use omnia_sdk::{bad_request, Blobstore, Config, Context, Error, Handler, Reply, Result};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize)]
pub struct UploadRequest {
    pub name: String,
    pub content_type: String,
    pub data: Vec<u8>,
}

#[derive(Clone, Debug, Serialize)]
pub struct UploadResponse {
    pub key: String,
}

async fn upload_blob<P: Config + Blobstore>(
    _owner: &str, provider: &P, req: UploadRequest,
) -> Result<UploadResponse> {
    if req.name.trim().is_empty() {
        return Err(bad_request!("name cannot be empty"));
    }
    if req.data.is_empty() {
        return Err(bad_request!("data cannot be empty"));
    }

    let container = Config::get(provider, "BLOB_CONTAINER")
        .await
        .context("getting BLOB_CONTAINER")?;

    Blobstore::write_data(provider, &container, &req.name, &req.data)
        .await
        .context("writing blob")?;

    Ok(UploadResponse { key: req.name })
}

impl<P: Config + Blobstore> Handler<P> for UploadRequest {
    type Error = Error;
    type Input = Vec<u8>;
    type Output = UploadResponse;

    async fn handle(self, ctx: Context<'_, P>) -> Result<Reply<UploadResponse>> {
        Ok(upload_blob(ctx.owner, ctx.provider, self).await?.into())
    }

    fn from_input(input: Self::Input) -> Result<Self> {
        serde_json::from_slice(&input)
            .context("deserializing UploadRequest")
            .map_err(Into::into)
    }
}
```

### Download Blob Handler

```rust
#[derive(Clone, Debug, Deserialize)]
pub struct DownloadRequest {
    pub name: String,
}

async fn download_blob<P: Config + Blobstore>(
    _owner: &str, provider: &P, req: DownloadRequest,
) -> Result<Vec<u8>> {
    let container = Config::get(provider, "BLOB_CONTAINER")
        .await
        .context("getting BLOB_CONTAINER")?;

    Blobstore::get_data(provider, &container, &req.name, 0, 0)
        .await
        .context("reading blob")?
        .ok_or_else(|| bad_request!("blob_not_found", "Blob not found: {}", req.name))
}

impl<P: Config + Blobstore> Handler<P> for DownloadRequest {
    type Error = Error;
    type Input = String;
    type Output = Vec<u8>;

    async fn handle(self, ctx: Context<'_, P>) -> Result<Reply<Vec<u8>>> {
        Ok(download_blob(ctx.owner, ctx.provider, self).await?.into())
    }

    fn from_input(input: Self::Input) -> Result<Self> {
        Ok(Self { name: input })
    }
}
```

### List and Cleanup Handler

```rust
#[derive(Clone, Debug, Serialize)]
pub struct ListResponse {
    pub objects: Vec<String>,
    pub count: usize,
}

async fn list_blobs<P: Config + Blobstore>(
    _owner: &str, provider: &P,
) -> Result<ListResponse> {
    let container = Config::get(provider, "BLOB_CONTAINER")
        .await
        .context("getting BLOB_CONTAINER")?;

    let objects = Blobstore::list_objects(provider, &container)
        .await
        .context("listing blobs")?;
    let count = objects.len();

    Ok(ListResponse { objects, count })
}
```

## Required Imports

```rust
// Blobstore trait
use omnia_sdk::Blobstore;

// SDK types
use omnia_sdk::{bad_request, Config, Context, Error, Handler, Reply, Result};

// Other common imports
use anyhow::Context as _;
use serde::{Deserialize, Serialize};
```

## Key Rules

1. **Target Architecture**: Blobstore handlers are designed for `wasm32-wasip2` only
2. **Range reads**: Pass `0, 0` for full reads; use `start` and `end` byte offsets for partial reads
3. **Config for container name**: Get container/bucket name from `Config` trait
4. **Validation first**: Validate input (non-empty name, non-empty data) before performing blob operations
5. **Error mapping**: Map blob errors to `omnia_sdk::Error` with context
6. **Binary data**: Blob `data` is `&[u8]` / `Vec<u8>` — no serialization format is assumed; use `serde_json::to_vec` for JSON, raw bytes for images/files

## Choosing Between Storage Traits

| Data Shape | Trait | When |
|------------|-------|------|
| Binary blobs by key | `Blobstore` | Files, images, large payloads, opaque binary data |
| JSON documents by key/query | `DocumentStore` | Cosmos DB documents, MongoDB, flexible schema |
| Tabular rows, SQL queries | `TableStore` | Relational data, Azure Table Storage, SQL CRUD |
| Small key-value cache entries | `StateStore` | Redis cache, session state, TTL-based expiry |

## References

- See [../../references/capabilities.md](../../references/capabilities.md) for trait definitions
- See [../../references/providers.md](../../references/providers.md) for provider bound composition
- See [../../references/error-handling.md](../../references/error-handling.md) for error conventions
