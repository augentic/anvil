# Example 04: Blob Storage with Blobstore

Complete working example demonstrating MockProvider with Blobstore, for components that store or retrieve binary objects.

## Scenario

Generate test harness for a report storage component that:
- Uploads generated reports as blobs
- Retrieves blobs by name
- Lists available reports
- Implements Config and Blobstore traits

## Component Structure

```
ex-blobstore/
├── src/
│   ├── lib.rs
│   ├── handlers.rs
│   └── types.rs
├── tests/
│   ├── provider.rs      # MockProvider with Blobstore
│   ├── reports.rs       # Test cases
│   └── data/
│       └── sample-report.json
└── Cargo.toml
```

## Handler Code (Reference)

### handlers.rs

```rust
use omnia_sdk::{bad_gateway, bad_request, Blobstore, Config, Result};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize)]
pub struct UploadReportRequest {
    pub name: String,
    pub data: Vec<u8>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadReportResponse {
    pub name: String,
    pub size_bytes: usize,
}

pub async fn upload_report<P>(provider: &P, request: UploadReportRequest) -> Result<UploadReportResponse>
where
    P: Config + Blobstore,
{
    let container = Config::get(provider, "REPORTS_CONTAINER").await?;
    Blobstore::write_data(provider, &container, &request.name, &request.data)
        .await
        .map_err(|e| bad_gateway!("failed to upload report: {e}"))?;
    Ok(UploadReportResponse {
        name: request.name,
        size_bytes: request.data.len(),
    })
}

pub async fn download_report<P>(provider: &P, name: &str) -> Result<Vec<u8>>
where
    P: Config + Blobstore,
{
    let container = Config::get(provider, "REPORTS_CONTAINER").await?;
    Blobstore::get_data(provider, &container, name, 0, 0)
        .await
        .map_err(|e| bad_gateway!("failed to download report: {e}"))?
        .ok_or_else(|| bad_request!("report not found: {name}"))
}

pub async fn list_reports<P>(provider: &P) -> Result<Vec<String>>
where
    P: Config + Blobstore,
{
    let container = Config::get(provider, "REPORTS_CONTAINER").await?;
    let keys = Blobstore::list_objects(provider, &container)
        .await
        .map_err(|e| bad_gateway!("failed to list reports: {e}"))?;
    Ok(keys)
}
```

## Generated Test Harness

### tests/provider.rs

