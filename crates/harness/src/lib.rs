//! Reusable native eval-harness core: adapter-agnostic machinery for
//! running the Specify workflow in-process, without a wasm runtime.
//!
//! Consumers declare which adapters are linked through the typed
//! [`catalog::Catalog`] builder over the per-axis operations traits
//! (`adapter::Source` / `adapter::Target`), exposed to the shared
//! entrypoints as one [`catalog::Binding`] hook; everything else — the
//! seam [`provider::Provider`], typed operation invocation
//! ([`invoke`]), and the optional layers — is generic over that
//! catalog. The two wrappers (the engine's eval crate over the fixture
//! adapters, the adapters repository's `engine` over the first-party
//! adapters) stay declarative bindings.
//!
//! Features are dependency cuts. The always-on core is the catalog,
//! the provider, typed invocation, and the env guard; `model` adds the
//! live cursor backend and the [`native`] bridge, `mcp` the reference
//! shelves and `Provider::bound`, `command` / `http` the native
//! transports, `scenario` the prompt-scenario runner (plus the [`fs`]
//! and [`inputs`] helpers), and `trial` the live-model trial driver
//! with [`sandbox`], [`telemetry`], and shared [`grade`] helpers.
//! `full` enables everything.

pub mod catalog;
#[cfg(feature = "command")]
pub mod command;
pub mod convert;
pub mod env;
#[cfg(feature = "scenario")]
pub mod fs;
#[cfg(feature = "trial")]
pub mod grade;
#[cfg(feature = "http")]
pub mod http;
#[cfg(feature = "scenario")]
pub mod inputs;
pub mod invoke;
#[cfg(feature = "mcp")]
pub mod mcp;
#[cfg(feature = "model")]
pub mod model;
#[cfg(feature = "model")]
pub mod native;
pub mod provider;
#[cfg(feature = "trial")]
pub mod sandbox;
#[cfg(feature = "scenario")]
pub mod scenario;
#[cfg(feature = "trial")]
pub mod telemetry;
#[cfg(feature = "trial")]
pub mod trial;
