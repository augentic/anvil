# Current State Analysis

Analyze existing Rust WASM crates to understand what the system does today.

## For each crate referenced in this change

Read the crate source and extract:

### Handler Inventory

For each handler in `src/handler.rs` or `src/handlers/*.rs`:
- Handler name (request type that implements `Handler<P>`)
- Provider trait bounds (e.g., `P: Config + HttpRequest + Identity`)
- Input type (`Vec<u8>`, `String`, `(String, String)`, `Option<String>`, `()`)
- Output type and `IntoBody` implementation
- Route path or message topic
- Business logic summary (what the `handle` function does)

### Domain Types

For each type in `src/types.rs` and domain modules:
- Type name, fields, and Rust types
- Serde attributes (`rename`, `rename_all`, `default`, `skip_serializing_if`)
- Newtype wrappers and enums
- Relationships between types

### Provider Usage

- Which provider traits are actually called (not just bounded)
- Config keys used via `Config::get`
- HTTP endpoints called via `HttpRequest::fetch`
- Topics published to via `Publish::publish`
- State keys used via `StateStore::get/set`

### Test Coverage

For each test file in `tests/`:
- Test name and which handler it covers
- MockProvider configuration (config keys, HTTP fixtures)
- Key assertions

### Dependencies

From `Cargo.toml`:
- Direct dependencies and their purpose
- Feature flags enabled

## Output Format

Write a structured summary per crate. This is not a full IR — it is a
concise inventory used to identify what exists, what is tested, and what
provider capabilities are in use