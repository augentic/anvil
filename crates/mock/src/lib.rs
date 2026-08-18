//! Emery's canonical mock adapter crate.
//!
//! One SDK-native mock kernel serving every workflow suite and the examples
//! guest; the crate speaks the SDK seam DTOs end to end.

pub mod behaviour;
pub mod ops;

#[cfg(not(target_arch = "wasm32"))]
pub mod invoke;
#[cfg(not(target_arch = "wasm32"))]
pub mod registry;
#[cfg(not(target_arch = "wasm32"))]
pub mod session;

pub use ops::{Adapter, Code, DOCS, Docs, FailExtract};
#[cfg(not(target_arch = "wasm32"))]
pub use registry::catalog;
#[cfg(not(target_arch = "wasm32"))]
pub use session::{Cwd, Session};
