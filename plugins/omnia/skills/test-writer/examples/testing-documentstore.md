# Example 05: Document Storage with DocumentStore

Complete working example demonstrating MockProvider with DocumentStore, for components that store and query JSON documents (Cosmos DB, MongoDB, Azure Table Storage).

## Scenario

Generate test harness for a customer management component that:
- Stores customer documents
- Retrieves customers by id
- Queries active customers
- Implements Config and DocumentStore traits

## Component Structure

```
ex-documentstore/
├── src/
│   ├── lib.rs
│   ├── handlers.rs
│   └── types.rs
├── tests/
│   ├── provider.rs      # MockProvider with DocumentStore
│   ├── customers.rs     # Test cases
│   └── data/
│       └── customers.json
└── Cargo.toml
```

## Handler Code (Reference)

### handlers.rs

```rust
use anyhow::Context;
use omnia_sdk::{bad_gateway, bad_request, Config, DocumentStore, Result};
use omnia_sdk::document_store::{Document, QueryOptions, Filter};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Customer {
    pub id: String,
    pub name: String,
    pub status: String,
    pub email: Option<String>,
}

pub async fn save_customer<P>(provider: &P, customer: Customer) -> Result<()>
where
    P: Config + DocumentStore,
{
    let store = Config::get(provider, "CUSTOMERS_STORE").await?;
    let data = serde_json::to_vec(&customer).context("serializing customer")?;
    let doc = Document { id: customer.id.clone(), data };
    DocumentStore::put(provider, &store, &doc)
        .await
        .map_err(|e| bad_gateway!("failed to save customer: {e}"))?;
    Ok(())
}

pub async fn get_customer<P>(provider: &P, id: &str) -> Result<Customer>
where
    P: Config + DocumentStore,
{
    let store = Config::get(provider, "CUSTOMERS_STORE").await?;
    let doc = DocumentStore::get(provider, &store, id)
        .await
        .map_err(|e| bad_gateway!("failed to get customer: {e}"))?
        .ok_or_else(|| bad_request!("customer not found: {id}"))?;
    let customer: Customer = serde_json::from_slice(&doc.data)
        .context("deserializing customer")?;
    Ok(customer)
}

pub async fn list_active_customers<P>(provider: &P) -> Result<Vec<Customer>>
where
    P: Config + DocumentStore,
{
    let store = Config::get(provider, "CUSTOMERS_STORE").await?;
    let options = QueryOptions {
        filter: Some(Filter::eq("status", "active")),
        limit: Some(100),
        ..Default::default()
    };
    let result = DocumentStore::query(provider, &store, options)
        .await
        .map_err(|e| bad_gateway!("failed to query customers: {e}"))?;
    let customers: Vec<Customer> = result
        .documents
        .into_iter()
        .map(|doc| serde_json::from_slice(&doc.data).context("deserializing customer"))
        .collect::<anyhow::Result<Vec<_>>>()?;
    Ok(customers)
}
```

## Generated Test Harness

### tests/provider.rs

```rust
use omnia_sdk::{Config, DocumentStore};
use omnia_sdk::document_store::{Document, QueryOptions, QueryResult};
use std::collections::HashMap;
use std::sync::Mutex;
use once_cell::sync::OnceCell;

static CONFIG: OnceCell<HashMap<String, String>> = OnceCell::new();
static DOCS: OnceCell<Mutex<HashMap<String, HashMap<String, Document>>>> = OnceCell::new();

fn config() -> &'static HashMap<String, String> {
    CONFIG.get_or_init(|| {
        HashMap::from([
            ("CUSTOMERS_STORE".to_string(), "customers".to_string()),
        ])
    })
}

fn docs() -> &'static Mutex<HashMap<String, HashMap<String, Document>>> {
    DOCS.get_or_init(|| Mutex::new(HashMap::new()))
}

#[derive(Clone)]
pub struct MockProvider;

impl MockProvider {
    pub fn new() -> Self {
        config();
        docs();
        Self
    }

    pub fn doc_exists(&self, store: &str, id: &str) -> bool {
        docs()
            .lock()
            .unwrap()
            .get(store)
            .map_or(false, |c| c.contains_key(id))
    }

    pub fn doc_get<T: serde::de::DeserializeOwned>(&self, store: &str, id: &str) -> Option<T> {
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

    pub fn seed_doc<T: serde::Serialize>(&self, store: &str, id: &str, value: &T) {
        let data = serde_json::to_vec(value).unwrap();
        let doc = Document { id: id.to_string(), data };
        docs()
            .lock()
            .unwrap()
            .entry(store.to_string())
            .or_default()
            .insert(id.to_string(), doc);
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

impl DocumentStore for MockProvider {
    async fn get(
        &self, store: &str, id: &str,
    ) -> anyhow::Result<Option<Document>> {
        let collections = docs()
            .lock()
            .map_err(|e| anyhow::anyhow!("doc lock poisoned: {e}"))?;
        Ok(collections.get(store).and_then(|c| c.get(id)).cloned())
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
        let mut collections = docs().lock().unwrap();
        Ok(collections
            .get_mut(store)
            .map_or(false, |c| c.remove(id).is_some()))
    }

    async fn query(
        &self, store: &str, options: QueryOptions,
    ) -> anyhow::Result<QueryResult> {
        let collections = docs().lock().unwrap();
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

### tests/customers.rs

```rust
mod provider;

