//! Live composed-WASM driver for the canonical workflow scenarios.
//!
//! The shipped `specify` binary is the deployment unit under test: it
//! hosts the freshly built workflow guest and the sibling checkout's
//! release-built adapter components, binds the live cursor backend,
//! and serves the `/mcp/<name>` routes the spawned agents fetch. This
//! crate owns only what surrounds that binary — sandbox staging, the
//! project-root deployment manifest, and per-step capture — so the
//! quality orchestrator and the ignored live test share one driver.

pub mod driver;
pub mod manifest;