```rust
use omnia_sdk::{Blobstore, Config};
use std::collections::HashMap;
use std::sync::Mutex;
use once_cell::sync::OnceCell;

static CONFIG: OnceCell<HashMap<String, String>> = OnceCell::new();
static BLOBS: OnceCell<Mutex<HashMap<String, HashMap<String, Vec<u8>>>>> = OnceCell::new();

fn config() -> &'static HashMap<String, String> {
    CONFIG.get_or_init(|| {
        HashMap::from([
            ("REPORTS_CONTAINER".to_string(), "reports".to_string()),
        ])
    })
}

fn blobs() -> &'static Mutex<HashMap<String, HashMap<String, Vec<u8>>>> {
    BLOBS.get_or_init(|| Mutex::new(HashMap::new()))
}

#[derive(Clone)]
pub struct MockProvider;

impl MockProvider {
    pub fn new() -> Self {
        config();
        blobs();
        Self
    }

    pub fn blob_exists(&self, container: &str, name: &str) -> bool {
        blobs()
            .lock()
            .unwrap()
            .get(container)
            .map_or(false, |c| c.contains_key(name))
    }

    pub fn blob_get(&self, container: &str, name: &str) -> Option<Vec<u8>> {
        blobs()
            .lock()
            .unwrap()
            .get(container)
            .and_then(|c| c.get(name).cloned())
    }

    pub fn blob_clear(&self) {
        blobs().lock().unwrap().clear();
    }
}

impl Config for MockProvider {
    async fn get(&self, key: &str) -> anyhow::Result<String> {
        config()
            .get(key)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("unknown config key: {key}"))
    }
}

impl Blobstore for MockProvider {
    async fn get_data(
        &self, container: &str, name: &str, _start: u64, _end: u64,
    ) -> anyhow::Result<Option<Vec<u8>>> {
        let store = blobs()
            .lock()
            .map_err(|e| anyhow::anyhow!("blob lock poisoned: {e}"))?;
        Ok(store.get(container).and_then(|c| c.get(name)).cloned())
    }

    async fn write_data(
        &self, container: &str, name: &str, data: &[u8],
    ) -> anyhow::Result<()> {
        blobs()
            .lock()
            .map_err(|e| anyhow::anyhow!("blob lock poisoned: {e}"))?
            .entry(container.to_string())
            .or_default()
            .insert(name.to_string(), data.to_vec());
        Ok(())
    }

    async fn delete_object(
        &self, container: &str, name: &str,
    ) -> anyhow::Result<()> {
        let mut store = blobs()
            .lock()
            .map_err(|e| anyhow::anyhow!("blob lock poisoned: {e}"))?;
        if let Some(c) = store.get_mut(container) {
            c.remove(name);
        }
        Ok(())
    }

    async fn has_object(
        &self, container: &str, name: &str,
    ) -> anyhow::Result<bool> {
        let store = blobs()
            .lock()
            .map_err(|e| anyhow::anyhow!("blob lock poisoned: {e}"))?;
        Ok(store.get(container).map_or(false, |c| c.contains_key(name)))
    }

    async fn list_objects(
        &self, container: &str,
    ) -> anyhow::Result<Vec<String>> {
        let store = blobs()
            .lock()
            .map_err(|e| anyhow::anyhow!("blob lock poisoned: {e}"))?;
        Ok(store
            .get(container)
            .map(|c| c.keys().cloned().collect())
            .unwrap_or_default())
    }
}
```

### tests/reports.rs

```rust
mod provider;

use provider::MockProvider;

#[tokio::test]
async fn upload_and_download_round_trip() {
    let provider = MockProvider::new();
    provider.blob_clear();

    let data = b"report-content-2024-q1".to_vec();
    let request = ex_blobstore::UploadReportRequest {
        name: "2024-q1.json".to_string(),
        data: data.clone(),
    };

    let result = ex_blobstore::upload_report(&provider, request).await.unwrap();
    assert_eq!(result.name, "2024-q1.json");
    assert_eq!(result.size_bytes, data.len());

    assert!(provider.blob_exists("reports", "2024-q1.json"));

    let downloaded = ex_blobstore::download_report(&provider, "2024-q1.json").await.unwrap();
    assert_eq!(downloaded, data);
}

#[tokio::test]
async fn download_missing_blob_returns_error() {
    let provider = MockProvider::new();
    provider.blob_clear();

    let result = ex_blobstore::download_report(&provider, "nonexistent.json").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn list_reports_returns_uploaded_names() {
    let provider = MockProvider::new();
    provider.blob_clear();

    let names = vec!["a.json", "b.json", "c.json"];
    for name in &names {
        let request = ex_blobstore::UploadReportRequest {
            name: name.to_string(),
            data: b"data".to_vec(),
        };
        ex_blobstore::upload_report(&provider, request).await.unwrap();
    }

    let mut listed = ex_blobstore::list_reports(&provider).await.unwrap();
    listed.sort();
    assert_eq!(listed, names);
}
```

## Key Patterns

1. **Nested HashMap** -- `HashMap<String, HashMap<String, Vec<u8>>>` models container -> name -> data
2. **OnceCell + Mutex** -- thread-safe global state for blob storage
3. **Lock poisoning** -- handle via `map_err` in trait implementations
4. **Range parameters** -- `start`/`end` can be ignored in mocks (return full data)
5. **Verification helpers** -- `blob_exists`, `blob_get`, `blob_clear` for test assertions
