//! Lab-only live-model probe over the native host.
//!
//! The core constructs no concrete adapter catalog, Cursor backend, or
//! Tokio runtime; `feature = "client"` adds the shared cursor composition.

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
