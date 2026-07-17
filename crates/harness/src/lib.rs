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
pub mod convert;
pub mod env;
pub mod invoke;
pub mod provider;

cfg_if::cfg_if! {
    if #[cfg(feature = "cursor")] {
        pub mod command;
        pub mod entry;
        mod fs;
        pub mod grade;
        pub mod mcp;
        mod model;
        mod native;
        mod sandbox;
        pub mod scenario;
        mod telemetry;
        pub mod trial;
    }
}
