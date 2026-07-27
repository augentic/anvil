//! Lab-only live-model probe over the native host.
//!
//! The library owns the typed eval [`case`] runner (workflow and
//! build cases over real `emery` verbs), deterministic grading,
//! model-request telemetry, retained-sandbox lifecycle, and the eval
//! CLI parsing. The core constructs no concrete adapter catalog, no
//! Cursor backend, and no Tokio runtime: the process-facing entry
//! ([`run`]) receives a workspace root, a validated
//! [`native::Catalog`], a [`ModelFactory`], and the composition
//! root's `cases/` and `sandbox/` directories (the `eval` example at
//! `examples/eval/` here and in `augentic/emery-adapters`), and
//! drives every case command through the native command API.
//!
//! `feature = "client"` adds [`client`] — the shared cursor-backed
//! composition (the lazily connected [`client::DevModel`], the
//! process tracing subscriber (console plus an optional `EVAL_LOG`
//! file copy), and the argv dispatch) both composition examples
//! delegate to,
//! leaving each root only the runtime, args collection, and catalog +
//! cases + sandbox declaration.

pub mod case;
#[cfg(feature = "client")]
pub mod client;
mod fs;
pub mod grade;
mod run;
pub mod sandbox;
pub mod telemetry;

pub use case::ModelFactory;
pub use run::run;
