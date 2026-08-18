//! The Emery native host: a Wasm-free deployment of the engine over
//! statically compiled adapters. `Provider: Clone` serves router
//! invocation, not concurrency — use independent providers for that.

pub mod catalog;
mod convert;
mod error;
mod model;
mod provider;

#[cfg(feature = "cli")]
pub mod command;

pub use catalog::{Builder, Catalog, Entry};
pub use error::Error;
pub use model::DynModel;
// Execution paths are deployment configuration shared with the
// engine core; re-exported so composition roots need no direct
// `project` import.
pub use project::handler::{CachePlacement, ExecutionPaths, Locations};
pub use provider::Provider;
