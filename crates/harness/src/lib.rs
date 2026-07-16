//! Reusable native eval-harness core: adapter-agnostic machinery for
//! running the Specify workflow in-process, without a wasm runtime.
//!
//! Consumers declare which adapters are linked through the typed
//! [`catalog::Catalog`] builder over the per-axis operations traits
//! (`adapter::Source` / `adapter::Target`), exposed to the shared
//! entrypoints as one [`catalog::Binding`] hook; everything else — the
//! seam [`provider::Provider`], the [`native::Native`] model bridge,
//! the [`model::DevModel`] live backend, [`telemetry`], the [`mcp`]
//! reference shelves, the [`command`] / [`http`] transports, and the
//! [`trial`] / [`scenario`] drivers over their wrapper-supplied
//! [`trial::Profile`] — is generic over that catalog. The two wrappers
//! (the engine's eval crate over the testkit fixture, the adapters
//! repository's `engine` over the first-party adapters) stay
//! declarative bindings.

pub mod catalog;
pub mod command;
pub mod env;
pub mod fs;
pub mod http;
pub mod inputs;
pub mod mcp;
pub mod model;
pub mod native;
pub mod provider;
pub mod sandbox;
pub mod scenario;
pub mod telemetry;
pub mod trial;
