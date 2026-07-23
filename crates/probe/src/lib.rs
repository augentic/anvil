//! Lab-only live-model probe over the native host.
//!
//! The library owns the multi-step live workflow trial, single-operation
//! adapter prompt scenarios, deterministic grading, model-request
//! telemetry, sandbox seeding and cleanup, and the trial CLI parsing.
//! The core constructs no concrete adapter catalog, no Cursor backend,
//! and no Tokio runtime: the process-facing entry ([`run`]) receives a
//! workspace root, a validated [`native::Catalog`], and a
//! [`ModelFactory`] from its composition root (the `eval` example at
//! `examples/eval/` here and in `augentic/specify-adapters`), and
//! drives workflow phases through the native command API.
//!
//! `feature = "client"` adds [`client`] — the shared cursor-backed
//! composition (the lazily connected [`client::DevModel`] and the argv
//! dispatch) both composition examples delegate to, leaving each root
//! only the runtime, args collection, and catalog declaration.

#[cfg(feature = "client")]
pub mod client;
mod fs;
pub mod grade;
mod run;
pub mod sandbox;
pub mod scenario;
pub mod telemetry;

pub use run::{ModelFactory, ModelInstance, run};
