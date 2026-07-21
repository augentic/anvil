//! The Specify native host: a Wasm-free deployment of the workflow
//! engine over statically compiled adapters.
//!
//! Consumers declare their linked adapters once through the validated
//! [`Catalog`] builder over the per-axis operations traits
//! (`adapter::Source` / `adapter::Target`), erase their model backend
//! once into [`DynModel`], and construct a [`Provider`] implementing
//! project anchoring, ensure/resolve, model, workflow source, and
//! workflow target capabilities. The optional `cli` feature adds
//! asynchronous command execution over the shared typed transport
//! router plus native reference hosting.
//!
//! The command path is single-flight: one command runs to completion
//! per provider graph. `Provider: Clone` supports router invocation
//! and shared capabilities, not concurrent independent commands;
//! embedders needing concurrency create independent providers and
//! cache/reference contexts.

pub mod catalog;
mod convert;
mod error;
mod model;
mod provider;
pub mod references;

#[cfg(feature = "cli")]
pub mod command;

pub use catalog::{Builder, Catalog, Entry};
pub use error::Error;
pub use model::DynModel;
// Execution paths are deployment configuration shared with the
// engine core; re-exported so composition roots need no direct
// `project` import.
pub use project::handler::{CachePlacement, ExecutionPaths, Locations};
pub use provider::{Provider, ReferenceMode};