use provider::MockProvider;

#[tokio::test]
async fn save_and_retrieve_customer() {
    let provider = MockProvider::new();
    provider.doc_clear();

    let customer = ex_documentstore::Customer {
        id: "cust-001".to_string(),
        name: "Acme Corp".to_string(),
        status: "active".to_string(),
        email: Some("info@acme.com".to_string()),
    };

    ex_documentstore::save_customer(&provider, customer.clone()).await.unwrap();

    assert!(provider.doc_exists("customers", "cust-001"));
    assert_eq!(provider.doc_count("customers"), 1);

    let retrieved = ex_documentstore::get_customer(&provider, "cust-001").await.unwrap();
    assert_eq!(retrieved.name, "Acme Corp");
    assert_eq!(retrieved.email, Some("info@acme.com".to_string()));
}

#[tokio::test]
async fn get_missing_customer_returns_error() {
    let provider = MockProvider::new();
    provider.doc_clear();

    let result = ex_documentstore::get_customer(&provider, "nonexistent").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn save_overwrites_existing_customer() {
    let provider = MockProvider::new();
    provider.doc_clear();

    let customer_v1 = ex_documentstore::Customer {
        id: "cust-001".to_string(),
        name: "Acme Corp".to_string(),
        status: "active".to_string(),
        email: None,
    };
    ex_documentstore::save_customer(&provider, customer_v1).await.unwrap();

    let customer_v2 = ex_documentstore::Customer {
        id: "cust-001".to_string(),
        name: "Acme Corp (Updated)".to_string(),
        status: "active".to_string(),
        email: Some("new@acme.com".to_string()),
    };
    ex_documentstore::save_customer(&provider, customer_v2).await.unwrap();

    assert_eq!(provider.doc_count("customers"), 1);
    let retrieved: ex_documentstore::Customer =
        provider.doc_get("customers", "cust-001").unwrap();
    assert_eq!(retrieved.name, "Acme Corp (Updated)");
}

#[tokio::test]
async fn list_active_customers_returns_seeded_data() {
    let provider = MockProvider::new();
    provider.doc_clear();

    let active = ex_documentstore::Customer {
        id: "cust-active".to_string(),
        name: "Active Inc".to_string(),
        status: "active".to_string(),
        email: None,
    };
    let inactive = ex_documentstore::Customer {
        id: "cust-inactive".to_string(),
        name: "Inactive LLC".to_string(),
        status: "inactive".to_string(),
        email: None,
    };

    provider.seed_doc("customers", &active.id, &active);
    provider.seed_doc("customers", &inactive.id, &inactive);

    let result = ex_documentstore::list_active_customers(&provider).await.unwrap();
    // MockProvider query ignores filters -- assert total count
    assert_eq!(result.len(), 2);
}
```

## Key Patterns

1. **Nested HashMap** -- `HashMap<String, HashMap<String, Document>>` models store -> id -> Document
2. **OnceCell + Mutex** -- thread-safe global state for document storage
3. **Lock poisoning** -- handle via `map_err` in trait implementations
4. **Insert uniqueness** -- `insert` must fail if the id already exists; `put` always upserts
5. **Verification helpers** -- `doc_exists`, `doc_get<T>`, `doc_count`, `doc_clear` for test assertions
6. **Seeding helper** -- `seed_doc` pre-populates documents for read-path tests
7. **Query simplification** -- mocks can ignore filters and return all documents (note in test comments)
