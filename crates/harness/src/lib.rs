//! Native Specify harness: an adapter-agnostic provider for model-free
//! workflow tests, plus the optional live eval runtime.
//!
//! Consumers declare which adapters are linked through the typed
//! [`catalog::Catalog`] builder over the per-axis operations traits
//! (`adapter::Source` / `adapter::Target`), exposed to the shared
//! entrypoints as one [`catalog::Binding`] hook; everything else — the
//! seam [`provider::Provider`], typed operation invocation
//! ([`invoke`]), and live runtime — is generic over that catalog. Eval
//! binaries enable `runtime` and provide only their adapter binding;
//! trial inputs are explicit command arguments.

pub mod catalog;
#[cfg(feature = "runtime")]
pub mod command;
pub mod convert;
#[cfg(feature = "runtime")]
pub mod entry;
pub mod env;
#[cfg(feature = "runtime")]
mod fs;
#[cfg(feature = "runtime")]
pub mod grade;
pub mod invoke;
#[cfg(feature = "runtime")]
pub mod mcp;
#[cfg(feature = "runtime")]
mod model;
#[cfg(feature = "runtime")]
mod native;
pub mod provider;
#[cfg(feature = "runtime")]
mod sandbox;
#[cfg(feature = "runtime")]
pub mod scenario;
#[cfg(feature = "runtime")]
mod telemetry;
#[cfg(feature = "runtime")]
pub mod trial;
