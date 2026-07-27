//! The mock source adapter component: the SDK's export macro over the
//! canonical `mock::Adapter` operations implementor — the exact
//! anatomy of a production source adapter in
//! `augentic/emery-adapters` (one axis world plus the embedded
//! references served over MCP).
#![cfg(target_arch = "wasm32")]

adapter::source!(mock::Adapter);
