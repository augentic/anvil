# Blobstore

**When Required**: Component stores or retrieves binary blobs (files, images, reports, large payloads).

For the trait definition and method signatures, see [capabilities.md](../capabilities.md). For provider composition rules, see [README.md](README.md).

---

## Production Patterns

The `Blobstore` trait provides binary object storage via `Blobstore::get_data`, `Blobstore::write_data`, `Blobstore::delete_object`, `Blobstore::has_object`, and `Blobstore::list_objects`. Production Provider structs use empty implementations that delegate to the Omnia SDK defaults:

```rust
impl Blobstore for Provider {}
```

Usage in domain functions:

```rust
// Write a blob
let data = serde_json::to_vec(&report)?;
Blobstore::write_data(provider, "reports", &key, &data).await?;

// Read a blob (0, 0 = full read)
let bytes = Blobstore::get_data(provider, "reports", &key, 0, 0)
    .await?
    .ok_or_else(|| bad_request!("blob not found: {key}"))?;

// Check existence before delete
if Blobstore::has_object(provider, "reports", &key).await? {
    Blobstore::delete_object(provider, "reports", &key).await?;
}

// List all objects in a container
let keys = Blobstore::list_objects(provider, "reports").await?;
```

---

## MockProvider Implementation

### Complete Implementation

```rust
use omnia_sdk::Blobstore;
use std::collections::HashMap;
use std::sync::Mutex;
use once_cell::sync::OnceCell;

static BLOBS: OnceCell<Mutex<HashMap<String, HashMap<String, Vec<u8>>>>> = OnceCell::new();

fn blobs() -> &'static Mutex<HashMap<String, HashMap<String, Vec<u8>>>> {
    BLOBS.get_or_init(|| Mutex::new(HashMap::new()))
}

#[derive(Clone)]
pub struct MockProvider;

impl MockProvider {
    pub fn new() -> Self {
        blobs();
        Self
    }
}

impl Blobstore for MockProvider {
    async fn get_data(
        &self, container: &str, name: &str, _start: u64, _end: u64,
    ) -> anyhow::Result<Option<Vec<u8>>> {
        let store = blobs()
            .lock()
            .map_err(|e| anyhow::anyhow!("blob lock poisoned: {e}"))?;
        Ok(store
            .get(container)
            .and_then(|c| c.get(name))
            .cloned())
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
        Ok(store
            .get(container)
            .map_or(false, |c| c.contains_key(name)))
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

### With Verification Helpers

```rust
impl MockProvider {
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

// Usage in tests:
assert!(provider.blob_exists("reports", "2024-q1.json"));
let data = provider.blob_get("reports", "2024-q1.json").unwrap();
```

### Best Practices

- Use OnceCell for global blob state (nested HashMap: container -> name -> data)
- Handle lock poisoning errors
- Range parameters (`start`, `end`) can be ignored in mocks (return full data)
- Provide test helpers for blob verification
- Don't use static mut (unsafe)

## References

- [capabilities.md](../capabilities.md) -- Blobstore trait definition
- [README.md](README.md) -- Provider composition rules
