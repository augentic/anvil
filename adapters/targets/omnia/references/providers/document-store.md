# DocumentStore

**When Required**: Component stores or queries JSON documents (Cosmos DB, MongoDB, PoloDB, Azure Table Storage, and other managed object or no-SQL stores).

For the trait definition and method signatures, see [capabilities.md](../capabilities.md). For provider composition rules, see [README.md](README.md).

---

## Production Patterns

The `DocumentStore` trait provides JSON document storage via `DocumentStore::get`, `DocumentStore::insert`, `DocumentStore::put`, `DocumentStore::delete`, and `DocumentStore::query`. Production Provider structs use empty implementations that delegate to the Omnia SDK defaults:

```rust
impl DocumentStore for Provider {}
```

Usage in domain functions:

```rust
use omnia_sdk::document_store::{Document, QueryOptions, Filter};

// Store a document (upsert)
let payload = serde_json::to_vec(&customer)?;
let doc = Document { id: customer_id.clone(), data: payload };
DocumentStore::put(provider, "customers", &doc).await?;

// Retrieve by id
let doc = DocumentStore::get(provider, "customers", &customer_id)
    .await?
    .ok_or_else(|| bad_request!("customer not found: {customer_id}"))?;
let customer: Customer = serde_json::from_slice(&doc.data)
    .context("deserializing customer")?;

// Query with filter and pagination
let options = QueryOptions {
    filter: Some(Filter::eq("status", "active")),
    limit: Some(50),
    ..Default::default()
};
let result = DocumentStore::query(provider, "customers", options).await?;

// Delete
let removed = DocumentStore::delete(provider, "customers", &customer_id).await?;
```

---

## MockProvider Implementation

### Complete Implementation

```rust
use omnia_sdk::DocumentStore;
use omnia_sdk::document_store::{Document, QueryOptions, QueryResult};
use std::collections::HashMap;
use std::sync::Mutex;
use once_cell::sync::OnceCell;

static DOCS: OnceCell<Mutex<HashMap<String, HashMap<String, Document>>>> = OnceCell::new();

fn docs() -> &'static Mutex<HashMap<String, HashMap<String, Document>>> {
    DOCS.get_or_init(|| Mutex::new(HashMap::new()))
}

#[derive(Clone)]
pub struct MockProvider;

impl MockProvider {
    pub fn new() -> Self {
        docs();
        Self
    }
}

impl DocumentStore for MockProvider {
    async fn get(
        &self, store: &str, id: &str,
    ) -> anyhow::Result<Option<Document>> {
        let collections = docs()
            .lock()
            .map_err(|e| anyhow::anyhow!("doc lock poisoned: {e}"))?;
        Ok(collections
            .get(store)
            .and_then(|c| c.get(id))
            .cloned())
    }

    async fn insert(
        &self, store: &str, doc: &Document,
    ) -> anyhow::Result<()> {
        let mut collections = docs()
            .lock()
            .map_err(|e| anyhow::anyhow!("doc lock poisoned: {e}"))?;
        let collection = collections.entry(store.to_string()).or_default();
        if collection.contains_key(&doc.id) {
            anyhow::bail!("document already exists: {}", doc.id);
        }
        collection.insert(doc.id.clone(), doc.clone());
        Ok(())
    }

    async fn put(
        &self, store: &str, doc: &Document,
    ) -> anyhow::Result<()> {
        docs()
            .lock()
            .map_err(|e| anyhow::anyhow!("doc lock poisoned: {e}"))?
            .entry(store.to_string())
            .or_default()
            .insert(doc.id.clone(), doc.clone());
        Ok(())
    }

    async fn delete(
        &self, store: &str, id: &str,
    ) -> anyhow::Result<bool> {
        let mut collections = docs()
            .lock()
            .map_err(|e| anyhow::anyhow!("doc lock poisoned: {e}"))?;
        Ok(collections
            .get_mut(store)
            .map_or(false, |c| c.remove(id).is_some()))
    }

    async fn query(
        &self, store: &str, options: QueryOptions,
    ) -> anyhow::Result<QueryResult> {
        let collections = docs()
            .lock()
            .map_err(|e| anyhow::anyhow!("doc lock poisoned: {e}"))?;
        let documents: Vec<Document> = collections
            .get(store)
            .map(|c| {
                let mut docs: Vec<Document> = c.values().cloned().collect();
                if let Some(limit) = options.limit {
                    docs.truncate(limit as usize);
                }
                docs
            })
            .unwrap_or_default();
        Ok(QueryResult {
            documents,
            continuation: None,
        })
    }
}
```

### With Verification Helpers

```rust
impl MockProvider {
    pub fn doc_exists(&self, store: &str, id: &str) -> bool {
        docs()
            .lock()
            .unwrap()
            .get(store)
            .map_or(false, |c| c.contains_key(id))
    }

    pub fn doc_get<T: serde::de::DeserializeOwned>(
        &self, store: &str, id: &str,
    ) -> Option<T> {
        docs()
            .lock()
            .unwrap()
            .get(store)
            .and_then(|c| c.get(id))
            .and_then(|doc| serde_json::from_slice(&doc.data).ok())
    }

    pub fn doc_count(&self, store: &str) -> usize {
        docs()
            .lock()
            .unwrap()
            .get(store)
            .map_or(0, |c| c.len())
    }

    pub fn doc_clear(&self) {
        docs().lock().unwrap().clear();
    }
}

// Usage in tests:
assert!(provider.doc_exists("customers", "cust-123"));
let customer: Customer = provider.doc_get("customers", "cust-123").unwrap();
assert_eq!(customer.name, "Acme Corp");
assert_eq!(provider.doc_count("customers"), 1);
```

### Best Practices

- Use OnceCell for global document state (nested HashMap: store -> id -> Document)
- Handle lock poisoning errors
- `insert` must fail if the id already exists (enforce in mock)
- `put` always succeeds (upsert semantics)
- `delete` returns whether a document was removed
- `query` in mocks can ignore filter/sort and return all documents (or a subset via limit)
- Provide test helpers for document verification
- Don't use static mut (unsafe)

## References

- [capabilities.md](../capabilities.md) -- DocumentStore trait definition
- [README.md](README.md) -- Provider composition rules
