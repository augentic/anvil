//! The mock target adapter component: the SDK's export macro over the
//! canonical `mock::Adapter` operations implementor — the exact
//! anatomy of a production target adapter in
//! `augentic/specify-adapters` (one axis world plus the embedded
//! references served over MCP).
#![cfg(target_arch = "wasm32")]

adapter::target!(mock::Adapter);
