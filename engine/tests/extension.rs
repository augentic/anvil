//! Integration tests for the `specify extension` surface (WASI extension
//! dispatch and extension-schema validation). Adapter-specific acceptance
//! tests (contract, vectis) live with their crates in
//! `augentic/specify-adapters`. Shared helpers live in [`common`].

mod common;

#[path = "extension/run.rs"]
mod run;

#[path = "extension/schema.rs"]
mod schema;
